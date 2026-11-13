# Web (WebAssembly) build

[Manual index](../README.md) · [ภาษาไทย](../th/web.md)

Status: **Preview**. QBNex programs can run in a browser: the QIR runner
(QIR -> bytecode -> VM, no JIT) is built for `wasm32-unknown-unknown` as
`qb_web.wasm`, and a small static page in `crates/web/www` runs `.qir` files.
Nothing is compiled in the browser. Details for developers:
[crates/web/README.md](../../../crates/web/README.md).

## Build

```bash
rustup target add wasm32-unknown-unknown        # once
cargo build -p qb-web --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/qb_web.wasm crates/web/www/
```

The module is about 4 MB.

## Get a program as QIR

```bash
qb game.bas -o game.qir          # or: qb game.bas --emit-ir
```

An output name ending in `.qir` (or `--emit-ir`) makes the compiler write the
program's QIR text instead of an executable. That text is what the page runs.

## Run it

```bash
node crates/web/www/serve.mjs          # http://localhost:8080/
```

`serve.mjs` serves `www/` with the cross-origin isolation headers (COOP/COEP)
that live keyboard and mouse input need (the page shares a
`SharedArrayBuffer` with the worker). It sends `qb_web.wasm` from `www/`, or
else from `target/wasm/wasm32-unknown-unknown/release/` (a build with
`CARGO_TARGET_DIR=target/wasm`). Any other static server works when
`qb_web.wasm` is in `www/`; without the headers programs still run, and
INPUT reads only the prepared answers.

The page has a list of examples (`hello`: text, COLOR, BEEP; `graphics`: a
SCREEN 13 animation; `guess`: an INPUT game with SOUND and PLAY), a box to open
or paste a `.qir` file, the screen canvas (click it to type), a box of prepared
answers for INPUT, and a console pane for `$CONSOLE` / `_DEST _CONSOLE` output.

- `worker.js` runs the program off the page thread: `_DELAY`, `_LIMIT` and
  `SLEEP` really wait, and frames stream to the canvas while it runs.
- `qbnex.js` loads a fresh instance per run.
- Text screens use the builtin 8x16 font; SCREEN 13, 32-bit and palette
  pages are drawn as pixels.

## What works / what does not

| QBasic | In the browser |
| --- | --- |
| PRINT, CLS, LOCATE, COLOR, SCREEN 0/13/32-bit graphics, `_DISPLAY` | Drawn by the runtime; frames are posted on `_DISPLAY`, `_LIMIT`, `_DELAY` and while the program polls input |
| INKEY$, `_KEYHIT`, `_KEYDOWN`, `_MOUSEINPUT`, `_MOUSEX`, `_MOUSEY`, `_MOUSEBUTTON`, `_MOUSEWHEEL` | Keyboard and mouse events of the canvas (needs the isolation headers) |
| INPUT, LINE INPUT | The prepared answers first, then live typing on the screen |
| SOUND, BEEP, PLAY, `_SND*` | Mixed at 44.1 kHz and played through WebAudio; only the `PLAY "MB"` queue limit applies |
| TIMER, `_DELAY`, `_LIMIT`, `SLEEP n` | The page's clock |
| Files, SHELL, TCP, HTTP | Not available: they raise ordinary QBasic errors (OPEN gives error 75 or 53), which `ON ERROR` can trap |
| JIT | Not available (the VM only) |

Unsupported statements end in QBasic errors, not crashes. Tests:
`node crates/web/tests/run.mjs [path/to/qb_web.wasm]` (headless, in Node) and
`cargo test -p qb-web`.
