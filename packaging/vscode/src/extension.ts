// QBNex BASIC extension: starts qb-lsp, registers the qb-dap debugger and
// provides Run/Compile commands.

import { ChildProcessWithoutNullStreams, spawn } from 'node:child_process';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions } from 'vscode-languageclient/node';
import {
    completeDebugConfig, expandArgs, expandHome, expandVariables, exeExtension, outputPath, parseDiagnostics,
    resolveCompiler, resolveTool, ToolCommand, writtenSince,
} from './util';

let client: LanguageClient | undefined;
let output: vscode.OutputChannel;
let serverLog: vscode.LogOutputChannel;
let diagnostics: vscode.DiagnosticCollection;
/** The terminal each source file's program runs in (keyed by the file's URI). */
const runTerminals = new Map<string, vscode.Terminal>();

export async function activate(context: vscode.ExtensionContext): Promise<void> {
    output = vscode.window.createOutputChannel('QBNex');
    serverLog = vscode.window.createOutputChannel('QBNex Language Server', { log: true });
    diagnostics = vscode.languages.createDiagnosticCollection('qbnex');
    context.subscriptions.push(output, serverLog, diagnostics);
    context.subscriptions.push(
        vscode.commands.registerCommand('qbnex.compile', () => compileCommand()),
        vscode.commands.registerCommand('qbnex.run', () => runCommand()),
        vscode.commands.registerCommand('qbnex.restartServer', () => restartServer(context)),
        vscode.window.onDidCloseTerminal((t) => {
            for (const [key, term] of runTerminals) {
                if (term === t) {
                    runTerminals.delete(key);
                }
            }
        }),
        vscode.debug.registerDebugAdapterDescriptorFactory('qbnex', {
            createDebugAdapterDescriptor: () => {
                const t = toolCommand(context, 'dap.path', 'qb-dap', 'dap');
                return new vscode.DebugAdapterExecutable(t.command, t.args);
            },
        }),
        vscode.debug.registerDebugConfigurationProvider('qbnex', {
            resolveDebugConfiguration: async (_folder, cfg) => {
                const editor = vscode.window.activeTextEditor;
                const doc = editor?.document.languageId === 'qbasic' ? editor.document : undefined;
                const usesActive = !cfg.program || String(cfg.program).includes('${file}');
                if (doc && usesActive) {
                    if (doc.isUntitled) {
                        void vscode.window.showErrorMessage('QBNex: save the file first.');
                        return undefined;
                    }
                    if (doc.isDirty && !(await doc.save())) {
                        return undefined;
                    }
                }
                // qb-dap starts the compiler without a shell: give it the program file, not
                // a name to look up or a .cmd front end
                let compiler: string;
                if (typeof cfg.compiler === 'string' && cfg.compiler) {
                    const r = resolveCompiler(cfg.compiler, process.platform, process.env, isFile);
                    compiler = 'path' in r ? r.path : cfg.compiler;
                } else {
                    const r = findCompiler(doc);
                    if ('error' in r) {
                        if (!cfg.qir) {
                            void vscode.window.showErrorMessage(r.error);
                            return undefined;
                        }
                        compiler = compilerSetting(doc);
                    } else {
                        compiler = r.path;
                    }
                }
                const done = completeDebugConfig({ ...cfg, compiler }, doc?.uri.fsPath, compiler);
                if (!done) {
                    void vscode.window.showErrorMessage('QBNex: open a .bas file to debug.');
                }
                return done as vscode.DebugConfiguration | undefined;
            },
        }),
        vscode.workspace.onDidChangeConfiguration(async (e) => {
            if (e.affectsConfiguration('qbnex.lsp') || e.affectsConfiguration('qbnex.compilerPath')) {
                await restartServer(context);
            }
        }),
    );
    await startServer(context);
}

export async function deactivate(): Promise<void> {
    await stopServer();
}

function config(): vscode.WorkspaceConfiguration {
    return vscode.workspace.getConfiguration('qbnex');
}

function isFile(p: string): boolean {
    try {
        return fs.statSync(p).isFile();
    } catch {
        return false;
    }
}

/** The qbnex.compilerPath setting with `~` and `${...}` variables expanded. */
function compilerSetting(doc?: vscode.TextDocument): string {
    const unknown: string[] = [];
    const raw = expandHome(config().get<string>('compilerPath', 'qb').trim() || 'qb', os.homedir());
    const cmd = expandVariables(raw, variables(doc), process.env, unknown);
    reportUnknown('qbnex.compilerPath', unknown);
    return cmd;
}

/** The compiler as a program file that can be spawned without a shell. */
function findCompiler(doc?: vscode.TextDocument): { path: string } | { error: string } {
    return resolveCompiler(compilerSetting(doc), process.platform, process.env, isFile);
}

/** A tool's command line (qb-lsp, qb-dap): see resolveTool for the order. */
function toolCommand(context: vscode.ExtensionContext, setting: string, tool: string, subcommand: string): ToolCommand {
    const doc = vscode.window.activeTextEditor?.document;
    const compilerCommand = compilerSetting(doc?.languageId === 'qbasic' ? doc : undefined);
    const found = resolveCompiler(compilerCommand, process.platform, process.env, isFile);
    let compiler: string | undefined;
    if ('path' in found) {
        // ~/.local/bin/qb links into the install folder, where the tools are
        try {
            compiler = fs.realpathSync(found.path);
        } catch {
            compiler = found.path;
        }
    }
    const r = resolveTool({
        tool,
        subcommand,
        configured: config().get<string>(setting, ''),
        extensionPath: context.extensionPath,
        compiler,
        compilerCommand,
        platform: process.platform,
        env: process.env,
        home: os.homedir(),
        exists: isFile,
    });
    if (r.missingConfigured) {
        void vscode.window.showWarningMessage(
            `QBNex: qbnex.${setting} is "${r.missingConfigured}", which does not exist; using ${[r.command.command, ...r.command.args].join(' ')} instead.`);
    }
    return r.command;
}

let restarting: Promise<void> = Promise.resolve();

/** One restart at a time: two that overlap each started a server and one was never stopped. */
function restartServer(context: vscode.ExtensionContext): Promise<void> {
    restarting = restarting
        .catch(() => undefined)
        .then(async () => {
            await stopServer();
            await startServer(context);
        });
    return restarting;
}

async function startServer(context: vscode.ExtensionContext): Promise<void> {
    if (!config().get<boolean>('lsp.enabled', true)) {
        return;
    }
    const { command, args } = toolCommand(context, 'lsp.path', 'qb-lsp', 'lsp');
    const serverOptions: ServerOptions = { run: { command, args }, debug: { command, args } };
    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'qbasic' },
            { scheme: 'untitled', language: 'qbasic' },
        ],
        outputChannel: serverLog,
    };
    client = new LanguageClient('qbnex', 'QBNex Language Server', serverOptions, clientOptions);
    try {
        await client.start();
    } catch (err) {
        client = undefined;
        const msg = `QBNex: could not start the language server "${[command, ...args].join(' ')}": ${String(err)}. ` +
            'Set qbnex.lsp.path or qbnex.compilerPath.';
        void vscode.window.showWarningMessage(msg);
    }
}

async function stopServer(): Promise<void> {
    const c = client;
    client = undefined;
    if (c) {
        await c.stop().catch(() => undefined);
    }
}

/** The active QBasic document, saved to disk. */
async function activeSource(): Promise<vscode.TextDocument | undefined> {
    const doc = vscode.window.activeTextEditor?.document;
    if (!doc || doc.languageId !== 'qbasic') {
        void vscode.window.showErrorMessage('QBNex: open a .bas file first.');
        return undefined;
    }
    if (doc.isUntitled) {
        void vscode.window.showErrorMessage('QBNex: save the file first.');
        return undefined;
    }
    if (doc.isDirty && !(await doc.save())) {
        return undefined;
    }
    return doc;
}

/** Values for `${name}` in the settings; the file ones only when there is a file. */
function variables(doc?: vscode.TextDocument): Record<string, string> {
    const v: Record<string, string> = { exeExtension: exeExtension(process.platform), userHome: os.homedir() };
    const file = doc && !doc.isUntitled ? doc.uri.fsPath : undefined;
    if (file) {
        v.file = file;
        v.fileDirname = path.dirname(file);
        v.fileBasename = path.basename(file);
        v.fileExtname = path.extname(file);
        v.fileBasenameNoExtension = path.basename(file, path.extname(file));
    }
    const folder = (doc ? vscode.workspace.getWorkspaceFolder(doc.uri) : vscode.workspace.workspaceFolders?.[0])?.uri.fsPath
        ?? (file ? path.dirname(file) : undefined);
    if (folder) {
        v.workspaceFolder = folder;
    }
    return v;
}

/** Names a setting's `${...}` variables that were left as written. */
function reportUnknown(setting: string, unknown: string[]): void {
    if (unknown.length) {
        const names = [...new Set(unknown)].map((n) => '${' + n + '}').join(', ');
        output.appendLine(`QBNex: ${setting}: unknown variable(s) left as written: ${names}`);
    }
}

interface CompileResult {
    /** The compiler's exit code; -1 when it did not start. */
    code: number;
    /** The message was already shown (no need for the generic "compile failed"). */
    reported?: boolean;
    args: string[];
    /** When the compile started (Date.now()). */
    started: number;
}

/** Runs the compiler, streaming its output; fills the Problems panel from its messages. */
function compile(doc: vscode.TextDocument): Promise<CompileResult> {
    diagnostics.clear();
    output.show(true);
    const unknown: string[] = [];
    const args = expandArgs(config().get<string[]>('compileArgs', ['${file}']), variables(doc), process.env, unknown);
    reportUnknown('qbnex.compileArgs', unknown);
    const started = Date.now();
    const found = findCompiler(doc);
    if ('error' in found) {
        output.appendLine(found.error);
        void vscode.window.showErrorMessage(found.error);
        return Promise.resolve({ code: -1, reported: true, args, started });
    }
    const cmd = found.path;
    output.appendLine(`> ${cmd} ${args.map((a) => (a.includes(' ') ? `"${a}"` : a)).join(' ')}`);
    const dir = path.dirname(doc.uri.fsPath);
    return new Promise((resolve) => {
        let text = '';
        let settled = false;
        const finish = (code: number): void => {
            if (settled) {
                return;
            }
            settled = true;
            const byFile = new Map<string, vscode.Diagnostic[]>();
            for (const d of parseDiagnostics(text, dir, process.platform)) {
                const line = Math.max(0, d.line - 1);
                const col = d.column ? d.column - 1 : 0;
                const severity = d.severity === 'error' ? vscode.DiagnosticSeverity.Error
                    : d.severity === 'warning' ? vscode.DiagnosticSeverity.Warning : vscode.DiagnosticSeverity.Information;
                const diag = new vscode.Diagnostic(new vscode.Range(line, col, line, Number.MAX_SAFE_INTEGER), d.message, severity);
                diag.source = 'qb';
                if (d.code) {
                    diag.code = d.code;
                }
                byFile.set(d.file, [...(byFile.get(d.file) ?? []), diag]);
            }
            for (const [file, list] of byFile) {
                diagnostics.set(vscode.Uri.file(file), list);
            }
            resolve({ code, args, started });
        };
        let child: ChildProcessWithoutNullStreams;
        try {
            child = spawn(cmd, args, { cwd: dir, windowsHide: true });
        } catch (err) {
            // spawn throws instead of emitting 'error' for some bad commands (EINVAL)
            output.appendLine(`QBNex: cannot start "${cmd}": ${err instanceof Error ? err.message : String(err)}`);
            finish(-1);
            return;
        }
        // decoded as a stream: a character split between two chunks stays whole
        child.stdout.setEncoding('utf8');
        child.stderr.setEncoding('utf8');
        const take = (d: string): void => {
            output.append(d);
            text += d;
        };
        child.stdout.on('data', take);
        child.stderr.on('data', take);
        child.on('error', (err) => {
            output.appendLine(`QBNex: cannot start "${cmd}": ${err.message}`);
            finish(-1);
        });
        child.on('close', (code) => {
            if (settled) {
                return;
            }
            output.appendLine(code === 0 ? 'Compiled.' : `Compiler exited with code ${code}.`);
            finish(code ?? -1);
        });
    });
}

async function compileCommand(): Promise<boolean> {
    const doc = await activeSource();
    if (!doc) {
        return false;
    }
    const r = await compile(doc);
    if (r.code !== 0) {
        if (!r.reported) {
            void vscode.window.showErrorMessage('QBNex: compile failed (see the QBNex output).');
        }
        return false;
    }
    return true;
}

/**
 * Closes the terminal of the file's last run. That ends the program, so on Windows its
 * executable can be written again; waits until VS Code reports the terminal closed.
 */
function closeRunTerminal(key: string): Promise<void> {
    const term = runTerminals.get(key);
    if (!term) {
        return Promise.resolve();
    }
    runTerminals.delete(key);
    return new Promise((resolve) => {
        const sub = vscode.window.onDidCloseTerminal((t) => {
            if (t === term) {
                done();
            }
        });
        const timer = setTimeout(done, 3000);
        function done(): void {
            clearTimeout(timer);
            sub.dispose();
            resolve();
        }
        term.dispose();
    });
}

async function runCommand(): Promise<void> {
    const doc = await activeSource();
    if (!doc) {
        return;
    }
    const key = doc.uri.toString();
    await closeRunTerminal(key);
    const r = await compile(doc);
    if (r.code !== 0) {
        if (!r.reported) {
            void vscode.window.showErrorMessage('QBNex: compile failed (see the QBNex output).');
        }
        return;
    }
    const exe = outputPath(r.args, doc.uri.fsPath, process.platform);
    let written = false;
    try {
        const st = fs.statSync(exe);
        written = st.isFile() && writtenSince(st.mtimeMs, r.started);
    } catch {
        written = false;
    }
    if (!written) {
        void vscode.window.showErrorMessage(`QBNex: ${exe} was not written by this compile; check qbnex.compileArgs.`);
        return;
    }
    // The program is the terminal's process: no shell quoting involved.
    const name = path.basename(doc.uri.fsPath, path.extname(doc.uri.fsPath));
    const term = vscode.window.createTerminal({ name: `QBNex: ${name}`, cwd: path.dirname(doc.uri.fsPath), shellPath: exe });
    runTerminals.set(key, term);
    term.show();
}
