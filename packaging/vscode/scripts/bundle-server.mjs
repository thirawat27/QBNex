// Copies built qb-lsp and qb-dap into server/ so the .vsix carries them.
// Build them first: cargo build --release -p qb-lsp -p qb-dap
// Then package for that platform: npx vsce package --target win32-x64 (or linux-x64, darwin-arm64, ...)
import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const ext = process.platform === 'win32' ? '.exe' : '';
const target = join(here, '..', '..', '..', 'target');
mkdirSync(join(here, '..', 'server'), { recursive: true });
let missing = 0;
for (const tool of ['qb-lsp', 'qb-dap']) {
    const exe = tool + ext;
    const from = ['release', 'debug'].map((p) => join(target, p, exe)).find(existsSync);
    if (!from) {
        console.error(`no ${exe} under ${target}; run: cargo build --release -p ${tool}`);
        missing++;
        continue;
    }
    copyFileSync(from, join(here, '..', 'server', exe));
    console.log(`copied ${from}`);
}
process.exit(missing ? 1 : 0);
