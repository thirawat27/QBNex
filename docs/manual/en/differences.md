# Differences from QB64 and QBasic

[Manual index](../README.md) · [ภาษาไทย](../th/differences.md)

QBNex started from QB64. Old programs should compile and give the same
output; this page lists where they do not, and why. Each change is
explained, with its tests, in
[`docs/dev/deviations.md`](../../dev/deviations.md).

## The tool

- **No IDE.** QBNex is a command-line compiler (`qb`, `qbnex`). Use any
  editor; VS Code has an [extension](vscode.md). `$DEBUG` and the `-l`
  switch of the QB64 IDE are not supported.
- **Standard library and `'$IMPORT`** are QBNex additions
  ([stdlib](stdlib.md)). Imported modules are hoisted: their declarations go
  before the program and their procedures after it.
- **Source encoding:** CP437 files compile byte for byte, UTF-8 too.
- **The runtime is Rust.** No OpenGL, FreeGLUT, FreeType, miniaudio or zlib
  is linked; behaviour was checked against the old runtime with differential
  tests (billions of compared results, 0 mismatches outside the items below).

## Intentional changes (bugs fixed on purpose)

- `VAL("123E-2")` is `1.23` (QB64 gave `.000123`: the exponent sign was applied after normalising).
- Rounding to integers (`CINT`, `CLNG`, assignment to integer variables) is round-half-even on every CPU (QB64 on ARM rounded half away from zero).
- `VAL` reads 1000 significant digits exactly (QB64 overran a buffer past 256).
- `STR$` of doubles needing more than 19 decimals prints the digits (QB64 printed ` .0f`); `STR$` of a non-finite SINGLE is ` 0`.
- DOUBLE and `_FLOAT` values print like QBasic 4.5 and QB64 (`PRINT`, `STR$`, `PRINT #`, `WRITE`): at most 16 significant digits, no trailing zeros. `PRINT 2.5#` shows ` 2.5`.
- Main-module code before the first SUB stays in the main module; `ON ERROR GOTO` handlers in the main module work.
- `_MEMFREE` of a `_MEMNEW` block no longer frees twice (heap corruption in QB64).
- Reads past buffers in QB64 (GET of strings, FILES, SHELL, stretched `_PUTIMAGE`, 2-bit GET, HEX$) now give defined results.
- `SHELL` keeps window events paused for the whole waited call.
- The millisecond clock is 64-bit everywhere (QB64 on Windows wrapped after 24.8 days).
- `_CLIPBOARD$` and `_CLIPBOARDIMAGE` work on Linux and macOS too (UTF-8 clipboard: non-UTF-8 bytes become U+FFFD there).
- `_OPENCLIENT` tries every IPv4 address of a host on every OS.
- CP437 source files compile.
- Compiler fixes: `CALL ABSOLUTE` wrappers, `PLAY ON/OFF/STOP` and `ON PLAY(n) GOSUB` compile and the PLAY event is raised, relational operators inside PRINT.
- `PLAY "Nn"` plays the QBasic note (N0 is a rest), `PLAY(n)` counts notes, `SOUND f, 0` stops the sound, `PLAY "X" + VARPTR$(s$)` works, and `_SNDOPEN` with "VOL" or "SYNC" keeps `_MEMSOUND` working.
- `SOUND`, `BEEP` and `PLAY` are heard and `PLAY(n)` counts down (QB64's synth was silent); in `MF` mode (the default) they wait until their notes have played, as in QBasic (QB64 never waited).
- Integer `\` and `MOD` by zero raise error 11 as an ordinary error that `ON ERROR` and `TRY`/`CATCH` catch (QBasic 4.5); QB64 ended the program.
- READ of decimal numbers into DOUBLE/`_FLOAT` is correctly rounded.
- With redirected output or `QBNEX_NO_ERROR_DIALOG=1`, a runtime error prints one line to stderr and exits with code 1 instead of opening a dialog.
- `PUT` to a TCP connection whose send buffer is full keeps the connection: the rest of the data is queued and sent, in order, by later network calls and `CLOSE` (QB64 dropped the connection and the data).

## QBasic behaviour kept on purpose

These match QB64 (golden outputs depend on them): `MKSMBF$(0)` gives MBF
exponent 2; `STR$` of
infinity/NaN is ` 0`; graphics keep QB64's float-to-integer conversions;
`_CONNECTIONADDRESS` shows a byte-swapped remote port; TIMER waits for the
next second at program start.

## Known defects

No open defects are known; see
[Limitations and differences from QB64](limitations.md#known-defects).

## Platform gaps

`_SCREENIMAGE`, `_SCREENCLICK` and `_SCREENPRINT` compile, but
`_SCREENIMAGE` is a blank image on Linux/macOS and `_SCREENCLICK` and
`_SCREENPRINT` do nothing on Linux. Printing (`LPRINT`, `_PRINTIMAGE`) goes to
the default printer on Windows and through `lp` on Linux and macOS.

- Linux: OPEN … LOCK READ and LOCK WRITE refuse every other OPEN of the file,
  not only the conflicting access (stricter than Windows). Locks are advisory:
  only programs that lock the file (QBNex programs, `flock`/`fcntl` users) see
  them. An OPEN without a LOCK clause takes a shared lock.
- macOS: the LOCK clause of OPEN is not enforced. LOCK ranges are per process,
  so closing another file number of the same file releases them.

## Compiler and runtime

- Programs are compiled to QIR and run by the Rust runner (see
  [commands](commands.md#how-a-program-is-built)).
  `-x` and `-z` change nothing (console output is always on); `-p`,
  `-s:debuginfo` and `--ir` are accepted and do nothing. `DECLARE LIBRARY`
  functions are called in shared libraries through `crates/engine/ffi` (see
  [language](language.md)); where the library is looked for is in
  [limitations](limitations.md#declare-library).
- A statement the QIR back end does not cover yet stops the build with
  `QIR: unsupported: line N: ...`.
- A relative `$INCLUDE` path is looked for next to the including file, next to
  the main source file, then in the folder `qb` was started in; `'$INCLUDEONCE`
  (QB64-PE) in a file includes it once only. `$INCLUDE` names are resolved
  case-insensitively on Linux/macOS when the
  exact path does not exist (folder by folder; an exact match wins). `$IF` does
  not interact with block structure (QB64 popped the open block at `$END IF`).
- INPUT with redirected stdin: an answer that does not fit prints "Redo from
  start" and reads the next line. When no input is left, the answer is taken as
  given (missing values 0 / "") instead of waiting for ever.
- `_SNDLIMIT` counts from the sound's start (QB64's implementation used
  absolute engine time).
- `TIME$ =` and `DATE$ =` move a clock of the program (TIME$, DATE$, TIMER),
  not the system clock; QB64 ignored them. `_MEM` of a variable-length STRING
  and `_MEMFILL` of a non-variable value without `AS` are accepted (see
  [limitations](limitations.md)).
- 3D `_MAPTRIANGLE` has no far plane (see [graphics](graphics-3d.md)).
- Web build limits: see [web](web.md).
