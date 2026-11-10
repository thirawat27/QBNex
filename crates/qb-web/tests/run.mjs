// Headless tests of the browser build: node crates/qb-web/tests/run.mjs [qb_web.wasm]
// (default: target/wasm/wasm32-unknown-unknown/release/qb_web.wasm). Exit code 1 on failure.
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { QBNex } from '../www/qbnex.js';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '../../..');
const wasmPath = process.argv[2] || join(root, 'target/wasm/wasm32-unknown-unknown/release/qb_web.wasm');
const fixture = (n) => readFileSync(join(here, 'fixtures', n + '.qir'), 'utf8');

let frames = 0;
const qb = await QBNex.load(readFileSync(wasmPath), { onFrame: () => frames++ });

let failed = 0;
let passed = 0;
function check(name, cond, detail) {
  if (cond) {
    passed++;
  } else {
    failed++;
    console.log(`FAIL ${name}${detail === undefined ? '' : ': ' + JSON.stringify(detail)}`);
  }
}

/** RGB at pixel (x, y) of a frame. */
function px(f, x, y) {
  const i = 4 * (y * f.width + x);
  return [f.rgba[i], f.rgba[i + 1], f.rgba[i + 2]];
}
const same = (a, b) => a.length === b.length && a.every((v, i) => v === b[i]);
const lines = (r) => r.screenText.split('\n');

// PRINT draws on the SCREEN 0 page (80x25 text, 8x16 font).
{
  const r = qb.run(fixture('hello'));
  check('hello status', r.status === 0, r);
  check('hello text', r.screenText === 'Hello, QBNex\n 1  2  3\ndone\n', r.screenText);
  check('hello frame size', r.frame && r.frame.width === 640 && r.frame.height === 400, r.frame && [r.frame.width, r.frame.height]);
  check('hello white on black', same(px(r.frame, 0, 0), [0, 0, 0]));
}

// CLS / COLOR / LOCATE / CSRLIN / POS
{
  const r = qb.run(fixture('console'));
  const l = lines(r);
  check('console status', r.status === 0, r);
  check('console locate', l[4] === '         Yellow on blue', l[4]);
  check('console csrlin', l[6] === 'row 6', l[6]);
  check('console pos', l[7] === ' 1', l[7]);
  // cell (row 5, column 10) has a blue background, and some yellow glyph pixels
  const f = r.frame;
  check('console background', same(px(f, 9 * 8, 4 * 16), [0, 0, 170]), px(f, 9 * 8, 4 * 16));
  let yellow = false;
  for (let y = 0; y < 16; y++) for (let x = 0; x < 8; x++) if (same(px(f, 9 * 8 + x, 4 * 16 + y), [255, 255, 85])) yellow = true;
  check('console foreground', yellow);
  check('console row 7 default colors', same(px(f, 0, 6 * 16), [0, 0, 0]));
}

// INPUT / LINE INPUT read the stdin text and echo it on the screen
{
  const r = qb.run(fixture('input'), 'Ann\n3,4\nhello world\n');
  const l = lines(r);
  check('input status', r.status === 0, r);
  check('input prompt+echo', l[0] === 'Name? Ann', l[0]);
  check('input greeting', l[1] === 'Hi Ann', l[1]);
  check('input numbers', l[2] === '? 3,4' && l[3] === ' 7', l.slice(2, 4));
  check('input line', l[4] === 'hello world' && l[5] === '[hello world]', l.slice(4, 6));
}

// INPUT past the end of the given text: QBasic error 62 (input past end)
{
  const r = qb.run(fixture('input'), 'Bob\n');
  check('input eof status', r.status === 1 && r.error === 62, r);
}

// SCREEN 13 framebuffer: 320x200, VGA palette
{
  const r = qb.run(fixture('screen13'));
  const f = r.frame;
  check('screen13 status', r.status === 0, r);
  check('screen13 size', f && f.width === 320 && f.height === 200, f && [f.width, f.height]);
  check('screen13 pset', same(px(f, 10, 10), [170, 0, 0]), px(f, 10, 10));
  check('screen13 box', same(px(f, 30, 25), [85, 85, 255]), px(f, 30, 25));
  check('screen13 line', same(px(f, 0, 199), [255, 255, 255]) && same(px(f, 319, 199), [255, 255, 255]), px(f, 0, 199));
  check('screen13 circle', same(px(f, 210, 100), [0, 170, 0]), px(f, 210, 100));
  check('screen13 background', same(px(f, 300, 5), [0, 0, 0]), px(f, 300, 5));
  check('screen13 no text', r.screenText === '');
}

// TIMER and _DELAY use the page's clock
{
  const r = qb.run(fixture('timer'));
  const l = lines(r);
  check('timer status', r.status === 0, r);
  check('timer delay', l[0] === 'timer ok', l[0]);
  check('timer range', l[1] === 'range ok', l[1]);
}

// Host services the browser does not have end in a QBasic error, not a trap
{
  const r = qb.run(fixture('unsupported'));
  const l = lines(r).filter((s) => s);
  check('unsupported status', r.status === 0, r);
  check('unsupported trapped error', /^error \d+$/.test(l[0]), l);
  check('unsupported continues', l[l.length - 1] === 'end', l);
}

// a broken program reports a load error
{
  const r = qb.run('QIR 1\nfunc main\nbogus\nendfunc\n');
  check('load error', r.status === 2 && r.message.length > 0, r);
}

// a fresh instance per run: the screen of the previous program is gone
{
  const r = qb.run(fixture('hello'));
  check('fresh instance', r.screenText.startsWith('Hello, QBNex'), r.screenText);
}

console.log(`qb-web: ${passed} passed, ${failed} failed`);
process.exit(failed ? 1 : 0);
