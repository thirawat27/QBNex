// Runs QBNex programs off the page's thread: _DELAY/_LIMIT really wait here,
// and frames, console text and sound are posted to the page while the
// program runs. Keyboard and mouse events arrive through a SharedArrayBuffer
// ring the page writes (only when the page is cross-origin isolated; see
// serve.mjs), because a busy worker cannot receive messages.
import { QBNex, RING } from './qbnex.js';

let qb = null;
let ring = null;
let lastOut = '';

function pollEvent() {
  if (!ring) return null;
  const written = Atomics.load(ring, 0);
  const read = Atomics.load(ring, 1);
  if (read >= written) return null;
  const at = 2 + (read % RING) * 6;
  const e = Array.from(ring.subarray(at, at + 6));
  Atomics.store(ring, 1, read + 1);
  return e;
}

const hooks = {
  interactive: false,
  pollEvent,
  onFrame(f) {
    postMessage({ type: 'frame', frame: f }, [f.rgba.buffer]);
    const out = qb.stdoutSoFar();
    if (out !== lastOut) {
      lastOut = out;
      postMessage({ type: 'stdout', text: out });
    }
  },
  onAudio(pcm, rate) {
    postMessage({ type: 'audio', pcm, rate }, [pcm.buffer]);
  },
};

const ready = QBNex.load(fetch('qb_web.wasm'), hooks).then((q) => {
  qb = q;
  postMessage({ type: 'ready' });
}).catch((e) => postMessage({ type: 'fatal', message: String(e && e.message || e) }));

onmessage = async (ev) => {
  const m = ev.data;
  if (m.type === 'events') {
    ring = new Int32Array(m.sab);
    hooks.interactive = true;
    return;
  }
  await ready;
  if (!qb) return;
  if (ring) Atomics.store(ring, 1, Atomics.load(ring, 0)); // drop keys typed before the run
  lastOut = '';
  const t0 = performance.now();
  const r = qb.run(m.qir, m.stdin);
  r.ms = performance.now() - t0;
  const transfer = r.frame ? [r.frame.rgba.buffer] : [];
  postMessage({ type: 'done', result: r }, transfer);
};
