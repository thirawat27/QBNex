// Helpers without a VS Code dependency (unit-tested with node --test).

import * as path from 'node:path';

/** Executable suffix for a platform. */
export function exeExtension(platform: string): string {
    return platform === 'win32' ? '.exe' : '';
}

/** Replaces `${name}` in each argument; unknown names stay as written. */
export function expandArgs(args: readonly string[], vars: Record<string, string>): string[] {
    return args.map((a) => a.replace(/\$\{(\w+)\}/g, (m, name: string) => (name in vars ? vars[name] : m)));
}

/** Places to look for qb-lsp, in order: the setting, the bundled copy, PATH. */
export function serverCandidates(configured: string, extensionPath: string, platform: string): string[] {
    const out: string[] = [];
    if (configured) {
        out.push(configured);
    }
    out.push(path.join(extensionPath, 'server', 'qb-lsp' + exeExtension(platform)));
    out.push('qb-lsp');
    return out;
}
