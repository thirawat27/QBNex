# Web (WebAssembly) build

[Manual index](../README.md) · [ภาษาไทย](../th/web.md)

Status: **Preview**. QBNex programs can run in a browser: the QIR runner
(QIR -> bytecode -> VM, no JIT) is built for `wasm32-unknown-unknown` as
`qb_web.wasm`, and a small static page runs `.qir` files.

## Build

```bash
cargo build -p qb-web --release --target wasm32-unknown-unknown
# (rustup target add wasm32-unknown-unknown once)
cp target/wasm32-unknown-unknown/release/qb_web.wasm crates/qb-web/www/
```

## Get a program as QIR

```bash
qb game.bas --ir -o game.exe
```

The QIR text is the payload stored before the `QBNEXIR0` trailer at the end
of `game.exe` (the last 16 bytes are the length and the trailer). The
debugger's code shows how to read it (`qir_payload` in
`crates/qb-dap/src/adapter.rs`).

## Run it

Serve `crates/qb-web/www` over HTTP (module workers do not load from
`file://`), for example `python -m http.server` in that folder, open the
page and choose the `.qir` file. The page has a canvas for the screen, a box
for standard input and a console pane.

- `worker.js` runs the program off the page thread: `_DELAY`, `_LIMIT` and
  `SLEEP` really wait, and frames stream to the canvas while it runs.
- `qbnex.js` loads a fresh instance per run.
- Text screens use the builtin 8x16 font; SCREEN 13, 32-bit and palette
  pages are drawn as pixels.

## What works / what does not

| Works | Not yet |
| --- | --- |
| PRINT, LOCATE, COLOR, CLS, SCREEN 0/13/32-bit graphics, TIMER, `_DELAY`, `_LIMIT`, `_DISPLAY` | Interactive keyboard (`INKEY$` is empty; INPUT reads the text given before the run) |
| INPUT / LINE INPUT from the stdin box (echoed) | Files (OPEN fails with a normal error you can trap) |
| `ON ERROR` for unsupported calls | TCP, HTTP, SHELL |
| | Sound (silent), JIT |

Unsupported statements end in QBasic errors, not crashes. Tests:
`node crates/qb-web/tests/run.mjs` (33 checks).
