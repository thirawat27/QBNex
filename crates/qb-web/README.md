# qb-web — QBNex programs in a browser

`qb-web` is the QBNex VM (`qb-exec`, no JIT) compiled to WebAssembly. Nothing is
compiled in the browser: you compile a program once with `qb --ir`, and the page
runs the resulting QIR.

## 1. Build the wasm module

Needs Rust with the `wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`).

```
cargo build -p qb-web --release --target wasm32-unknown-unknown
```

The module is `target/wasm32-unknown-unknown/release/qb_web.wasm`, about 4 MB.
The port's notes use `CARGO_TARGET_DIR=target/wasm` so this build does not
disturb the native one. On a machine short of memory, also set
`CARGO_BUILD_JOBS=1` and pass
`--config 'profile.release.package.qb-exec.codegen-units=16'`.

## 2. Compile a program to QIR

```
qb prog.bas --ir -o prog.exe
```

`prog.exe` is the qb-exec runner with the QIR appended. The QIR is the text
before the 16-byte trailer: a u32 length, a u32 zero, then `QBNEXIR0`.
The page also takes the plain QIR text. `www/examples/*.qir` were made this
way from the `.bas` files next to them.

## 3. Serve the page

```
node crates/qb-web/www/serve.mjs          # http://localhost:8080/
```

`serve.mjs` serves `www/`. It sends `qb_web.wasm` from `www/` if a copy is
there, otherwise from `target/wasm/wasm32-unknown-unknown/release/`.
It also sends the cross-origin isolation headers. Live keyboard and mouse need
them, because the page shares a `SharedArrayBuffer` with the worker.

Any static server works if you copy `qb_web.wasm` into `www/`. Without those
headers, programs still run, and INPUT uses the prepared answers box.

The page has these parts:

- **Examples:**
  - `hello`: text, COLOR, BEEP
  - `graphics`: a SCREEN 13 animation with `_DISPLAY`, `_LIMIT` and INKEY$ to stop
  - `guess`: an INPUT number game with SOUND and PLAY
- **Program box:** open a `.qir` file or paste one.
- **Screen:** a canvas. Click it to type.
- **Prepared answers:** INPUT reads these first, then waits for typing.
- **Console pane:** `$CONSOLE` / `_DEST _CONSOLE` output.

## What runs where

| QBasic | Browser |
|---|---|
| PRINT, CLS, LOCATE, COLOR, SCREEN 0/13 and the other graphics | drawn on qb-runtime surfaces; the worker posts frames on `_DISPLAY`, `_LIMIT`, `_DELAY` and while the program polls input |
| INKEY$, `_KEYHIT`, `_KEYDOWN` | keyboard events of the canvas, translated by qb-runtime's keymap (same codes as the desktop window) |
| `_MOUSEINPUT`, `_MOUSEX/_MOUSEY`, `_MOUSEBUTTON`, `_MOUSEWHEEL` | mouse events of the canvas, in screen pixels |
| INPUT, LINE INPUT | prepared answers first, then live typing on the screen (Backspace, Enter) |
| SOUND, BEEP, PLAY, `_SND*` | qb-audio's mixer at 44.1 kHz; the worker hands PCM to WebAudio |
| TIMER, `_DELAY`, `_LIMIT`, SLEEP n | the page's clock; the worker really waits |
| files, SHELL, TCP/HTTP | QBasic errors (no file system or network in the browser build) |

## Tests

```
node crates/qb-web/tests/run.mjs [path/to/qb_web.wasm]
```

These are headless checks in Node. They cover the text screen, COLOR/LOCATE
pixels, INPUT, SCREEN 13 pixels, TIMER and `_DELAY`, scripted keyboard and
mouse events, live-typed LINE INPUT, SOUND/PLAY PCM, and the three examples.
`cargo test -p qb-web` adds unit tests of the screen renderer.
