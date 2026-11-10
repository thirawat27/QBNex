// Runs QBNex programs off the page's thread: _DELAY/_LIMIT really wait here,
// and every _DISPLAY frame is posted to the page while the program runs.
import { QBNex } from './qbnex.js';

let qb = null;
let lastOut = '';

const ready = QBNex.load(fetch('qb_web.wasm'), {
  onFrame(f) {
    postMessage({ type: 'frame', frame: f }, [f.rgba.buffer]);
    const out = qb.stdoutSoFar();
    if (out !== lastOut) {
      lastOut = out;
      postMessage({ type: 'stdout', text: out });
    }
  },
}).then((q) => {
  qb = q;
  postMessage({ type: 'ready' });
}).catch((e) => postMessage({ type: 'fatal', message: String(e && e.message || e) }));

onmessage = async (ev) => {
  const { qir, stdin } = ev.data;
  await ready;
  if (!qb) return;
  lastOut = '';
  const t0 = performance.now();
  const r = qb.run(qir, stdin);
  r.ms = performance.now() - t0;
  const transfer = r.frame ? [r.frame.rgba.buffer] : [];
  postMessage({ type: 'done', result: r }, transfer);
};
