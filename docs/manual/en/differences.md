# Differences from QB64 and QBasic

[Manual index](../README.md) · [ภาษาไทย](../th/differences.md)

QBNex started from QB64. Old programs should compile and give the same
output; this page lists where they do not, and why. The IDs (D-xx, L-xx)
are explained, with their tests, in
[`docs/dev/deviations.md`](../../dev/deviations.md).

## The tool

- **No IDE.** QBNex is a command-line compiler (`qb`, `qbnex`). Use any
  editor; VS Code has an [extension](vscode.md). `$DEBUG` and the `-l`
  switch of the QB64 IDE are not supported.
- **Standard library and `'$IMPORT`** are QBNex additions
  ([stdlib](stdlib.md)). Imported modules are hoisted: their declarations go
  before the program and their procedures after it (D-25).
- **Source encoding:** CP437 files compile byte for byte, UTF-8 too (D-22).
- **The runtime is Rust.** No OpenGL, FreeGLUT, FreeType, miniaudio or zlib
  is linked; behaviour was checked against the old runtime with differential
  tests (billions of compared results, 0 mismatches outside the items below).

## Intentional changes (bugs fixed on purpose)

| ID | What changed |
| --- | --- |
| D-01 | `VAL("123E-2")` is `1.23` (QB64 gave `.000123`: the exponent sign was applied after normalising). |
| D-02 | Rounding to integers (`CINT`, `CLNG`, assignment to integer variables) is round-half-even on every CPU (QB64 on ARM rounded half away from zero). |
| D-03 | `VAL` reads 1000 significant digits exactly (QB64 overran a buffer past 256). |
| D-04, D-05 | `STR$` of doubles needing more than 19 decimals prints the digits (QB64 printed ` .0f`); `STR$` of a non-finite SINGLE is ` 0`. |
| D-06 | Main-module code before the first SUB stays in the main module; `ON ERROR GOTO` handlers in the main module work. |
| D-07 | `_MEMFREE` of a `_MEMNEW` block no longer frees twice (heap corruption in QB64). |
| D-08, D-10, D-11, D-12, D-R12b-1, D-R12b-3 | Reads past buffers in QB64 (GET of strings, FILES, SHELL, stretched `_PUTIMAGE`, 2-bit GET, HEX$) now give defined results. |
| D-09 | `SHELL` keeps window events paused for the whole waited call. |
| D-18 | The millisecond clock is 64-bit everywhere (QB64 on Windows wrapped after 24.8 days). |
| D-19, D-20 | `_CLIPBOARD$` and `_CLIPBOARDIMAGE` work on Linux/macOS (UTF-8 clipboard: non-UTF-8 bytes become U+FFFD there). |
| D-21 | `_OPENCLIENT` tries every IPv4 address of a host on every OS. |
| D-22 | CP437 source files compile (QBNex used to reject them). |
| D-23, D-24, D-26 | Compiler fixes: `CALL ABSOLUTE` wrappers, `PLAY ON/OFF/STOP` and `ON PLAY(n) GOSUB` compile and the PLAY event is raised, relational operators inside PRINT. |
| D-R12b-5 | Integer `\` and `MOD` by zero raise error 11 as an ordinary error that `ON ERROR` and `TRY`/`CATCH` catch (QBasic 4.5); QB64 ended the program. |
| D-R12b-2 | READ of decimal numbers into DOUBLE/`_FLOAT` is correctly rounded. |
| D-R12b-4 | With redirected output or `QBNEX_NO_ERROR_DIALOG=1`, a runtime error prints one line to stderr and exits with code 1 instead of opening a dialog. |
| D-R11b-1 | `PUT` to a TCP connection whose send buffer is full keeps the connection: the rest of the data is queued and sent, in order, by later network calls and `CLOSE` (QB64 dropped the connection and the data). |

## QBasic behaviour kept on purpose

These match QB64 (golden outputs depend on them): `MKSMBF$(0)` gives MBF
exponent 2; some DOUBLE values print a trailing zero (`-2.50`); `STR$` of
infinity/NaN is ` 0`; graphics keep QB64's float-to-integer conversions;
`_CONNECTIONADDRESS` shows a byte-swapped remote port; TIMER waits for the
next second at program start.

## Known defects not fixed yet

See [Limitations and differences from QB64](limitations.md#known-defects).

## Platform gaps

`_SCREENIMAGE` is a blank image on Linux/macOS; `_SCREENCLICK` and
`_SCREENPRINT` do nothing on Linux; printing (`LPRINT`, `_PRINTIMAGE`) goes to
the default printer on Windows and through `lp` on Linux and macOS.

## Since the QIR cutover

- Programs are compiled to QIR and run by the Rust runner (see
  [commands](commands.md#how-a-program-is-built)); no C++ is generated, so
  `-z`, `-p`, `-s:debuginfo` and `--ir` do nothing. `DECLARE LIBRARY`
  functions are called in shared libraries through `crates/engine/ffi` (see
  [language](language.md)).
- A statement the QIR back end does not cover yet stops the build with
  `QIR: unsupported: line N: ...`.
- BYREF parameters are copied in and out: different from QB64 only when a
  SUB changes the same variable both through a parameter and through SHARED.
- GPU 3D clips at the far plane (as QB64's OpenGL did); the software
  renderer does not.
- Web build limits: see [web](web.md).
