// Serves the QBNex web runner on http://localhost:8080 (node www/serve.mjs [port]).
//
// Sends the cross-origin isolation headers (COOP/COEP) the page needs for
// live keyboard and mouse input (SharedArrayBuffer), and serves qb_web.wasm
// from this folder or, when it is not here, from the cargo build output
// (target/wasm/wasm32-unknown-unknown/release). Local use only.
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { dirname, extname, join, normalize, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const www = dirname(fileURLToPath(import.meta.url));
const built = resolve(www, '../../../target/wasm/wasm32-unknown-unknown/release/qb_web.wasm');
const port = Number(process.argv[2]) || 8080;
const types = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.wasm': 'application/wasm',
  '.qir': 'text/plain; charset=utf-8',
  '.bas': 'text/plain; charset=utf-8',
};

createServer(async (req, res) => {
  let path;
  try {
    path = decodeURIComponent(new URL(req.url, 'http://x').pathname);
  } catch {
    // Malformed escape (`/%`): a throw here would reject the handler and
    // stop the server.
    res.writeHead(400).end('bad request');
    return;
  }
  let file = normalize(join(www, path === '/' ? 'index.html' : path));
  if (file !== www && !file.startsWith(www + sep)) {
    res.writeHead(403).end();
    return;
  }
  if (path === '/qb_web.wasm' && !existsSync(file)) file = built;
  try {
    const body = await readFile(file);
    res.writeHead(200, {
      'Content-Type': types[extname(file)] || 'application/octet-stream',
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp',
      'Cache-Control': 'no-cache',
    });
    res.end(body);
  } catch {
    res.writeHead(404).end('not found');
  }
}).listen(port, '127.0.0.1', () => console.log(`QBNex web runner: http://localhost:${port}/`));
