# Changelog

All notable changes to QBNex will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0] - 2026-11-13

QBNex 2.0 has no C or C++ left: programs are compiled to QIR and run by a Rust
runtime. Old QBasic/QB64 programs compile and behave as before, except for the
fixes listed in `docs/manual/en/differences.md`. This entry collects every change
made after 1.0.4.

### Changed

#### Compiler and runtime

- The compiler emits QIR (the QBNex intermediate representation) instead of C++. A compiled program is the Rust runner `qb-exec` with the program's QIR appended: QIR is lowered to bytecode, run on a register VM, and hot loops are compiled to native code with Cranelift.
- The whole runtime (screen, graphics, fonts, images, sound, files, network, `_MEM`, `_FLOAT`, `DECLARE LIBRARY` calls) is Rust (`crates/runtime/core`, `qb-audio`, `qb-float80`, `qb-ffi`, `qb-gpu`); no OpenGL, FreeGLUT, FreeType, miniaudio or zlib is linked. Behaviour was checked against the old runtime with conformance and differential tests.
- The compiler no longer builds any C++ text. Messages that named the C++ compiler or a C type now describe the QIR build: "Cannot find C type to return array data" is now "Unsupported array element type". `$EXEICON`, `$VERSIONINFO` and `$CHECKING:OFF` are accepted and have no effect (see the limitations page).
- Native executables (preview): with the native kit next to the runner, the compiler links ordinary executables through `rust-lld` and the `qb-native` library, still without a C/C++ toolchain.
- `-z`, `-p` and `--ir` are accepted for old scripts and change nothing. `--emit-ir` (or an `-o` name ending in `.qir`) writes the program's QIR text, for `qb-web`.
- `-s` shows and sets `exewithsource` (save the executable next to the source) and `debuginfo` (kept for old scripts, no effect). Settings live in `target/qb-config.ini`.

#### Console and build screen

- The compiler's build screen: a shaded logo shading from periwinkle through sky blue and mint to light green, icons, the source file, a progress bar redrawn in place with a carriage return, the line count and the build time. Help, version, info, examples, warnings and diagnostics use the same colors. It no longer runs together on one line or writes over the shell prompt. `-m` or the `NO_COLOR` environment variable keeps plain ASCII output and turns the diagnostics' colors off too.
- Console programs on Linux and macOS: `COLOR` shows the QBasic colors (1 is blue, 4 is red; before, the terminal's ANSI order swapped them), and `CSRLIN`/`POS` read the terminal's cursor. LOCATE, COLOR and CLS go through the `crossterm` library. On a Windows console, `COLOR` 16 to 255 now shows the 256-color palette (as a VT sequence) instead of a wrong 16-color attribute. On every system, text PRINTed before `LOCATE`, `COLOR`, `CLS`, `CSRLIN` or `POS` now reaches the console first.

#### Performance

- The compiler is about 25-30% faster with the same QIR output: a 10,000-line program compiles in 14.7 s instead of 20.2 s, a 5,000-line one in 7.3 s instead of 10.5 s, a 10,500-line program with 300 SUBs in 24.0 s instead of 32.3 s, and the compiler itself in 41 s instead of 51 s. An assignment's right side is fixed once instead of twice; the element, operator and symbol lookups take shorter paths (scalar caches, a hash that spreads names over the whole table, an index for the used-variable list); `qb-exec` runs the pure string builtins (UCASE$, LTRIM$, STR$, VAL, MKL$, ...) without the runtime's name dispatch. `QB_PROFILE` also reports inclusive time and exact call counts per function and per caller.
- Compiling large programs is about 10 times faster (10,000 lines: 20 s instead of 250 s): reading part of a long STRING variable (`LEN`, `ASC`, `MID$`, `LEFT$`, `RIGHT$`, `INSTR`, comparisons) no longer copies the whole string first.
- The JIT and `qb-exec build` compile a big program much faster: native code writes frame slots through to memory instead of writing every slot back at each exit (that made Cranelift time grow with slots x exits), already-complete blocks are sealed at once, and the functions are compiled to machine code on all CPU threads while the IR of the next ones is built. A 10,500-line program (305 functions) now compiles in 0.3 s instead of 13.8 s (1.5 s on one thread); its run time and a hot-loop benchmark are unchanged. `QB_JIT_THREADS` sets the number of compiler threads, `QB_JIT_STATS=1` also prints compile times.
- A large program no longer stalls at its first hot loop while the JIT compiles it: big modules are compiled on a background thread while the program keeps running (a 10,500-line program: 1.7 s instead of 16 s). The JIT also declares only the functions each function calls.

#### Build, repository and CI

- No MinGW, g++ or CMake: `scripts/setup_win.cmd`, `scripts/setup_lnx.sh` and `scripts/setup_osx.command` build `qb-exec` with cargo, pack the bootstrap compiler `bootstrap/qbnex.qir` into `qb-stage0`, and let it compile `compiler/qbnex.bas` into `qb`. `internal/` is gone. `scripts/selfhost_fixpoint.*` checks that the compiler rebuilds itself to the same QIR.
- Reorganized the repository: compiler sources moved from `source/` to `compiler/`; the runtime is a Cargo workspace in `crates/` (`compiler/`, `engine/`, `runtime/`, `tools/`, `web/`); tests live under `tests/` (`smoke/`, `fixtures/`, `conformance/`, `docs/`); setup scripts moved to `scripts/`, Docker files to `docker/`, the VS Code extension to `packaging/vscode/`.
- Rust 1.100.0 is the required toolchain (`rust-toolchain.toml`, `rust-version`). All crates share the workspace version (2.0.0).
- CI is one workflow (`ci.yml`: audit, build, clippy, tests, smoke, conformance, self-host fixpoint, native executables on Linux, macOS and Windows x64) plus `release.yml`, and `docs.yml` compiles the BASIC examples of the README and the manual; they run with read-only repository permissions.
- `release.yml` signs `qb-fmt`, `qb-lsp` and `qb-dap` when signing secrets are set (Windows: `signtool` with a `.pfx`; macOS: `codesign` and `notarytool`) and skips with a notice otherwise; the secrets are listed in CONTRIBUTING.md. The macOS and Linux CI build no longer fails on the conformance harness, the smoke tests keep the system `TMPDIR` (macOS sets it), and each smoke script runs under the shell its first line names.
- Release bundles include `qb-fmt`, `qb-lsp` and `qb-dap`, next to the one-file installers.
- CI also runs the GPU tests on a software adapter (Mesa lavapipe on Linux, WARP on Windows), a compiled program in a real X11 window under Xvfb (`tests/smoke/window_smoke.sh`: the drawing on screen, a key press), and the one-file installer on Linux, macOS and Windows (install over a folder with the user's files, compile and run a program with it, install again, uninstall: `.github/scripts/installer_check.*`).
- Developer documents are `docs/dev` (QIR format, deviation IDs, FFI, 3D API); the port journal and planning notes were removed.

#### License

- QBNex is now licensed under the GNU General Public License, version 3 or later, with the QBNex Compiled Program Exception (`LICENSE-EXCEPTION.txt`): programs compiled with QBNex may be licensed under any terms. The parts that come from QB64 keep their MIT license (`licenses/QB64.txt`). Releases made before this change remain available under the MIT license. Added `NOTICE.txt`, `TRADEMARKS.md` (use of the name and logo) and `CLA.md` (contributor agreement).

### Added

#### Language

- Language extensions: `ENUM`, `FOR EACH`, `TRY`/`CATCH`/`FINALLY`, `OPTIONAL` parameters, `$"..{expr}.."` interpolation, `_IIF`, `_TOSTR$`, `_ANDALSO`/`_ORELSE`, `_CONTINUE`, `$ASSERTS`/`_ASSERT` (see `docs/manual/en/language.md`).
- `DEF FN` functions, single-line (`DEF FnName(params) = expression`) and multi-line (`DEF FnName ... END DEF`, with `EXIT DEF` and STATIC variables).
- `RANDOMIZE` without a seed asks for one, as in QBasic.
- `VARPTR$` of a STRING variable, for `DRAW "X"` and `PLAY "X"`.
- `_ECHO` writes a line to the console.
- `ON KEY(n) SubName` and `ON TIMER(x) SubName`, also inside a SUB or FUNCTION.
- `ON STRIG(n) GOSUB` (and `ON STRIG(n) SubName`) with `STRIG(n) ON/OFF/STOP`: the handler runs when the joystick button is pressed.
- `ON PLAY(n) GOSUB` and `PLAY ON/OFF/STOP`: the handler runs when the background music queue drops below n notes.
- `CHAIN` passes the COMMON variables (numbers, STRINGs, fixed-length strings, TYPEs) and COMMON arrays to the next program by position: numeric, fixed-length string and TYPE arrays with their bounds and values, STRING arrays element by element. A dynamic array arrives with the passed bounds; a static array of the same size gets the values.
- `LPRINT`, `LPRINT USING`, `LPOS` and `WIDTH LPRINT`: pages of 60 lines go to the default printer (Windows), to `lp` (Linux, macOS) or, with `QBNEX_LPRINT=<file>`, to a text file. `_PRINTIMAGE` prints an image the same way.
- `CALL INTERRUPTX` and the `INTERRUPT[X] n, in, out` form; `_MEMFILL` of a variable without `AS type`; `_MEMGET` into a variable-length STRING; `_MEM` of a fixed-length STRING array.
- `_MEM(a$)` of a variable-length STRING (an extension; QB64 rejects it): a block over the string's current characters, one byte per element, valid until the string is assigned again. `_MEM` of a variable-length STRING array stays an error with a clearer message.
- `_MEMFILL` of a value that is not a variable, without `AS type`, uses the value's own type (its suffix, or the type of the expression).
- `TIME$ = "hh:mm:ss"` and `DATE$ = "mm-dd-yyyy"` (also `hh`, `hh:mm`, `mm/dd/yy`) set a clock of the program: TIME$, DATE$ and TIMER count on from the set time; the system clock is not changed. A badly formed value raises illegal function call, as in QBasic.

#### Graphics, window and sound

- 3D API (preview): `_LOADMODEL`, `_MESH*`, `_DRAW3D`, `_CAMERA`, `_LIGHT`, `_AMBIENT`, `_CLEAR3D`, `_GPU3D` and user WGSL shaders (`_SHADER`, `_SHADERPARAM`, `_MESHSHADER`), drawn with wgpu by a runtime built with the `gpu` feature; without a GPU the statements run and draw nothing. `_MAPTRIANGLE` 3D keeps its software renderer.
- `SUB _GL` and the `_gl*` functions run on a real OpenGL context (WGL, EGL/GLX, CGL; an offscreen one when headless); `QB_GL=off` turns it off (D-14).
- The QB64 keywords the QIR back end did not compile yet now compile and run: `_RESIZE`/`$RESIZE`, `_WINDOWHASFOCUS`, `_WINDOWHANDLE`, file drop, `_FPS`, `_ICON`, `_PRINTSTRING`, `_MEMIMAGE`, `_SCREENIMAGE`, `_SCREENCLICK`, `_SCREENPRINT`, `_MOUSEMOVE`, mouse pipes, `_DEVICES` and the device functions with `STICK` and `STRIG`, `_MAPUNICODE`, `LOCK`/`UNLOCK`, console input, cursor and font, `SHELL` with `_HIDE` and `_DONTWAIT`. QuickBASIC statements old programs still contain (`FRE`, `SETMEM`, `FILEATTR`, `IOCTL`, `PEN`, ...) compile and answer as a machine with plenty of memory and no light pen (see the limitations page).
- Variable-length `STRING` fields in `TYPE`.
- ALT+ENTER switches a window between windowed, stretched and square-pixel full screen; `_ALLOWFULLSCREEN` limits the modes or, with `_OFF`, leaves the keys to the program.
- A silent audio device: without an output device (or with `QB_AUDIO_DEVICE=null`) sound is mixed in real time and discarded, so `PLAY(n)`, `ON PLAY(n) GOSUB` and sound timing work as with speakers. `QB_AUDIO_DEVICE=none` still starts no audio at all.

#### Standard library, tools and installer

- `ext.*` standard library modules backed by Rust (`ext.regex`, `ext.hash`, `ext.zip`, `ext.http`, `ext.utf8`, `ext.collections`, `ext.datetime`; shared plumbing in `ext.core`).
- Tools: formatter `qb-fmt`, language server `qb-lsp`, debugger `qb-dap` (run as `qbnex fmt|lsp|dap`), a VS Code extension (`packaging/vscode`), and a WebAssembly build of the runner (`crates/web`).
- One-file installer `qbnex-setup` (crate `crates/tools/setup`): releases publish `qbnex_<tag>_<os>-setup(.exe)`, which installs per user without administrator rights (Windows: `%LOCALAPPDATA%\Programs\QBNex`, user PATH, Start menu, Apps & features entry; Linux/macOS: `~/.local/share/qbnex` with links in `~/.local/bin` and a "QBNex Terminal" menu entry) and uninstalls with `--uninstall`. A setup window leads through the license, the folder and the optional tasks (desktop shortcut, "Compile with QBNex" for `.bas` files), shows the progress and can be cancelled (a cancelled or failed install puts back what was there); without a display it asks in the terminal. `--yes`, `--dir`, `--no-path`, `--no-menu`, `--desktop`, `--associate`, `--console` (and `/S`, `/DIR=`) script it. It keeps the user's PATH entries and files.
- User manual in English and Thai (`docs/manual`), checked by `tests/docs/check_docs.ps1`.

### Fixed

- Integer `\` and `MOD` by zero raise a catchable error 11; runtime errors with redirected output go to stderr with exit code 1; and the other intentional changes listed in `docs/manual/en/differences.md`.
- DOUBLE and `_FLOAT` numbers print like QBasic 4.5 and QB64 in `PRINT`, `STR$`, `PRINT #` and `WRITE`: `d# = 2.5: PRINT d#` shows ` 2.5` instead of ` 2.50`, 42 shows ` 42` instead of ` 42.0` (at most 16 significant digits, no trailing zeros).
- Module code with labels placed after the SUBs (an `ON ERROR` handler, a `GOSUB` target) compiles and runs at module level; its variables are the main module's.
- Module code with labels after an `$INCLUDE` that brings the SUBs is moved in front of the first SUB of that file, so its labels work too.
- Module code moved in front of a first SUB that is in an `$INCLUDE` file now lands after what that file declares before the SUB, so it can use the file's `DIM SHARED` variables, `TYPE`s and `CONST`s. A runtime error in an included file names the line in that file (error message, `_INCLERRORLINE`, `_INCLERRORFILE$`), also in that moved code.
- Module code with labels after the SUBs inside an `$INCLUDE` file (also a nested one) is moved in front of the first SUB as well, so an `ON ERROR` or `GOSUB` label there compiles; a compile error in that code still names the included file and its line.
- `DATE$` and `TIME$` returned an empty string; they give the local date (`mm-dd-yyyy`) and time (`hh:mm:ss`) again.
- `TIMER` without an argument ticks 18.2 times a second (a SINGLE counted from local midnight), as in QBasic and QB64.
- `TIMER ON` after `TIMER STOP` delivers the event queued during the stop.
- `ON TIMER(x) SubName` compiled without error but never called the SUB.
- `LINE INPUT #` on a BINARY file reads a final line of 1 or 2 bytes.
- A failed `_MEMGET` check of 0 bytes reports the check error instead of error 518.
- `SUB _GL` without a visible window (`QBNEX_HEADLESS`) draws into an offscreen OpenGL framebuffer, and its frame (with the software screen per `_GLRENDER`) now reaches the window layer, so input-script snapshots show it. Input scripts get `pixel x y`.
- The window follows the size of the screen page: SCREEN 12 no longer loses its bottom 80 rows, and SCREEN 9 no longer shows a black band.
- A second `OPEN "COMn:..."` parses its options instead of failing with `Bad file name`.
- Native images (`qb-exec build`) of programs that use TYPEs or fixed-length strings in SUBs ran only by chance: the bytecode lowering allocated registers in hash-map order.
- An empty `$IMPORT` head file no longer stops the compiler with internal error 62.
- `CALL INTERRUPT` compiled to nothing: INT 33h (mouse) calls now answer as under `CALL ABSOLUTE`.
- `CHAIN "name"` without a directory runs the program in the current directory on Linux and macOS too.
- `CHAIN` inside a SUB or FUNCTION works when a COMMON variable or array is not SHARED (the build stopped with `QIR: unsupported`). COMMON variables and arrays no longer make the whole program run in the slower memory mode. `RUN file$` and `CHAIN` replace the program on Linux and macOS (exec); on Windows the program closes its files, stops its sound and hides its window before it waits, and ends with the other program's exit code.
- `REDIM _PRESERVE a(UBOUND(b))`, and any `DIM`/`REDIM` whose bounds read another array, resizes `a` again instead of `b`.
- Name lookups of `_OPENCLIENT` and friends give up after 10 seconds instead of blocking forever.
- Closing the window while the first `SOUND`, `PLAY` or other sound statement opens the audio device ends the program at once instead of after the device is up (0.4-1 s).
- A tight `SOUND` or `PLAY` loop no longer queues audio until memory runs out (3 GB in 5 s, then a crash): like the QBasic music buffer, a statement waits while more than 10 seconds of sound per voice is queued.
- `SOUND`, `BEEP` and `PLAY` follow `PLAY "MF"`/`"MB"`: in `MF` (the QBasic default) the statement returns once its notes have played; after `MB` it returns at once, with the 10-second queue limit. A window close during the wait ends the program at once.
- Found in a full audit of the runtime and tools. Memory safety: `LINE` on images above about 4200 pixels, `PRINT` on an image smaller than a character cell, tiled `_MAPTRIANGLE`, the image handle -2147483648, `WIDTH` with huge values and `LOCATE , , cursor` no longer write or read outside their buffers; a SUB that redimensions the SHARED array whose element it was passed keeps valid memory. Wrong results: a string comparison in a FUNCTION called from a compiled loop, a BYREF array element whose subscript the callee changes, arguments lost by a call that fails, `REDIM` with bounds wider than 64 bits, `LINE INPUT` on a BINARY file dropping the last byte, a failed `OPEN` keeping its file number, `OPEN ... LEN = 0` (now error 5), "Permission denied" (70) on Linux and macOS for a file that may not be opened. Aborts that are now run-time errors: `VARPTR`/`SADD` past the 64 KB segment, `ENVIRON` with a malformed name, `_LOADFONT` of a damaged font
- `'$IMPORT:'qbnex'` combines with `json`, `url` and `ext.*`, in any order, and with the modules it bundles (the later import used to be dropped). The JSON type constants are `JSON_TYPE_NULL` ... `JSON_TYPE_OBJECT`; the old names stay except `JSON_STRING`, which was the same name as `Json_String$`
- Standard library: `List_Set`, `List_RemoveAt` and with them `Queue_Dequeue$` and `Stack_Pop$` no longer rebuild the list (a 2000-item queue took minutes); JSON numbers near the DOUBLE limit; `Date_FromUnixMs` milliseconds; `Result_AddContext` with a context that is part of another one
- Tools: the debugger shows Thai output split across reads, and a second "continue" no longer cancels the next breakpoint; the formatter keeps the case of an unquoted `ALIAS` name and formats a `_` glued to a number or name once; `qb-fmt` and `qb-lsp` survive thousands of nested blocks and a wrong `Content-Length`; closed sounds are freed
- Installer: installing into a folder that already holds files replaces the files QBNex installs and leaves the rest; a failed or cancelled install puts every replaced file back. `qbnex fmt|lsp|dap` work from the installed link, and an uninstall finds its links when the folder is reached through a symlink
- Native images (`qb-exec build`) use the baseline instruction set of the architecture, not the extensions of the CPU they were built on
- Compiler, found in the same audit. `TRY`: the program's own `ON ERROR GOTO` handler is in force again after `END TRY`, an inner `TRY ... FINALLY` passes its error to the enclosing `TRY`, and a `TRY` left with `EXIT FOR`/`DO`/`WHILE`, `_CONTINUE` or `EXIT SUB` no longer keeps catching every later error. Line numbers after a `TRY` block or a `CLASS` are the source lines again. `DEF FN` works after a line number or a label, with another statement after `:`, with variables declared `DIM name AS type`, with fields of a `TYPE` variable, and under a `DEFINT` that changes later in the file. `CONST`: `-2 ^ 2` is -4, a negative constant keeps its sign under `^`, brackets nest more than two deep. An array used without `DIM` starts at 1 under `OPTION BASE 1`. `EXIT FOR` leaves a `FOR EACH` over several dimensions. A method inherited through `EXTENDS` can be called on the derived class, and `object.method()` is a statement without `CALL`. Module code after a `SUB` uses the module's variables. `END n` sets the exit code. `VARPTR` of a `TYPE` variable, a `TYPE` variable in brackets and a `CASE` with over 100 values no longer end in an internal compiler error
- `CALL ABSOLUTE` accepts prefixes in any order and 32-bit operands with a SIB byte; a timer freed in its own handler, or `RUN` inside an event handler, no longer stops the timer for good; `ON TIMER` with an interval of 0 or less is error 5
- Files and input: `FIELD` on a variable of a SUB ends when the SUB returns (a later `GET` wrote into freed memory); `FIELD`s over the same bytes show each other's `LSET`; `OPEN "R", #n, file$, reclen` uses the record length; `TAB` in `PRINT #` counts from the column of the file; `INPUT` and `INPUT #` read `_INTEGER64` values exactly; `READ` and `INPUT` wrap `_BIT` values; `PUT`/`GET` take a whole array; a fixed-length string starts as `CHR$(0)` bytes everywhere (it was spaces outside a `TYPE`); `PALETTE USING` works; a whole-number literal past 64 bits is a floating-point value
- Compiler command line: a relative `-o` path is checked in the folder the compiler was started from, `-o` without a name and a second source file are errors, `qb name` finds `name.bas`, and `--emit-ir -o x.exe` writes `x.qir` on every system
- `TRY`: a `GOTO` out of the body ends it. `DEF FN` works in `$INCLUDE` files. A `CONST` keeps the precision of a DOUBLE
- Sound and network: `PLAY "Nn"` notes and `N0` rests, `PLAY(n)` counts notes, `SOUND f, 0` stops the sound, `PLAY "X" + VARPTR$(s$)`, `_SNDOPEN` with "VOL"/"SYNC" keeps `_MEMSOUND`; on TCP handles `PUT`/`GET` of a whole array and `EOF(handle)` work; the HTTP client repeats a POST/PUT on a 307/308 redirect and keeps header bytes that are not UTF-8
- Windows: native executables are linked again when the native kit is present (cmd.exe took the quotes off the link command), `CHAIN`/`RUN` of a path with `(`, `)` or `&`, `COM10:` and higher, `KILL "D:name"`, `DECLARE DYNAMIC LIBRARY` of a DLL in the current folder, `qb fmt` with `&` in a quoted file name, uninstalling from inside the install folder; `CHAIN "..\next.bas"` and other paths with a `.` in a folder name find the program on every system; "Compile with QBNex" in the file manager writes the program next to the source
- Images and 3D: `PRINT` that scrolls an image lower than one character row no longer writes past its pixels; a PNG with a palette that is not whole entries (which aborted the program) or with a tRNS chunk stb rejected returns -1; the 3D form of `_MAPTRIANGLE` (`TO (x, y, z)-...`) projects with perspective, uses the depth buffer and `_CLOCKWISE`/`_ANTICLOCKWISE` (it drew a 2D triangle at x, y), and `_DEPTHBUFFER ON|OFF|LOCK|_CLEAR` works; `_COPYIMAGE(img&, 33)` of a 32-bit image; `_MESHVERTEX` rejects NaN and infinity
- Web build: output and input are UTF-8 (Thai text), and a byte outside UTF-8 is its code page 437 character
- `CLOSE` without a file number, and `RUN`, `CHAIN` and `CLEAR`, close the TCP/IP handles too
- Linux: a program that opens a window, and the setup window, no longer abort when libxkbcommon-x11 (or libxkbcommon) is not installed; the program runs without a window and says which library is missing, setup asks its questions in the terminal

### Security

- `_CONNECTIONADDRESS` of a host asks api.ipify.org over HTTPS instead of plain HTTP.
- CI actions are pinned to commit SHAs.

### Removed

- The C++ back end and runtime: `internal/` (libqb, the C++ build scripts, MinGW downloads), the CMake build and the `-z` C++ output.
- One-shot migration scripts (`scripts/reorg_crates*.ps1`), the legacy C++ oracle helpers (`tests/conformance/oracle`), the corpus harness that needed the removed C++ backend (`tests/corpus-run`) and the Inno Setup script (`packaging/installer`), replaced by `qbnex-setup`.

## [1.0.4] - 2026-05-05

### Added

#### Input Runtime
- Added Unicode key event handling for non-ASCII Windows `WM_CHAR` input so extended characters are routed through the QBNex keyboard path instead of being truncated to single-byte callbacks.

### Changed

#### Build System
- Enabled CMake self-hosting by default so the standard build target produces the final `qb`/`qb.exe` compiler.
- Added an explicit stage0 source snapshot step that refreshes `internal/temp` from `internal/source` before building `qb-stage0`.

#### Compiler Progress
- Tracked the last rendered compiler progress percentage to avoid redundant progress redraws.
- Moved final progress completion to the successful native-build preparation path so progress output no longer finishes before late diagnostics and build setup run.
- Added progress cancellation for frontend error paths so diagnostics start on a clean line instead of being mixed with an active progress bar.

### Fixed

#### Compiler Shutdown
- Closed open file handles before compiler process exit paths, reducing stale locks after success or failure.

#### Test Cleanup
- Updated CLI smoke tests to remove the default `-z` output binary so test runs no longer leave `label_recompile_success` executables in the repository root.

## [1.0.3] - 2026-04-24

### Changed

#### Build System
- Refined the CMake build flow and documentation so native builds are easier to follow across supported platforms.
- Removed the generated `internal/c/qbx.cpp` source from the tracked build inputs so CMake uses regenerated compiler output instead of stale checked-in C++.

#### Compiler Progress
- Improved compiler progress rendering so the banner shows progress immediately after startup.
- Updated progress completion to redraw the final 100% line, clear stale characters from shorter updates, and leave the console cursor on a clean line.

## [1.0.2] - 2026-04-23

### Added

#### Smart Linking
- Added an explicit GUI-core dependency flag so text-console programs can stay console-only while graphics programs still link the window/OpenGL runtime.
- Expanded smart-link smoke coverage to verify `PRINT`, `BEEP`, `CLS`, `LOCATE`, and `COLOR` do not pull in the audio or OpenGL runtime.

#### Audio Synthesis
- Added 4-voice polyphonic synthesis for `SOUND` and `PLAY`, with `_VOICE` for fixed or automatic voice selection.
- Added `_ADSR` envelope controls for attack, decay, sustain, and release shaping.
- Added `_WAVE` waveform selection with built-in sine, square, saw, triangle, white noise, pink noise, brown noise, LFSR noise, and custom sample-table waveforms.
- Added audio synth smoke fixtures covering `_VOICE`, `_ADSR`, `_WAVE`, `SOUND`, and `PLAY`.

### Fixed

#### Runtime Memory Cleanup
- Released retained text-cell and image/palette vector capacity when images switch ownership or are freed.
- Cleared stale page references when image slots are reclaimed to reduce long-running screen-buffer leaks and dangling state.

## [1.0.1] - 2026-04-22

### Changed

#### Compiler Performance
- Reduced repeated linker symbol scans by caching `nm` output during native build preparation.
- Reduced repeated build-script reads by caching Windows and Unix build command templates.
- Added token access caching in the parser hot path to cut repeated sequential string scans.
- Added Windows precompiled-header generation for `common.h` so repeated compiler runs can reuse a dependency-specific PCH.

#### Compiler Structure
- Split static link symbol registration and cached build-line helpers out of `qbnex.bas` into dedicated utility modules.
- Simplified the monolithic compiler flow by routing static symbol registration through shared helpers instead of duplicating dynamic array growth logic inline.

#### Validation
- Added a larger benchmark fixture and extended benchmark smoke coverage to exercise small, medium, and large compile paths.
- Verified the release build against CLI, diagnostics, warnings, encoding, labels, stdlib, and benchmark smoke suites on Windows.

## [1.0.0] - 2026-04-21

### Initial Release

#### Release Engineering
- Added GitHub Actions release packaging for Linux, macOS, Windows x86, and Windows x64 artifacts.
- Added tag-driven GitHub Release publishing for tags matching `v*`, with generated release notes and packaged archives.
- Updated push and pull-request CI workflows to build cross-platform artifacts instead of Linux-only builds.
- Removed smoke and benchmark suites from the default GitHub-hosted build workflows; these suites remain available for manual verification.

#### Compatibility Fixes
- Fixed the macOS clang build in `sub_environ` by using a mutable `std::string` buffer pointer compatible with Apple toolchains.

#### Documentation
- Updated `README.md` to reflect the current release assets, CI matrix, tag-based release flow, and the distinction between local smoke tests and default hosted CI behavior.

#### Core Features
- Complete QBasic/QB4.5 compatible compiler
- Self-hosting compiler written in QBNex BASIC (~26,000 lines)
- Transpiles BASIC source to C++ and compiles to native binaries
- Command-line interface with comprehensive flag support
- Support for 150+ QBasic/QB64 keywords and functions
- TCP/IP networking support with `_OPENHOST`, `_OPENCLIENT`, and connection management
- OpenGL extension support with `_GLRENDER` and `_DISPLAYORDER` metacommands
- TrueType font rendering via FreeType library
- Image format loading support (BMP, PCX, PNG, JPEG, etc.) via STB Image
- Optional ZLIB compression support
- Game controller/gamepad input support
- Compiler settings management via INI configuration (`internal/config.ini`)
- `-z` flag to generate C code without compiling to executable
- `-s` flag to view/edit compiler settings
- `-p` flag to purge all pre-compiled content
- `-e` flag to enable `OPTION _EXPLICIT` for compilation
- Auto-build message support via `internal/version.txt` (Git integration)
- Debug mode with GDB debugging information (`DebugInfo` setting)
- `IgnoreWarnings` and `SaveExeWithSource` compiler settings
- Network socket support with conditional compilation (`DEPENDENCY_SOCKETS`)
- Audio output via miniaudio library with cross-platform support
- Support for unsigned integer types (`UNSIGNED BYTE`, `UNSIGNED INTEGER`, `UNSIGNED LONG`, `UNSIGNED _INTEGER64`)
- Pointer types (`OFFSET` / `UNSIGNED OFFSET`)
- 64-bit integer support (`_INTEGER64`)
- Extended precision floating-point (`_FLOAT`)

#### Standard Library
- **Complete stdlib implementation** with 18 modules covering collections, strings, I/O, system utilities, math, error handling, and OOP
- **Collections Module** (`collections.*`):
  - `list.bas` - Dynamic array with automatic resizing (List_Init, List_Add, List_Get, List_Remove, List_Count, List_Join, List_Free)
  - `stack.bas` - LIFO stack implementation (Stack_Init, Stack_Push, Stack_Pop, Stack_Peek, Stack_Count, Stack_Free)
  - `queue.bas` - FIFO queue implementation (Queue_Init, Queue_Enqueue, Queue_Dequeue, Queue_Peek, Queue_Count, Queue_Free)
  - `set.bas` - Hash-based set with unique values (HashSet_Init, HashSet_Add, HashSet_Contains, HashSet_Remove, HashSet_Count, HashSet_ToString, HashSet_Free)
  - `dictionary.bas` - Key-value store (Dict_Init, Dict_Set, Dict_Get, Dict_Remove, Dict_Count, Dict_HasKey, Dict_Free)
- **String Utilities** (`strings.*`):
  - `text.bas` - String manipulation (Text_PadLeft, Text_PadRight, Text_Repeat, Text_StartsWith, Text_EndsWith, Text_Contains)
  - `strbuilder.bas` - Efficient string concatenation (SB_Init, SB_Append, SB_AppendLine, SB_ToString, SB_Free)
- **I/O Utilities** (`io.*`):
  - `path.bas` - Cross-platform path manipulation (Path_Join, Path_FileName, Path_DirName, Path_Extension, Path_WithoutExtension, Path_Normalize)
  - `csv.bas` - CSV generation (CSV_Row3, CSV_Escape)
  - `json.bas` - JSON object creation and parsing (Json_Object3, Json_String, Json_Number, Json_Array, json_parse, json_get_str)
- **Network & Web Utilities** (`net.*` / Built-in):
  - `http.bas` - HTTP client implementation (get, post, put, delete, fetch)
  - `url.bas` - URL encoding, decoding, and parsing (encode, decode, url_parse)
  - Web Server - Built-in functionality for HTTP servers (server, route_get, route_post, listen)
- **System Utilities** (`sys.*`):
  - `env.bas` - Platform detection and environment variables (Env_Platform, Env_Is64Bit, Env_GetHome, Env_Get)
  - `args.bas` - Command-line argument access (Args_Count, Args_Get)
  - `datetime.bas` - Date/time utilities (Date_SetNow, Date_ToISOString, Date_GetFullYear, Date_GetMonth, Date_GetDay, Date_NowMs, Date_IsLeapYear)
- **Math Utilities** (`math.*`):
  - `numeric.bas` - Mathematical helpers (Math_Clamp, Math_Min, Math_Max)
- **Error Handling** (`error.*`):
  - `result.bas` - Result pattern for error handling (Result_Ok, Result_Fail, Result_FailCode, Result_IsOk, Result_Value, Result_Message, Result_ErrorChain)
- **OOP Support** (`oop.*`):
  - `class.bas` - Class registry and inheritance (QBNEX_RegisterClass, QBNEX_FindClass, QBNEX_RegisterMethod, QBNEX_FindMethodSlot, QBNEX_IsInstance, QBNEX_ObjectInit)
  - `interface.bas` - Interface implementation (QBNEX_RegisterInterface, QBNEX_FindInterface, QBNEX_Implements)
- **Core Library**:
  - `qbnex_stdlib.bas` - Unified stdlib combining all modules (18 modules in one file)
- **Modern Import System**:
  - Clean Python-style module imports (`IMPORT module.name`) alongside traditional syntax (`'$IMPORT:'module.name'`)
- **Example Programs** (9 comprehensive examples):
  - `stdlib_demo.bas` - Full stdlib demonstration
  - `class_syntax_demo.bas` - Native CLASS syntax examples
  - `import_smoke.bas` - Import system verification
  - `runtime_smoke.bas` - Runtime functionality tests
  - `data_smoke.bas` - Data structure tests
  - `ecosystem_smoke.bas` - Integration tests
  - `top_level_runtime_regression.bas` - Runtime regression tests
  - `method_chain_regression.bas` - Method chaining tests
  - `top_level_qbnex_runtime_min.bas` - Minimal runtime tests

#### Language Support
- **Control Flow**: IF/THEN/ELSE, SELECT CASE, FOR/NEXT, DO/LOOP, WHILE/WEND, GOTO, GOSUB
- **Data Types**: BIT, BYTE, INTEGER, LONG, _INTEGER64, SINGLE, DOUBLE, _FLOAT, STRING, OFFSET
- **Variables & Arrays**: DIM, REDIM, REDIM PRESERVE, multi-dimensional arrays, OPTION BASE
- **User-Defined Types**: TYPE...END TYPE with nested structures
- **Subroutines & Functions**: SUB, FUNCTION, DECLARE with parameter support
- **String Operations**: LEFT$, RIGHT$, MID$, INSTR, LCASE$, UCASE$, LTRIM$, RTRIM$, TRIM$, etc.
- **Math Functions**: ABS, SGN, SIN, COS, TAN, ATN, EXP, LOG, SQR, INT, FIX, RND, etc.
- **Type Conversion**: CINT, CLNG, CSNG, CDBL, CSTR, MKI$, MKL$, CVS, CVD, etc.
- **Modern Extended Syntax**:
  - `IMPORT` statement for modular ecosystem (`IMPORT module_name`)
  - Augmented assignment operators (`+=`, `-=`, `*=`, `/=`)
  - Alternative comment syntax (`# comment`)
  - Modern function type definitions (`FUNCTION name AS STRING` instead of `FUNCTION name$`)
  - Shorter function declarations (`FUNC name()`)
  - Lambda-like single-line functions (`DEF name(x) = x*2`)

#### Graphics Capabilities
- OpenGL-based graphics subsystem with FreeGLUT
- SCREEN modes (0-12+) with VGA and hi-res graphics
- Drawing primitives: PSET, PRESET, LINE, CIRCLE, PAINT, DRAW
- Image manipulation: GET/PUT with transfer modes (PSET, AND, OR, XOR)
- Viewport and window management: VIEW, WINDOW, PMAP
- Color and palette control: COLOR, PALETTE, POINT
- Text positioning: CLS, LOCATE, WIDTH
- TrueType font support via FreeType

#### Sound Capabilities
- BEEP, SOUND, PLAY commands
- Cross-platform audio via miniaudio (Windows, ALSA, CoreAudio)
- Music string syntax with tempo and octave control
- Tone generation with frequency and duration control

#### File I/O
- Sequential file access: OPEN, PRINT #, WRITE #, INPUT #, LINE INPUT #
- Random file access: OPEN FOR RANDOM, GET, PUT, FIELD, LSET, RSET
- Binary file access: OPEN FOR BINARY, GET, PUT
- File management: FREEFILE, EOF, LOF, LOC, SEEK
- Directory operations: KILL, NAME...AS, FILES, CHDIR, MKDIR, RMDIR
- Binary load/save: BLOAD, BSAVE

#### System Features
- Timer and date/time functions: TIMER, DATE$, TIME$
- Command-line argument access: COMMAND$
- Environment variables: ENVIRON$
- Memory operations: PEEK, POKE, DEF SEG, VARPTR, VARSEG, SADD
- Process control: SHELL, CHAIN, CALL, CALL ABSOLUTE
- Error handling: ON ERROR GOTO, RESUME, ERR, ERL

#### Build System
- Automated setup scripts for Windows (`setup_win.cmd`), Linux (`setup_lnx.sh`), and macOS (`setup_osx.command`)
- CI/CD pipelines for all platforms via GitHub Actions
- Cross-compilation support for Windows x86 and x64
- Dependency management with conditional compilation
- Build utilities and file management tools
- Refactored from QB64 to CLI-driven compiler without legacy IDE components
- Modernized build system with automated dependency installation
- Improved cross-platform compatibility (Windows, Linux, macOS)
- Enhanced graphics subsystem with OpenGL and FreeGLUT
- Updated sound system to use miniaudio instead of legacy APIs

#### Platform Support
- **Windows**: Windows 7+ (32-bit and 64-bit), MinGW g++ compiler
- **Linux**: OpenGL, ALSA, FreeGLUT, X11, g++ compiler
- **macOS**: Xcode command line tools, clang++, OpenGL, CoreAudio
- **Docker**: Ubuntu 22.04 base container with all dependencies pre-installed

#### Developer Tools
- Compiler version tracking (`-v` flag)
- Help and documentation (`-h`, `--help`)
- Examples display (`-g` flag)
- Warning system with `-w` flag
- Quiet mode (`-q` flag)
- Monochrome output option (`-m` flag)
- Settings management (`-s` flag)
- Pre-compiled content purge (`-p` flag)
- C code generation without compilation (`-z` flag)
- OPTION _EXPLICIT enforcement (`-e` flag)
- Compile and run immediately (`-x` flag)
- Custom output naming (`-o` flag)

#### Error Handling
- Expanded `QBNex_Result` in `error.result` and `qbnex_stdlib.bas` from a minimal success/message container into a structured result type with `Code`, `Context`, `Source`, and `Cause` metadata.
- Added richer result helpers for contextual propagation and inspection: `Result_FailCode`, `Result_FailWithContext`, `Result_AddContext`, `Result_SetSource`, `Result_SetCause`, `Result_Propagate`, `Result_IsError`, `Result_Code`, `Result_Context`, `Result_Source`, `Result_Cause`, `Result_ErrorChain`, `Result_Describe`, and `Result_Expect`.
- Upgraded compiler diagnostics in `source/utils/error_handler.bas` with phase-aware reporting, context-flow tracing, duplicate suppression, and blocking-diagnostic summaries.
- Reworked diagnostic presentation into a QBNex-specific format using markers such as `[x]`, `[!!]`, `[@]`, `[#]`, `[>]`, `[::]`, and `[=]` instead of generic compiler-style notes.
- Added compact, modern diagnostic summaries such as `QBNex :: Build Halted` and `QBNex :: Build Complete` to better distinguish build outcome from individual errors.

#### Testing & Quality Assurance
- **Comprehensive test suite** (`test_all.bas`) with 17 test categories
- **Core language tests** (15 tests): Runtime paths, variables, math, strings, control flow, loops, arrays, types, functions, date/time, file I/O, SELECT CASE, type conversion, logical operations
- **Standard library compilation tests** (18 modules): All stdlib modules compile successfully
- **Example program tests** (9 programs): All example programs compile and run correctly
- **Test automation**: `run_tests.cmd` script for Windows
- **Test documentation**: `TEST_README.md` with comprehensive testing guide
- **Test results**: `TEST_RESULTS.md` with detailed test execution report
- **100% test success rate**: All 17 test categories pass
- **Zero compilation errors**: Clean compilation across all modules
- **Zero warnings**: No compiler warnings in stdlib or test suite

#### Docker Support
- Multi-stage Dockerfile for production builds (~300MB)
- Development Dockerfile with build cache support (~500MB)
- Docker Compose configuration for easy deployment
- Comprehensive `.dockerignore` for optimized build context
- Volume mounting for seamless development workflow
- Graphics support via X11 forwarding (Linux, macOS, Windows)
- Network mode support for TCP/IP applications
- Interactive development mode with bash shell
- Cross-platform compilation without local dependencies
- CI/CD ready Docker configuration
- Merged Docker documentation into main README.md

#### Documentation
- Comprehensive README.md with code examples
- Supported commands reference by category
- Installation guides for all platforms
- Usage examples and common patterns
- Docker usage guide integrated into main README
- Complete Docker documentation with troubleshooting
- Updated Table of Contents with Docker sections
- Standard library reference with function signatures
- Testing documentation with usage examples
- CHANGELOG.md with detailed version history

#### Bug Fixes
- **Fixed missing DIM declarations** for loop variables in stdlib modules (path.bas, file.bas, opengl_methods.bas)
- **Fixed missing return values** in FUNCTION definitions (Date_IsLeapYear, Date_PartValue, Text_Repeat, Text_StartsWith, Text_EndsWith, Text_Contains, Stack_Peek, Stack_Pop, Queue_Peek, Queue_Dequeue)
- **Fixed unreachable code warnings** by converting inline EXIT FUNCTION to multi-line format
- **Fixed GOTO label case sensitivity** issues (errorCleanup → ErrorCleanup in file.bas)
- **Replaced SYSTEM 1 calls** with graceful error handling in stdlib (List_Init, QBNEX_RegisterClass, QBNEX_RegisterInterfaceName, QBNEX_RegisterMethod, QBNEX_RegisterInterface)
- **Fixed missing TYPE declarations** in interface.bas (added QBNex_ClassInfo and related shared variables)
- **Fixed OOP function return values** (QBNEX_FindClass, QBNEX_FindMethodSlot, QBNEX_ClassName, QBNEX_IsInstance, QBNEX_FindInterface, QBNEX_Implements)
- **Cross-platform compilation issues** on Windows x86, x64, Linux, and macOS
- **Graphics rendering consistency** across different platforms
- **Audio output stability** and performance
- **File I/O operations** for sequential, random, and binary access modes

#### Security
- Added input validation for file operations to prevent buffer overflows
- Improved memory safety in pointer operations
- Enhanced network socket security with proper connection handling
- Graceful error handling instead of abrupt program termination

---

## Notes

### Versioning Scheme
- **Major**: Significant breaking changes or architectural shifts
- **Minor**: New features, backward-compatible enhancements
- **Patch**: Bug fixes, performance improvements, documentation updates

### Where the version lives (2.0)
- The compiler version is `Version$` in `compiler/global/version.bas` (`qb --version`); a build may add a line from `target/qb-version.txt`.
- All Rust crates share `version` in the workspace `Cargo.toml`.

### Compiler settings (2.0)
`qb -s` reads and writes `target/qb-config.ini`:
- `exewithsource`: write the executable next to the source instead of the QBNex folder
- `debuginfo`: accepted for old scripts; it has no effect
- `IgnoreWarnings` (set in the INI file only): hide warnings

1.x kept these in `internal/config.ini`, where `DebugInfo` added GDB symbols to the C++ build.

### Dependencies (2.0)
- Building QBNex needs the Rust toolchain (`cargo`); compiled programs need nothing else.
- Linux builds need `gcc` (as the system linker only), ALSA, udev, xkbcommon, X11 and `pkg-config` development packages.
- Window, graphics, sound, fonts, images and compression are Rust crates (see `licenses/THIRD_PARTY.TXT`).

1.x needed a C++ toolchain (MinGW, g++ or clang++) with OpenGL, FreeGLUT, GLEW, FreeType, ZLIB and the platform audio libraries.

---

[2.0.0]: https://github.com/thirawat27/QBNex/releases/tag/v2.0.0
[1.0.4]: https://github.com/thirawat27/QBNex/releases/tag/v1.0.4
[1.0.3]: https://github.com/thirawat27/QBNex/releases/tag/v1.0.3
[1.0.2]: https://github.com/thirawat27/QBNex/releases/tag/v1.0.2
[1.0.1]: https://github.com/thirawat27/QBNex/releases/tag/v1.0.1
[1.0.0]: https://github.com/thirawat27/QBNex/releases/tag/v1.0.0
