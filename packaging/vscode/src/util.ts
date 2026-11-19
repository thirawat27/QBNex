// Helpers without a VS Code dependency (unit-tested with node --test).

import * as path from 'node:path';

/** Executable suffix for a platform. */
export function exeExtension(platform: string): string {
    return platform === 'win32' ? '.exe' : '';
}

/** Path functions of the given platform, so the helpers can be tested for every OS on any OS. */
function pathFor(platform: string): typeof path.posix {
    return platform === 'win32' ? path.win32 : path.posix;
}

export type Env = Record<string, string | undefined>;

/** An environment variable; Windows names ignore case. */
function envValue(env: Env, name: string, platform: string): string | undefined {
    if (platform !== 'win32') {
        return env[name];
    }
    const key = Object.keys(env).find((k) => k.toUpperCase() === name.toUpperCase());
    return key === undefined ? undefined : env[key];
}

/**
 * Replaces `${name}` in a string. `${env:NAME}` is the environment variable (empty when
 * unset, as VS Code does). Unknown names stay as written and are added to `unknown`.
 */
export function expandVariables(s: string, vars: Record<string, string>, env: Env = {}, unknown?: string[], platform: string = process.platform): string {
    return s.replace(/\$\{([^}]+)\}/g, (m, name: string) => {
        if (Object.prototype.hasOwnProperty.call(vars, name)) {
            return vars[name];
        }
        if (name.startsWith('env:')) {
            return envValue(env, name.slice(4), platform) ?? '';
        }
        unknown?.push(name);
        return m;
    });
}

/** Replaces `${name}` in each argument; unknown names stay as written. */
export function expandArgs(args: readonly string[], vars: Record<string, string>, env: Env = {}, unknown?: string[], platform: string = process.platform): string[] {
    return args.map((a) => expandVariables(a, vars, env, unknown, platform));
}

/** A leading `~` is the home folder (settings are often copied from a shell). */
export function expandHome(p: string, home: string): string {
    if (p === '~') {
        return home;
    }
    if (p.startsWith('~/') || p.startsWith('~\\')) {
        return home.replace(/[\\/]+$/, '') + p.slice(1);
    }
    return p;
}

/**
 * Where a command runs from, the way the shell finds it: a name is looked up in each PATH
 * folder, a path (it has a separator) is taken as written. On Windows the PATHEXT suffixes
 * are tried when the name has none of them. `exists` says whether a file is there.
 */
export function findOnPath(command: string, platform: string, env: Env, exists: (p: string) => boolean): string | undefined {
    if (!command) {
        return undefined;
    }
    const p = pathFor(platform);
    const win = platform === 'win32';
    let names = [command];
    if (win) {
        const exts = (envValue(env, 'PATHEXT', platform) ?? '.COM;.EXE;.BAT;.CMD').split(';').filter(Boolean).map((e) => e.toLowerCase());
        if (!exts.includes(p.extname(command).toLowerCase())) {
            names = exts.map((e) => command + e);
        }
    }
    const hasSeparator = win ? /[\\/]/.test(command) : command.includes('/');
    if (hasSeparator) {
        return names.find(exists);
    }
    const dirs = (envValue(env, 'PATH', platform) ?? '').split(win ? ';' : ':');
    for (const raw of dirs) {
        const dir = win ? raw.replace(/^"(.*)"$/, '$1') : raw;
        if (!dir) {
            continue;
        }
        for (const n of names) {
            const full = p.join(dir, n);
            if (exists(full)) {
                return full;
            }
        }
    }
    return undefined;
}

/**
 * The compiler to spawn. Without a shell Windows cannot start a .cmd or .bat file, so
 * the qb.cmd / qbnex.cmd front ends are replaced by the qb.exe next to them.
 */
export function resolveCompiler(command: string, platform: string, env: Env, exists: (p: string) => boolean): { path: string } | { error: string } {
    const exe = 'qb' + exeExtension(platform);
    const found = findOnPath(command, platform, env, exists);
    if (!found) {
        return { error: `QBNex: the compiler "${command}" was not found. Set qbnex.compilerPath to the full path of ${exe}.` };
    }
    if (platform === 'win32' && /\.(cmd|bat)$/i.test(found)) {
        const next = path.win32.join(path.win32.dirname(found), 'qb.exe');
        if (exists(next)) {
            return { path: next };
        }
        return { error: `QBNex: the compiler "${found}" is a batch file, which VS Code cannot start directly. Set qbnex.compilerPath to the full path of qb.exe.` };
    }
    return { path: found };
}

/** A tool's command line: the program and its arguments. */
export interface ToolCommand {
    command: string;
    args: string[];
}

export interface ToolLookup {
    /** qb-lsp or qb-dap. */
    tool: string;
    /** The compiler subcommand that starts the same tool (lsp, dap). */
    subcommand: string;
    /** The qbnex.lsp.path / qbnex.dap.path setting (may be empty). */
    configured: string;
    extensionPath: string;
    /** The compiler's real path (symbolic links followed), when it was found. */
    compiler: string | undefined;
    /** The compiler command as configured, used when it was not found. */
    compilerCommand: string;
    platform: string;
    env: Env;
    home: string;
    exists: (p: string) => boolean;
}

/**
 * Finds qb-lsp / qb-dap, in order: the setting, the copy bundled in the extension, the tool
 * on PATH, the tool in the compiler's folder (the install folder: on Linux and macOS only
 * qb and qbnex are linked onto PATH), and last the compiler itself (`qb lsp`, `qb dap`).
 * `missingConfigured` is the configured path when it does not exist.
 */
export function resolveTool(o: ToolLookup): { command: ToolCommand; missingConfigured?: string } {
    const p = pathFor(o.platform);
    const file = o.tool + exeExtension(o.platform);
    let missingConfigured: string | undefined;
    const configured = expandHome(o.configured.trim(), o.home);
    if (configured) {
        const found = findOnPath(configured, o.platform, o.env, o.exists);
        if (found) {
            return { command: { command: found, args: [] } };
        }
        missingConfigured = configured;
    }
    const candidates = [
        p.join(o.extensionPath, 'server', file),
        findOnPath(o.tool, o.platform, o.env, o.exists),
        o.compiler ? p.join(p.dirname(o.compiler), file) : undefined,
    ];
    for (const c of candidates) {
        if (c && o.exists(c)) {
            return { command: { command: c, args: [] }, missingConfigured };
        }
    }
    return { command: { command: o.compiler ?? o.compilerCommand, args: [o.subcommand] }, missingConfigured };
}

/**
 * The executable a compile writes: the `-o` / `--output` value (relative to the source
 * folder, where the compiler is started; `.exe` added on Windows when missing, as qb does),
 * else the source name next to the source.
 */
export function outputPath(args: readonly string[], source: string, platform: string): string {
    const p = pathFor(platform);
    let out: string | undefined;
    for (let i = 0; i + 1 < args.length; i++) {
        if (args[i] === '-o' || args[i] === '--output') {
            out = args[++i];
        }
    }
    const dir = p.dirname(source);
    if (out === undefined) {
        return p.join(dir, p.basename(source, p.extname(source)) + exeExtension(platform));
    }
    let file = p.normalize(p.isAbsolute(out) ? out : p.join(dir, out));
    if (platform === 'win32' && p.extname(file).toLowerCase() !== '.exe') {
        file += '.exe';
    }
    return file;
}

/**
 * Whether a file modified at `mtimeMs` was written by a compile that started at `startMs`.
 * The start is taken to its whole second: some file systems keep times in whole seconds.
 */
export function writtenSince(mtimeMs: number, startMs: number): boolean {
    return mtimeMs >= Math.floor(startMs / 1000) * 1000;
}

/** A compiler message with its place (line and column count from 1). */
export interface CompilerDiagnostic {
    file: string;
    line: number;
    column?: number;
    severity: 'error' | 'warning' | 'information';
    message: string;
    code?: string;
}

/**
 * Reads the compiler's diagnostics from its plain-text output:
 *   `[!!] QBNex :: Fatal [E1201]  message` (also `[x]` error, `[~]` warning, `[i]` note)
 *   followed by `  [@] path(line[,column])`, and the `name.bas:LINE: warning: text` lines
 *   printed with -w, whose name is relative to the source folder.
 * Paths may mix `\` and `/`; they come back with the platform's separator.
 */
export function parseDiagnostics(text: string, sourceDir: string, platform: string): CompilerDiagnostic[] {
    const p = pathFor(platform);
    const place = (f: string): string => {
        const s = f.trim().replace(/[\\/]/g, p.sep);
        return p.normalize(p.isAbsolute(s) ? s : p.join(sourceDir, s));
    };
    const severities: Record<string, CompilerDiagnostic['severity']> = { '!!': 'error', x: 'error', '~': 'warning', i: 'information' };
    // colour codes, in case a terminal-style output reaches us
    const lines = text.replace(/\x1b\[[0-9;]*[A-Za-z]/g, '').split(/\r?\n/);
    const out: CompilerDiagnostic[] = [];
    for (let i = 0; i < lines.length; i++) {
        const head = /^\[(!!|x|~|i)\] QBNex :: .*?\[([^\]]+)\]\s+(.*)$/.exec(lines[i]);
        if (head) {
            const at = /^\s+\[@\]\s+(.+)\((\d+)(?:,\s*(\d+))?\)\s*$/.exec(lines[i + 1] ?? '');
            if (at) {
                out.push({
                    file: place(at[1]),
                    line: Number(at[2]),
                    column: at[3] === undefined ? undefined : Number(at[3]),
                    severity: severities[head[1]],
                    message: head[3].trim(),
                    code: head[2],
                });
                i++;
            }
            continue;
        }
        const warn = /^(.+?):(\d+): warning: (.*)$/.exec(lines[i]);
        if (warn) {
            const file = place(warn[1]);
            const line = Number(warn[2]);
            // the same place already reported by a header (a warning -Werror promoted): once
            if (!out.some((d) => d.file === file && d.line === line)) {
                out.push({ file, line, severity: 'warning', message: warn[3].trim() });
            }
        }
    }
    return out;
}

/** A launch configuration with defaults filled in (F5 without a launch.json). */
export interface DebugConfig {
    type?: string;
    request?: string;
    name?: string;
    program?: string;
    compiler?: string;
    [key: string]: unknown;
}

export function completeDebugConfig(cfg: DebugConfig, activeFile: string | undefined, compiler: string): DebugConfig | undefined {
    const out: DebugConfig = { ...cfg };
    if (!out.type && !out.request && !out.name) {
        if (!activeFile) {
            return undefined;
        }
        out.type = 'qbnex';
        out.request = 'launch';
        out.name = 'Debug QBNex program';
        out.program = activeFile;
    }
    if (!out.program) {
        if (!activeFile) {
            return undefined;
        }
        out.program = activeFile;
    }
    if (!out.compiler) {
        out.compiler = compiler;
    }
    return out;
}
