// Copies built qb-lsp and qb-dap into server/ so the .vsix carries them.
// Build them first: cargo build --release -p qb-lsp -p qb-dap [--target <rust triple>]
// Then copy and package for one platform:
//   npm run bundle-server -- --target linux-x64
//   npx vsce package --target linux-x64
// The platform (a vsce target: win32-x64, linux-x64, darwin-arm64, ...) comes from --target,
// else the QBNEX_VSCE_TARGET environment variable, else the machine this runs on. It picks the
// .exe suffix and the cargo folder (target/<triple>/release for a cross build). A debug build
// is copied only with --allow-debug: it is slow and was easy to ship by mistake.
import { copyFileSync, existsSync, mkdirSync, rmSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const argv = process.argv.slice(2);
function option(name) {
    const i = argv.indexOf(name);
    if (i >= 0) {
        return argv[i + 1];
    }
    return argv.find((a) => a.startsWith(name + '='))?.slice(name.length + 1);
}

// vsce target -> the Rust target triples that build for it
const triples = {
    'win32-x64': ['x86_64-pc-windows-msvc', 'x86_64-pc-windows-gnu', 'x86_64-pc-windows-gnullvm'],
    'win32-arm64': ['aarch64-pc-windows-msvc', 'aarch64-pc-windows-gnullvm'],
    'linux-x64': ['x86_64-unknown-linux-gnu'],
    'linux-arm64': ['aarch64-unknown-linux-gnu'],
    'linux-armhf': ['armv7-unknown-linux-gnueabihf'],
    'alpine-x64': ['x86_64-unknown-linux-musl'],
    'alpine-arm64': ['aarch64-unknown-linux-musl'],
    'darwin-x64': ['x86_64-apple-darwin'],
    'darwin-arm64': ['aarch64-apple-darwin'],
};
const host = `${process.platform}-${process.arch === 'arm' ? 'armhf' : process.arch}`;
const platform = option('--target') ?? process.env.QBNEX_VSCE_TARGET ?? host;
if (!triples[platform]) {
    console.error(`unknown target ${platform}; use one of: ${Object.keys(triples).join(', ')}`);
    process.exit(1);
}
const allowDebug = argv.includes('--allow-debug');
const ext = platform.startsWith('win32-') ? '.exe' : '';
const otherExt = ext ? '' : '.exe';
const here = dirname(fileURLToPath(import.meta.url));
const target = process.env.CARGO_TARGET_DIR ?? join(here, '..', '..', '..', 'target');
// a build without --target writes target/<profile> for the machine it runs on
const roots = triples[platform].map((t) => join(target, t));
if (platform === host) {
    roots.push(target);
}
const server = join(here, '..', 'server');
mkdirSync(server, { recursive: true });
let missing = 0;
for (const tool of ['qb-lsp', 'qb-dap']) {
    const exe = tool + ext;
    // a copy left from packaging for another platform must not ride along
    rmSync(join(server, tool + otherExt), { force: true });
    const find = (profile) => roots.map((r) => join(r, profile, exe)).find(existsSync);
    let from = find('release');
    if (!from) {
        const debug = find('debug');
        if (debug && allowDebug) {
            console.warn(`using the debug build ${debug} (--allow-debug)`);
            from = debug;
        } else {
            const hint = debug ? ` (only a debug build is there: ${debug}; pass --allow-debug to ship it)` : '';
            console.error(`no release ${exe} for ${platform} under ${target}${hint}; run: cargo build --release -p ${tool}` +
                (platform === host ? '' : ` --target ${triples[platform][0]}`));
            missing++;
            continue;
        }
    }
    copyFileSync(from, join(server, exe));
    console.log(`copied ${from}`);
}
process.exit(missing ? 1 : 0);
