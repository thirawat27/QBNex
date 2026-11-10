// Copies a built qb-lsp into server/ so the .vsix carries its own server.
// Build it first: cargo build --release -p qb-lsp
// Then package for that platform: npx vsce package --target win32-x64 (or linux-x64, darwin-arm64, ...)
import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const exe = process.platform === 'win32' ? 'qb-lsp.exe' : 'qb-lsp';
const target = join(here, '..', '..', '..', 'target');
const from = ['release', 'debug'].map((p) => join(target, p, exe)).find(existsSync);
if (!from) {
    console.error(`no ${exe} under ${target}; run: cargo build --release -p qb-lsp`);
    process.exit(1);
}
mkdirSync(join(here, '..', 'server'), { recursive: true });
copyFileSync(from, join(here, '..', 'server', exe));
console.log(`copied ${from}`);
