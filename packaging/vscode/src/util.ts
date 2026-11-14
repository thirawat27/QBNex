// Helpers without a VS Code dependency (unit-tested with node --test).

import * as path from 'node:path';

/** Executable suffix for a platform. */
export function exeExtension(platform: string): string {
    return platform === 'win32' ? '.exe' : '';
}

/** Replaces `${name}` in each argument; unknown names stay as written. */
export function expandArgs(args: readonly string[], vars: Record<string, string>): string[] {
    return args.map((a) => a.replace(/\$\{(\w+)\}/g, (m, name: string) => (Object.prototype.hasOwnProperty.call(vars, name) ? vars[name] : m)));
}

/** Places to look for a tool (qb-lsp, qb-dap), in order: the setting, the bundled copy, PATH. */
export function serverCandidates(configured: string, extensionPath: string, platform: string, tool = 'qb-lsp'): string[] {
    const out: string[] = [];
    if (configured) {
        out.push(configured);
    }
    out.push(path.join(extensionPath, 'server', tool + exeExtension(platform)));
    out.push(tool);
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