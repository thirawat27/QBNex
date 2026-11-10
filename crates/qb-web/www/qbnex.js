// QBNex in a browser (R13-wasm): loads qb_web.wasm and runs QIR programs.
//
// Works in a Web Worker (worker.js, used by index.html: sleeps really wait,
// frames stream to the page while the program runs) and in Node
// (tests/run.mjs). Each run gets a fresh instance of the compiled module, so
// no runtime state leaks from one program into the next. On a page's main
// thread, use the worker: a run blocks its thread until the program ends.
//
//   const qb = await QBNex.load(wasmBytesOrResponse, { onFrame, onSleep });
//   const r = qb.run(qirText, stdinText);
//   r.status        0 finished, 1 runtime error, 2 did not load
//   r.exitCode      SYSTEM n
//   r.error, r.line QBasic error number and source line
//   r.message       error text
//   r.stdout        console output ($CONSOLE / _DEST _CONSOLE)
//   r.screenText    characters of the final text screen
//   r.frame         { width, height, rgba: Uint8ClampedArray } of the final screen

const dec = new TextDecoder('latin1');

function nowMs() {
  return Date.now();
}

function monoMs() {
  return typeof performance !== 'undefined' ? performance.now() : Date.now();
}

// Blocks for `ms` where the platform allows it (workers, Node); returns at
// once on a page's main thread, which must never block.
function defaultSleep(ms) {
  const isWindow = typeof window !== 'undefined' && typeof document !== 'undefined';
  if (isWindow || !(ms > 0)) return;
  const until = monoMs() + Math.min(ms, 60000);
  while (monoMs() < until) { /* busy wait: no SharedArrayBuffer needed */ }
}

export class QBNex {
  constructor(module, hooks) {
    this.module = module;
    this.hooks = hooks;
    this.x = null;
  }

  /**
   * @param {BufferSource|Response|Promise<Response>} source the .wasm
   * @param {{onFrame?:(f:{width:number,height:number,rgba:Uint8ClampedArray})=>void,
   *          onSleep?:(ms:number)=>void}} hooks
   */
  static async load(source, hooks = {}) {
    let bytes = await source;
    if (typeof Response !== 'undefined' && bytes instanceof Response) bytes = await bytes.arrayBuffer();
    const module = await WebAssembly.compile(bytes);
    const qb = new QBNex(module, hooks);
    qb.instantiate();
    return qb;
  }

  /** New instance with the page's imports (fresh runtime state). */
  instantiate() {
    const { module, hooks } = this;
    const self = this;
    const qbnex = {
      qb_web_now_ms: nowMs,
      qb_web_monotonic_ms: monoMs,
      qb_web_sleep_ms: (ms) => (hooks.onSleep || defaultSleep)(ms),
      qb_web_present: () => {
        if (hooks.onFrame) {
          const f = self.frame(0);
          if (f) hooks.onFrame(f);
        }
      },
    };
    // Every other import is a host service the browser build does not have:
    // calling one stops the program with a clear message instead of a link error.
    const imports = { qbnex };
    for (const imp of WebAssembly.Module.imports(module)) {
      if (imp.module === 'qbnex' && imp.name in qbnex) continue;
      if (imp.kind !== 'function') throw new Error(`qb_web.wasm needs ${imp.module}.${imp.name} (${imp.kind})`);
      (imports[imp.module] ||= {})[imp.name] = () => {
        throw new Error(`unsupported in the browser build: ${imp.module}.${imp.name}`);
      };
    }
    this.x = new WebAssembly.Instance(module, imports).exports;
  }

  /** Bytes of the result buffer (copy). */
  buf(len) {
    return new Uint8Array(this.x.memory.buffer, this.x.qb_web_buf(), len).slice();
  }

  text(kind) {
    return dec.decode(this.buf(this.x.qb_web_text(kind)));
  }

  /** @param {0|1} which 0 = frame of the last _DISPLAY, 1 = final screen */
  frame(which) {
    if (!this.x.qb_web_frame(which)) return null;
    const width = this.x.qb_web_frame_width();
    const height = this.x.qb_web_frame_height();
    const rgba = new Uint8ClampedArray(this.buf(width * height * 4).buffer);
    return { width, height, rgba };
  }

  /** Console output so far (call from onFrame for a live console). */
  stdoutSoFar() {
    return dec.decode(this.buf(this.x.qb_web_poll_stdout()));
  }

  put(bytes) {
    const p = this.x.qb_web_alloc(bytes.length);
    new Uint8Array(this.x.memory.buffer, p, bytes.length).set(bytes);
    return p;
  }

  /**
   * @param {string} qir program text (`qb --ir`)
   * @param {string} stdin text INPUT and LINE INPUT read (lines end with \n)
   */
  run(qir, stdin = '') {
    if (this.used) this.instantiate();
    this.used = true;
    const enc = new TextEncoder();
    const src = enc.encode(qir);
    const inp = Uint8Array.from(stdin, (c) => c.charCodeAt(0) & 255);
    const ps = this.put(src);
    const pi = this.put(inp);
    let status;
    try {
      status = this.x.qb_web_run(ps, src.length, pi, inp.length);
    } catch (e) {
      // a trap (e.g. an unsupported import or a VM abort): the instance state is gone
      return { status: 3, exitCode: 0, error: 0, line: 0, message: String(e && e.message || e), stdout: '', screenText: '', frame: null };
    } finally {
      try {
        this.x.qb_web_free(ps, src.length);
        this.x.qb_web_free(pi, inp.length);
      } catch { /* instance trapped */ }
    }
    return {
      status,
      exitCode: this.x.qb_web_exit_code(),
      error: this.x.qb_web_error(),
      line: this.x.qb_web_error_line(),
      message: this.text(1),
      stdout: this.text(0),
      screenText: this.text(2),
      frame: this.frame(1),
    };
  }
}

export default QBNex;
