// QBNex BASIC extension: starts qb-lsp, registers the qb-dap debugger and
// provides Run/Compile commands.

import { spawn } from 'node:child_process';
import * as fs from 'node:fs';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions } from 'vscode-languageclient/node';
import { completeDebugConfig, expandArgs, exeExtension, serverCandidates } from './util';

let client: LanguageClient | undefined;
let output: vscode.OutputChannel;
let serverLog: vscode.LogOutputChannel;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
    output = vscode.window.createOutputChannel('QBNex');
    serverLog = vscode.window.createOutputChannel('QBNex Language Server', { log: true });
    context.subscriptions.push(output, serverLog);
    context.subscriptions.push(
        vscode.commands.registerCommand('qbnex.compile', () => compileCommand()),
        vscode.commands.registerCommand('qbnex.run', () => runCommand()),
        vscode.commands.registerCommand('qbnex.restartServer', () => restartServer(context)),
        vscode.debug.registerDebugAdapterDescriptorFactory('qbnex', {
            createDebugAdapterDescriptor: () => new vscode.DebugAdapterExecutable(toolCommand(context, 'dap.path', 'qb-dap')),
        }),
        vscode.debug.registerDebugConfigurationProvider('qbnex', {
            resolveDebugConfiguration: async (_folder, cfg) => {
                const editor = vscode.window.activeTextEditor;
                const file = editor?.document.languageId === 'qbasic' ? editor.document.uri.fsPath : undefined;
                if (editor?.document.isDirty && file) {
                    await editor.document.save();
                }
                const done = completeDebugConfig(cfg, file, config().get<string>('compilerPath', 'qb'));
                if (!done) {
                    void vscode.window.showErrorMessage('QBNex: open a .bas file to debug.');
                }
                return done as vscode.DebugConfiguration | undefined;
            },
        }),
        vscode.workspace.onDidChangeConfiguration(async (e) => {
            if (e.affectsConfiguration('qbnex.lsp')) {
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

/** A tool's command (qb-lsp, qb-dap): setting, then bundled copy, then PATH. */
function toolCommand(context: vscode.ExtensionContext, setting: string, tool: string): string {
    const configured = config().get<string>(setting, '').trim();
    for (const c of serverCandidates(configured, context.extensionPath, process.platform, tool)) {
        if (!path.isAbsolute(c) || fs.existsSync(c)) {
            return c;
        }
    }
    return tool;
}

function serverCommand(context: vscode.ExtensionContext): string {
    return toolCommand(context, 'lsp.path', 'qb-lsp');
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
    const command = serverCommand(context);
    const serverOptions: ServerOptions = { run: { command }, debug: { command } };
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
        const msg = `QBNex: could not start the language server "${command}": ${String(err)}. ` +
            'Set qbnex.lsp.path or put qb-lsp on PATH.';
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

function variables(doc: vscode.TextDocument): Record<string, string> {
    const file = doc.uri.fsPath;
    const folder = vscode.workspace.getWorkspaceFolder(doc.uri)?.uri.fsPath ?? path.dirname(file);
    return {
        file,
        fileDirname: path.dirname(file),
        fileBasenameNoExtension: path.basename(file, path.extname(file)),
        exeExtension: exeExtension(process.platform),
        workspaceFolder: folder,
    };
}

/** Runs the compiler, streaming its output; resolves with its exit code. */
function compile(doc: vscode.TextDocument): Promise<number> {
    const cmd = config().get<string>('compilerPath', 'qb');
    const args = expandArgs(config().get<string[]>('compileArgs', ['${file}']), variables(doc));
    output.show(true);
    output.appendLine(`> ${cmd} ${args.map((a) => (a.includes(' ') ? `"${a}"` : a)).join(' ')}`);
    return new Promise((resolve) => {
        const child = spawn(cmd, args, { cwd: path.dirname(doc.uri.fsPath), windowsHide: true });
        child.stdout.on('data', (d: Buffer) => output.append(d.toString()));
        child.stderr.on('data', (d: Buffer) => output.append(d.toString()));
        child.on('error', (err) => {
            output.appendLine(`QBNex: cannot start "${cmd}": ${err.message}`);
            resolve(-1);
        });
        child.on('close', (code) => {
            output.appendLine(code === 0 ? 'Compiled.' : `Compiler exited with code ${code}.`);
            resolve(code ?? -1);
        });
    });
}

async function compileCommand(): Promise<boolean> {
    const doc = await activeSource();
    if (!doc) {
        return false;
    }
    const code = await compile(doc);
    if (code !== 0) {
        void vscode.window.showErrorMessage('QBNex: compile failed (see the QBNex output).');
        return false;
    }
    return true;
}

async function runCommand(): Promise<void> {
    const doc = await activeSource();
    if (!doc) {
        return;
    }
    const v = variables(doc);
    if ((await compile(doc)) !== 0) {
        void vscode.window.showErrorMessage('QBNex: compile failed (see the QBNex output).');
        return;
    }
    const exe = path.join(v.fileDirname, v.fileBasenameNoExtension + v.exeExtension);
    if (!fs.existsSync(exe)) {
        void vscode.window.showErrorMessage(`QBNex: ${exe} was not produced; check qbnex.compileArgs.`);
        return;
    }
    // The program is the terminal's process: no shell quoting involved.
    const term = vscode.window.createTerminal({ name: `QBNex: ${v.fileBasenameNoExtension}`, cwd: v.fileDirname, shellPath: exe });
    term.show();
}
