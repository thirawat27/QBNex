# Changelog

All notable changes to QBNex will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0] - 2026-11-13

In QBNex 2.0, programs are compiled to QIR and run by a Rust runtime. Old
QBasic/QB64 programs compile and behave as before, except for the fixes listed
in `docs/manual/en/differences.md`. This entry collects every change made after
1.0.4.

The standard library has a new, short API in the style of Python and JavaScript, and
the compiler calls procedures with method syntax on TYPE values (`names.Add "x"`).
Programs that used the old module names or procedures must be changed (see Standard
library under Changed, and Removed).

Pre-releases from this section: `2.0.0-preview` (2026-11-13), `2.0.0-preview.2`
(2026-11-14), which adds the fixes of two audits of the compiler, runtime, tools and
installer, and CI checks of the GPU paths, a real window and the installer on Linux,
macOS and Windows, and `2.0.0-preview.3` (2026-11-15). A third audit added the fixes
marked so below and removed code nothing used (about 1,900 lines of the compiler and the
Rust crates); `2.0.0-preview.3` adds the items marked "In 2.0.0-preview.3".
`2.0.0-preview.4` (2026-11-16) adds the items marked "In 2.0.0-preview.4": GPU 3D in
the standard builds, faster 3D drawing and `qb --help` options that do what they say.
`2.0.0-preview.5` (2026-11-18) adds the items marked "In 2.0.0-preview.5": programs about a quarter of
their former size.

### Changed

#### Compiler and runtime

- The compiler emits QIR (the QBNex intermediate representation) instead of source code for an external compiler. A compiled program is the Rust runner `qb-exec` with the program's QIR appended: QIR is lowered to bytecode, run on a register VM, and hot loops are compiled to native code with Cranelift.
- The whole runtime (screen, graphics, fonts, images, sound, files, network, `_MEM`, `_FLOAT`, `DECLARE LIBRARY` calls) is Rust (`crates/runtime/core`, `qb-audio`, `qb-float80`, `qb-ffi`, `qb-gpu`); no OpenGL, FreeGLUT, FreeType, miniaudio or zlib is linked. Behaviour was checked against the old runtime with conformance and differential tests.
- Messages that named the old back end's compiler or its types now describe the QIR build: the array return type error is now "Unsupported array element type". `$EXEICON`, `$VERSIONINFO` and `$CHECKING:OFF` are accepted and have no effect (see the limitations page).
- Native executables (preview): with the native kit next to the runner, the compiler links ordinary executables through `rust-lld` and the `qb-native` library.
- `-z`, `-p` and `--ir` are accepted for old scripts and change nothing. `--emit-ir` (or an `-o` name ending in `.qir`) writes the program's QIR text, for `qb-web`.
- `-s` shows and sets `ignorewarnings` and `debuginfo` (kept for old scripts, no effect). Settings live in `target/qb-config.ini`.
- Without `-o`, the executable (or `--emit-ir`'s `.qir`) is always written next to the source file, never into the QBNex folder. `--exe-with-source` and the `exewithsource` setting (`SaveExeWithSource` in `qb-config.ini`) are removed.
- `qb run <file>` (or `--run`) runs the program after a successful build, from the current folder, and `qb` exits with the program's exit code; arguments after `--` go to the program's `COMMAND$`.
- Friendlier command line: one-letter switches run together (`-wq`), long spellings (`--output`, `--quiet`, `--warnings`, `--no-color`, `--explicit`, `--werror`), "Did you mean" for a mistyped switch or source file name (or the list of `.bas` files in that folder), a hint to use `qb run <file> -- <args>` when a second file name is given, and a "Run it" line after the build.
- The build screen shows an Options line with every switch (and the `ignorewarnings` setting) that changes the build, so a switch whose effect is not visible in a clean build (`-e`, `-w`, `-Werror`, `-d`, `-k`) still shows that it was taken.
- `qb fmt`, `qb lsp` and `qb dap` work when `qb` is `qb.exe` on the PATH (before, `qb.exe` came first and took the subcommand for a source file name); `qb-lsp` and `qb-dap` answer `--help` and `--version`.
- Windows: `OPEN`, `SHELL _HIDE` / `_DONTWAIT`, `FILES` and `_DIR$` use the Unicode ("W") system calls with UTF-8 names, as `_FILEEXISTS` and `KILL` already did; a file with a Thai name, or in a Thai folder, was not found (the compiler said "internal error 76" for such a source file), `FILES` in a Thai folder was "Illegal function call", and `_DIR$` gave `?` for each Thai letter of a user name. Names in other bytes still go through the code page.
- `-o` followed by a switch (`-o -q`) is an error instead of a file called `-q.exe`; `-o prog.bas` is refused on Windows too (it wrote `prog.bas.exe`); a folder given as the source file is named as one, with its `.bas` files; "Did you mean" for a file name with four letters or fewer allows one changed letter; `qb-lsp` says that its input ended instead of "disconnected channel".
- `-r` is no longer an undocumented `-Werror`; `-s:ignorewarnings=true` now also keeps `-Werror` from failing the build, as its help says.

#### Console and build screen

- The compiler's build screen: a shaded logo shading from periwinkle through sky blue and mint to light green, icons, the source file, a progress bar redrawn in place with a carriage return, the line count and the build time. Help, version, info, examples, warnings and diagnostics use the same colors. It no longer runs together on one line or writes over the shell prompt. `-m` or the `NO_COLOR` environment variable keeps plain ASCII output and turns the diagnostics' colors off too.
- Console programs on Linux and macOS: `COLOR` shows the QBasic colors (1 is blue, 4 is red; before, the terminal's ANSI order swapped them), and `CSRLIN`/`POS` read the terminal's cursor. LOCATE, COLOR and CLS go through the `crossterm` library. On a Windows console, `COLOR` 16 to 255 now shows the 256-color palette (as a VT sequence) instead of a wrong 16-color attribute. On every system, text PRINTed before `LOCATE`, `COLOR`, `CLS`, `CSRLIN` or `POS` now reaches the console first.

#### Performance

- The compiler is about 25-30% faster with the same QIR output: a 10,000-line program compiles in 14.7 s instead of 20.2 s, a 5,000-line one in 7.3 s instead of 10.5 s, a 10,500-line program with 300 SUBs in 24.0 s instead of 32.3 s, and the compiler itself in 41 s instead of 51 s. An assignment's right side is fixed once instead of twice; the element, operator and symbol lookups take shorter paths (scalar caches, a hash that spreads names over the whole table, an index for the used-variable list); `qb-exec` runs the pure string builtins (UCASE$, LTRIM$, STR$, VAL, MKL$, ...) without the runtime's name dispatch. `QB_PROFILE` also reports inclusive time and exact call counts per function and per caller.
- Compiling large programs is about 10 times faster (10,000 lines: 20 s instead of 250 s): reading part of a long STRING variable (`LEN`, `ASC`, `MID$`, `LEFT$`, `RIGHT$`, `INSTR`, comparisons) no longer copies the whole string first.
- The JIT and `qb-exec build` compile a big program much faster: native code writes frame slots through to memory instead of writing every slot back at each exit (that made Cranelift time grow with slots x exits), already-complete blocks are sealed at once, and the functions are compiled to machine code on all CPU threads while the IR of the next ones is built. A 10,500-line program (305 functions) now compiles in 0.3 s instead of 13.8 s (1.5 s on one thread); its run time and a hot-loop benchmark are unchanged. `QB_JIT_THREADS` sets the number of compiler threads, `QB_JIT_STATS=1` also prints compile times.
- A large program no longer stalls at its first hot loop while the JIT compiles it: big modules are compiled on a background thread while the program keeps running (a 10,500-line program: 1.7 s instead of 16 s). The JIT also declares only the functions each function calls.

#### Build, repository and CI

- `scripts/setup_win.cmd`, `scripts/setup_lnx.sh` and `scripts/setup_osx.command` build `qb-exec` with cargo, pack the bootstrap compiler `bootstrap/qbnex.qir` into `qb-stage0`, and let it compile `compiler/qbnex.bas` into `qb`. `internal/` is gone. `scripts/selfhost_fixpoint.*` checks that the compiler rebuilds itself to the same QIR.
- Reorganized the repository: compiler sources moved from `source/` to `compiler/`; the runtime is a Cargo workspace in `crates/` (`compiler/`, `engine/`, `runtime/`, `tools/`, `web/`); tests live under `tests/` (`smoke/`, `fixtures/`, `conformance/`, `docs/`); setup scripts moved to `scripts/`, Docker files to `docker/`, the VS Code extension to `packaging/vscode/`.
- Rust 1.100.0 is the required toolchain (`rust-toolchain.toml`, `rust-version`). All crates share the workspace version (2.0.0).
- CI is one workflow (`ci.yml`: audit, build, clippy, tests, smoke, conformance, self-host fixpoint, native executables on Linux, macOS and Windows x64) plus `release.yml`, and `docs.yml` compiles the BASIC examples of the README and the manual; they run with read-only repository permissions.
- `release.yml` signs `qb-fmt`, `qb-lsp` and `qb-dap` when signing secrets are set (Windows: `signtool` with a `.pfx`; macOS: `codesign` and `notarytool`) and skips with a notice otherwise; the secrets are listed in CONTRIBUTING.md. The macOS and Linux CI build no longer fails on the conformance harness, the smoke tests keep the system `TMPDIR` (macOS sets it), and each smoke script runs under the shell its first line names.
- Release bundles include `qb-fmt`, `qb-lsp` and `qb-dap`, next to the one-file installers.
- The Windows installer has the QBNex icon and version information (description, file and product version, product name, copyright, language), written as a resource file by its build script and linked without a resource compiler.
- CI also runs the GPU tests on a software adapter (Mesa lavapipe on Linux, WARP on Windows), a compiled program in a real X11 window under Xvfb (`tests/smoke/window_smoke.sh`: the drawing on screen, a key press), and the one-file installer on Linux, macOS and Windows (install over a folder with the user's files, compile and run a program with it, install again, uninstall: `.github/scripts/installer_check.*`).
- Developer documents are `docs/dev` (QIR format, legacy defects and deviations, FFI, 3D API).

#### License

- QBNex is now licensed under the GNU General Public License, version 3 or later, with the QBNex Compiled Program Exception (`LICENSE-EXCEPTION.txt`): programs compiled with QBNex may be licensed under any terms. The parts that come from QB64 keep their MIT license (`licenses/QB64.txt`). Releases made before this change remain available under the MIT license. Added `NOTICE.txt`, `TRADEMARKS.md` (use of the name and logo) and `CLA.md` (contributor agreement).

#### Standard library

- One flat module per file in `compiler/stdlib/`: `core`, `collections`, `strings`, `json`, `url`, `path`, `csv`, `math`, `sys`, `result`, the Rust-backed `regex`, `hash`, `zip`, `http`, `utf8` and `datetime` (with their shared `ext`), and `oop.class` / `oop.interface` for CLASS. They are imported as `'$IMPORT:'collections'`, `'$IMPORT:'sys'`, ...; `'$IMPORT:'qbnex'` imports all of them (`compiler/stdlib/qbnex.bas`). The old dotted names (`collections.list`, `sys.env`, `io.path`, `ext.regex`, ...) are gone.
- Collections: `List`, `Dict` and `Set` of strings are ready to use without an Init call, and a copy is the same collection. `List` has `Add`, `Insert`, `Get$` and `Set` with negative indexes counting from the end, `Pop$`, `RemoveAt`, `Remove`, `IndexOf`, `Has`, `Join$` (default `", "`), `Split`, `Sort`, `Reverse`, `Clear` and `Free`; `Dict` has `Get$(key$[, fallback$])`, `Set`, `Has`, `Remove`, `Count`, `Key$`, `Value$`; `Set` has `Add`, `Has`, `Remove`, `Count`, `Join$`.
- New names: `Text_*` is `Str_*` (`Str_Has&`, `Str_Replace$`, `Str_Trim$`, ...); `Env_*` and `Args_*` are `Sys_*` (`Sys_Platform$` returns `"windows"`, `"linux"` or `"macos"`, `Sys_Env$`, `Sys_Arg$`); `Path_FileName$`/`DirName$`/`Extension$`/`Separator$` are `Path_Base$`/`Dir$`/`Ext$`/`Sep$` (plus `Path_Stem$`, `Path_Exists&`); `Math_Deg2Rad#`/`Rad2Deg#` are `Math_Radians#`/`Degrees#`; CSV rows are a `List` (`Csv_Row$`, `Csv_Parse`).
- JSON: `Json_Parse`, then path reads such as `Json_Str$(root, "users[0].name")`, `Json_Num#`, `Json_Bool%`, `Json_Type$`, `Json_Has%`, `Json_Count`, `Json_Key$`; building with `Json_Object`, `Json_Array`, `Json_SetStr`/`SetNum`/`SetBool`/`SetNull`/`Set`, `Json_PushStr`/`PushNum`/`PushBool`/`Push`, `Json_Remove`; `Json_Stringify$(v[, indent])` replaces `JSON_Stringify$` and `JSON_StringifyPretty$`.
- URL: `DIM u AS Url: u.Parse text$` replaces `UrlParse` with `UrlParts`; `u.Query$`, `u.SetQuery`, `u.Format$`, `Url_Encode$`, `Url_Decode$`, `Url_Resolve$`, `Url_ToPath$`, `Url_FromPath$`.
- `Result` is a TYPE with the fields `Failed`, `Value`, `Message` and `Code`, set by `r.Ok`, `r.Fail message$[, code&]` and `r.Context where$`.
- Native modules: `Regex_Test&` replaces `Regex_IsMatch&`; `Regex_Find$` returns the match (with `Regex_Group$`, `Regex_Pos&`) instead of `Regex_Find&` + `Regex_Match$`; `Regex_Split` and `Regex_FindAll` fill a `List`; `Http_Get$`, `Http_Post$` and `Http_Request$` return the body, with `Http_Status&` and `Http_Header$`; `Zip_List` fills a `List`; the `DT_*` functions are `Date_*` (`Date_Now#`, `Date_Make#`, `Date_Iso$`, `Date_Format$`, `Date_Parse#`, ...) and take an optional last argument for local time.
- One `LastError$` (module `core`) holds the message of the last failed library call for every module, `""` after a call that worked; it replaces `Ext_LastError$`, `JSON_LastError$` and `UrlLastError$`.
- `LIST` is no longer a reserved word, so a TYPE or a variable can be named `List`.
- The manual, the README, the VS Code snippets and the language server's completion write library calls without the type suffix of the FUNCTION (`names.Get(0)`, `Json_Str(root, "user.name")`, `LastError`); writing the suffix still works.
- A compile error on a line that method syntax rewrote shows the line as written (for `x = l.Get(0)` it showed `x = LIST_Get(l, 0)`).

#### In 2.0.0-preview.3

- `DECLARE LIBRARY "name"` and `DECLARE DYNAMIC LIBRARY "name"` without a path no longer load a file of that name from the current directory. The library is looked for in the program's folder (for `qb-exec prog.qir`, the folder of `prog.qir`; under the debugger, the folder of the `.bas` file), then in the system folders. A library that sits only in the folder the program is started from must now be named `"./name"` (the source folder) or with a path, or be put next to the program. `"./name"` means the source folder also when the source was given without a folder (`qb prog.bas`).
- Console `INPUT` asks again with "Redo from start", as QBasic does, when the answer has too few or too many items, or a numeric item is not a number or is too large for its variable. At the end of redirected input the last answer is taken as it is. Numeric `INPUT` in a window ignores keys that cannot be part of a number. In a terminal `INPUT` works line by line (stdin used to be read to its end first) and `PRINT` output shows at once; into a redirected stdout `PRINT` is no longer flushed after every item, but before input, `SLEEP`, `SHELL`/`RUN`/`CHAIN`, error messages and at the end.
- BYREF parameters are real references: a SHARED variable changed under its own name inside the procedure is changed for the parameter too, and the reverse; a whole array passed as `a()` stays usable under its SHARED name during the call; `RESUME label`, `RETURN label` and failed calls no longer lose what procedures did to their BYREF strings and arrays; `ON ERROR` and event handlers see the arguments' current values.
- `LOCK`/`UNLOCK` work on Linux and macOS (fcntl record locks; error 70 on a conflict; `UNLOCK` must name a locked range); `OPEN … LOCK READ/WRITE` is enforced on Linux.
- Sound: `_SNDLIMIT` limits how long a sound plays from its `_SNDPLAY`/`_SNDLOOP` (it counted from when the audio engine started, so a sound played later stopped at once); `_SNDCLOSE` of a looping sound lets it finish its current pass and frees it (it looped for ever); `_SNDOPEN(f$, "STREAM")` and `_SNDPLAYFILE` decode the file while it plays; a sound decoded into memory is limited to 4 hours and 4 GiB of samples (a longer file fails to open: use "STREAM"); `SOUND` and `PLAY` render without holding up other sounds, and a long `SOUND` is generated as it plays.
- Window settings made while the window is hidden (`$SCREENHIDE` with `_FULLSCREEN`, `_RESIZE ON`, `_SCREENMOVE`, `_MOUSESHOW` cursor, `_ICON`, size) take effect when it is shown. Windows: `_SCREENCLICK x, y, button` sends one press and release of that button (it pressed the left button too and left it down).
- Compiler: `$IF`/`$END IF` no longer close the BASIC block that is open, so a block may open in a `$IF` branch and close after `$END IF`, or the reverse; `$IF THEN` / `$ELSEIF THEN` with no condition, `END SUB`/`EXIT SUB` inside a FUNCTION (and the reverse) and a typed `CONST` whose value does not fit its type are compile errors; a leading unary plus is accepted (`x = +5`); `$INCLUDE` names are matched ignoring letter case on Linux and macOS when the exact name does not exist; `$IMPORT` lines inside `$INCLUDE` files are hoisted like those of the main file; the compiler no longer rewrites `target/qb-config.ini` on every start.
- stdlib: `'qbnex'` imports its modules instead of holding a copy of them, and combines with any of them in any order; collections have no fixed limit on lists or items (it was 256 lists) and item access is constant time; a failed JSON parse keeps nothing; URL fields are variable-length and an invalid port is reported as an error.
- Speed: `INPUT #`, `LINE INPUT #` and `INPUT$` of files read ahead (`LINE INPUT` of a 5 MB file: 1.45 s to 0.05 s); built-in calls are resolved once at load (PRINT loops about 30% faster) and one-argument math functions (`SQR`, `SIN`, `INT`, `ABS`, ...) skip the name dispatch (a `SQR` loop runs about 3 times faster); fewer allocations per array-element access, string-element access and `READ`; faster native start-up; `_PRINTWIDTH` and PRINT with proportional `_LOADFONT` fonts use a glyph cache; `VAL` of extremely long digit strings is fast and still correctly rounded.

#### In 2.0.0-preview.4

- The setup scripts build `qb-exec` with the `gpu` feature, so the release bundles and every packaged program draw `_DRAW3D`, `_LOADMODEL`, `_MESH*` and `_SHADER` with wgpu (Vulkan, DX12, Metal) and show the window through the GPU; `QBNEX_GPU=0` builds without it. With only a software adapter the window keeps the software presenter; with no adapter or `QB_GPU=off` 3D draws nothing and `_GPU3D` returns 0; a GPU error is printed once and turns 3D off instead of ending the program.
- 3D `_MAPTRIANGLE` is queued and drawn on the GPU in batches, with a depth buffer kept on the GPU per image; the software renderer still draws without a hardware adapter, after a GPU error and for images handed out by `_MEMIMAGE`. The native kit builds `qb-native` with GPU 3D too.
- 3D speed: the `_DRAW3D` layer keeps its target and readback buffers between frames (about 330 to 1000 fps at 800x600); `_MAPTRIANGLE` skips the builtin name lookup and reuses its GPU buffers; the 3D layer and batched `_MAPTRIANGLE` share one GPU device (half the GPU memory); the software triangle rasterizer finds each row's span from the edges and works in runs of 16 pixels, with bit-identical pixels and depth (a large textured mesh: 1289 to 619 ms per frame).
- `qb --help`: `-s:<name>=value` saves the setting without printing the help, `-d` shows the source lines around a diagnostic, `-k` hides detail notes also with `-Werror`, `-w` prints warnings on their own lines and ends with a warning count, `-i`, `-g` and `-s` show more, and `-m` works anywhere on the line. `-c`, `-x`, `-p`, `-z` and `--ir` change nothing and are no longer listed, but are still accepted.
- `DECLARE LIBRARY` and `DECLARE DYNAMIC LIBRARY` are checked at compile time with the same search order the runner uses to load the library.
- `qb-fmt` and the language server read `a$"..."` as a name followed by a plain string, as the compiler does; they took the `$` of the name for the start of an interpolated string, and formatting could change the program.
- `examples/showcase`: eight runnable programs shown in the README (among them `life.bas`, Conway's Game of Life), `shapes3d.bas` (six 3D shapes drawn with `_DRAW3D` or `_MAPTRIANGLE` 3D), `physics3d.bas` (five animated physics scenes on the GPU with `_SHADER` highlights and soft shadows), and `raytrace_gpu.bas` (six animated scenes ray traced live by a WGSL `_SHADER`: reflections, glass, soft shadows, coloured lamps).

#### In 2.0.0-preview.5

- Smaller programs: a compiled program is the launcher `qb-stub` with the runner packed behind it (brotli with a branch filter, a third of its size) and the QIR text as a zlib stream (a fifth of its size). The launcher unpacks the runner once per runner build into the user's cache folder and starts it there. On Windows x64 a hello world is 4.2 MB instead of 18.4 MB, a window or 3D program 5.7 MB, the compiler `qb.exe` 5.4 MB instead of 24.2 MB. Run time is unchanged; a start takes about 15 ms more on Windows, where the runner is a child process of the launcher (it passes on the exit code, leaves Ctrl+C to the program and ends the runner when it is killed). `COMMAND$(0)`, `DECLARE DYNAMIC LIBRARY` and the "own console" check go by the program file, not the cached runner.
- Console-only programs that use no 3D get the runner built without the GPU feature (`qb-exec-lite.pack`, 4.2 MB packed instead of 5.7 MB); window programs keep the GPU runner, which presents the window through the GPU.
- The release binaries keep no symbol table, and the runner builds only the host's Cranelift back end. Fat LTO was measured (3% smaller, twice the build time and memory) and left off.
- `qb-payload` holds the layout of a compiled program in one place for the runner, the launcher and `qb-dap`, which now also reads compressed programs. Setup builds `qb-stub` and packs the runners with `qb-pack`; `QBNEX_PACK=0` writes a copy of the runner as before.

#### Since 2.0.0-preview.5

- The compiler is about twice as fast with the same QIR output: it compiles itself in 11.4 s instead of 21.7 s (Linux x64), a 941-line program in 0.65 s instead of 1.04 s. The compiler keeps the positions of a line's elements (tokens) instead of scanning the line again for each one, cuts a statement out of its line at once, and walks a symbol's hash chain without a call per candidate. `qb-exec`, which runs the compiler and every program: the memory-backed variables of a SUB/FUNCTION call are one allocation instead of one per variable, string registers and memory strings keep their buffers between calls, a TYPE assignment is one copy instead of one per field, a SHARED string read by LEN/INSTR/MID$/a comparison or appended to (`g$ = g$ + x$`) is no longer copied first, `INSTR(p, text$, CHR$(13))` no longer copies all of `text$`, and UCASE$ / the TRIMs write into the result's own buffer. Programs and the JIT run as before; the self-host fixpoint and the conformance suite are unchanged.
- Programs start about 20% sooner: loading the QIR no longer rescans a whole function at every label, and the names are hashed with FxHash. The compiler reaches its first line in 140 ms instead of 173 ms, most of the time of compiling a small program (the examples, tests and conformance sources, 304 files: 67 s instead of 94 s).

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
- Method syntax on TYPE values: for a variable, parameter, array element or field of a TYPE `T` that has no field of that name, `x.Name(args)` calls `T_Name(x, args)`, the statement `x.Add "a"` calls `T_Add x, "a"`, and the expression `x.Count` calls `T_Count(x)`. CLASS methods come first. It works for user TYPEs as well as for the library's `List`, `Dict`, `Set`, `Url` and `Result`.

#### Graphics, window and sound

- 3D API (preview): `_LOADMODEL`, `_MESH*`, `_DRAW3D`, `_CAMERA`, `_LIGHT`, `_AMBIENT`, `_CLEAR3D`, `_GPU3D` and user WGSL shaders (`_SHADER`, `_SHADERPARAM`, `_MESHSHADER`), drawn with wgpu by a runtime built with the `gpu` feature; without a GPU the statements run and draw nothing. `_MAPTRIANGLE` 3D is drawn on the GPU in batches when a hardware adapter is present, else by the software renderer.
- `SUB _GL` and the `_gl*` functions run on a real OpenGL context (WGL, EGL/GLX, CGL; an offscreen one when headless); `QB_GL=off` turns it off.
- The QB64 keywords the QIR back end did not compile yet now compile and run: `_RESIZE`/`$RESIZE`, `_WINDOWHASFOCUS`, `_WINDOWHANDLE`, file drop, `_FPS`, `_ICON`, `_PRINTSTRING`, `_MEMIMAGE`, `_SCREENIMAGE`, `_SCREENCLICK`, `_SCREENPRINT`, `_MOUSEMOVE`, mouse pipes, `_DEVICES` and the device functions with `STICK` and `STRIG`, `_MAPUNICODE`, `LOCK`/`UNLOCK`, console input, cursor and font, `SHELL` with `_HIDE` and `_DONTWAIT`. QuickBASIC statements old programs still contain (`FRE`, `SETMEM`, `FILEATTR`, `IOCTL`, `PEN`, ...) compile and answer as a machine with plenty of memory and no light pen (see the limitations page).
- Variable-length `STRING` fields in `TYPE`.
- ALT+ENTER switches a window between windowed, stretched and square-pixel full screen; `_ALLOWFULLSCREEN` limits the modes or, with `_OFF`, leaves the keys to the program.
- A silent audio device: without an output device (or with `QB_AUDIO_DEVICE=null`) sound is mixed in real time and discarded, so `PLAY(n)`, `ON PLAY(n) GOSUB` and sound timing work as with speakers. `QB_AUDIO_DEVICE=none` still starts no audio at all.

#### Standard library, tools and installer

- Standard library modules backed by Rust: `regex`, `hash` (with Base64 and hex), `zip`, `http` (HTTPS through rustls), `utf8` and `datetime`, with their shared plumbing in `ext`.
- Tools: formatter `qb-fmt`, language server `qb-lsp`, debugger `qb-dap` (run as `qbnex fmt|lsp|dap`), a VS Code extension (`packaging/vscode`), and a WebAssembly build of the runner (`crates/web`).
- One-file installer `qbnex-setup` (crate `crates/tools/setup`): releases publish `qbnex_<tag>_<os>-setup(.exe)`, which installs per user without administrator rights (Windows: `%LOCALAPPDATA%\Programs\QBNex`, user PATH, Start menu, Apps & features entry; Linux/macOS: `~/.local/share/qbnex` with links in `~/.local/bin` and a "QBNex Terminal" menu entry) and uninstalls with `--uninstall`. A setup window leads through the license, the folder and the optional tasks (desktop shortcut, "Compile with QBNex" for `.bas` files), shows the progress and can be cancelled (a cancelled or failed install puts back what was there); without a display it asks in the terminal. `--yes`, `--dir`, `--no-path`, `--no-menu`, `--desktop`, `--associate`, `--console` (and `/S`, `/DIR=`) script it. It keeps the user's PATH entries and files.
- User manual in English and Thai (`docs/manual`), checked by `tests/docs/check_docs.ps1`.

### Fixed

- Console programs on a terminal: box drawing (`CHR$(201)`, `STRING$(5, 205)`) shows as code page 437 instead of stopping all later output on a Windows console that is not in code page 65001; Thai printed a byte at a time still comes out whole. `INKEY$` reads the keys typed in a `$CONSOLE:ONLY` program and `INPUT$(n)` takes them without Enter or echo, on Windows, Linux and macOS (arrows and F1-F12 as `CHR$(0)` + scan code).
- Output redirected to a file reaches it while the program runs (at least every quarter second and before `_LIMIT` waits); a program stopped with Ctrl+C left an empty file.
- The UTF-8 byte order mark at the start of a text file (`OPEN ... FOR INPUT`) or of stdin is no part of the first line.
- `SHELL`, `RUN` and `CHAIN` pass a program path with spaces and quoted arguments to `cmd /s /c` as written; `COMMAND$`, `ENVIRON$` and `_CWD$` keep names that are not UTF-8 (the runner panicked on a Latin-1 argument on Linux); `KILL` and `FILES` wildcards follow DOS rules (`*.*` matches names without a dot; case-sensitive only on Linux); the clipboard, the console title and the printer name keep Thai text.
- `$INCLUDE` looks next to the including file, then the main source folder, then the current folder, and says where it looked when the file is missing; `$INCLUDEONCE` works; an install folder with `'` in its name, UNC source paths, `$EXEICON` and `DECLARE LIBRARY` folders relative to the source, and an output file that is in use (a clear message instead of "internal error 75") are fixed.
- The installer keeps a user's file that has the name of a QBNex file (renamed `.before-qbnex`), uninstalls files before it unregisters and keeps what is still in use, warns about an elevated install outside the user's folders and about another `qb` earlier on PATH, reads the Linux desktop folder from `user-dirs.dirs`, and escapes `.desktop` entries and `%` in Windows shortcut commands. `qb.cmd` passes `( ) & !` in arguments and keeps the exit code.
- `qb-fmt` goes on after a missing file, writes files atomically and reads `-` as stdin; `qb-lsp` answers a malformed message with an error instead of ending, applies incremental changes in UTF-16 positions and finds `$INCLUDE` files as the compiler does; `qb-dap` passes the launch `args`, reports the program's real exit code, keeps breakpoints set before launch and stops in `$INCLUDE` files.
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
- `'$IMPORT:'qbnex'` combines with the modules it bundles, in any order (the later import used to be dropped).
- Standard library: setting or removing a list item no longer rebuilds the list (a 2000-item queue took minutes); JSON numbers near the DOUBLE limit
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
- `ON TIMER(n)` set when the program starts fires after n seconds, not 2n: the clock counts from the start of the program, and the first tick waited for one interval before it started counting (issue #7)
- Windows: a window program (no `$CONSOLE`) built as the runner with the program appended (no native kit) is a GUI executable, as `qb-exec build` already made it, so Windows no longer opens an empty console next to its window (issue #7)
- Linux: a program that opens a window, and the setup window, no longer abort when libxkbcommon-x11 (or libxkbcommon) is not installed; the program runs without a window and says which library is missing, setup asks its questions in the terminal
- GPU feature: a program made one Vulkan instance per GPU user (window, 3D scene) and enumerated adapters each time; the Vulkan loader loads and unloads drivers while it does that, which crashed another thread that was using its device. The instance and adapter are now made once per process.
- Found in a third audit, before 2.0.0. Events and errors: `RETURN label` out of an `ON TIMER`/`ON KEY`/`ON PLAY`/`ON STRIG` handler arms the trap again and leaves the SUB the event interrupted (main-module code ran on the SUB's variables and the event never fired again); `ON ERROR GOTO 0` inside a handler stops the program with the error being handled, as in QBasic; a program that leaves `GOSUB` routines by `GOTO` forever gets error 7 instead of using all memory. Comparisons with NaN are false in `IF`, `WHILE` and `LOOP` conditions as they are in an expression (`IF n < 1` was taken).
- Arrays and strings: `ERASE` of a static array clears it in place (an element passed by reference pointed into freed memory); `a(i) = f()` stores into the array as `f` left it when `f` changes its size with `REDIM` (the value was lost, or the program crashed); a SHARED string passed by reference keeps its text under its own name during the call; `ASC(s$, position)` past the end of the string is error 5 (it returned -1); `INPUT$(n)` in a window returns n bytes after an extended key.
- Compiler: a `SUB` or `FUNCTION` inside a `$IF` branch that is not compiled no longer makes all module code after it disappear, and an `$INCLUDE` in such a branch opens no file; `CONST` expressions with an intermediate value of 1E+15 or more keep the rest of the expression and all digits of a whole number (`CONST c = 3000000000 * 3000000000 + 1 - 3000000000 * 3000000000` was 9000000000000000000), and `OR` followed by `XOR` is accepted; `SELECT CASE` on a SINGLE matches fractional `CASE` values (`CASE 0.1`); `ABS` and `SGN` of an `_UNSIGNED _INTEGER64` from 2^63 up; `-o` naming the source file is an error instead of overwriting the source; a switch is matched whole (`-emit-ir` was taken for `-e`, and `-O2 file.bas` took the file as the output name); `OPTIONAL` defaults are filled for a call inside the arguments of another call (`h(h(1))` was "Incorrect number of arguments"); `Sys_Platform$` answers `macos` on macOS (it said Linux). Unused procedures left from the IDE and the old back end were removed from the compiler.
- Screen and sound: `TAB(n)` on a screen page moves to the next line when the cursor is past column n, as on the console; text printed on a text page is read as UTF-8 only when all of it is UTF-8, so a box drawn with `STRING$(10, 196) + CHR$(191)` keeps its corner; `VIEW` on a text page is error 5; `_RED`, `_GREEN` and `_BLUE` of a colour number above 255 on a text page are error 5 (they read past the palette); `_MESHTEXTURE` in a `$CONSOLE:ONLY` program raises an error instead of crashing; `_LIMIT` keeps the fraction of the frame time (`_LIMIT 60` ran at 62.5 frames a second, and anything above 1000 was not limited); `_LOADIMAGE(f$, 256)` frees its image when it fails; a PCX file whose header claims a picture far larger than the file can hold is not decoded (128 bytes could ask for gigabytes of memory); a `PLAY` statement that ended in an error no longer leaves its dots to the next one, and `V` and `N` reject negative values from a variable.
- Files and programs: `NAME` raises error 58 when the new name exists (the file was replaced); a full disk is error 61 ("Disk full"), not "Permission denied"; on Linux and macOS file names keep bytes that are not UTF-8 (two such names opened one file), `OPEN "COMn:"` is error 68 instead of creating a file of that name, and a read interrupted by a signal is repeated; `RUN` and `CLEAR` give back the DOS memory of `SADD`/`VARPTR` (a program that restarted itself with `RUN` ran out after some dozens of runs); the page collected by `LPRINT` is printed when the window is closed and before `CHAIN`/`RUN file$`; `RUN file$` and `CHAIN` start a program whose name has a `$` or a quote in it.
- Tools and installer: the formatter and the language server read `ELSE IF c THEN` at the end of a line as a block of its own (a correct program got "END IF without IF"); files of the previous version that an upgrade could not delete are no longer taken for an interrupted install and moved back over the new version; the debugger no longer copies the breakpoint set at every statement; `licenses/THIRD_PARTY.TXT` covers the tools and the installer too. "Compile with QBNex" in the file manager writes the program next to the source file (it went into the installation folder) through the new `--exe-with-source` switch. A program with many array accesses in one SUB starts quickly (the loader's work grew with the square of their number: 5 s for 20,000 accesses, 20 s for 40,000; now under 0.1 s).
- In 2.0.0-preview.3. Compiler: CONST expressions with `NOT` after an operator (`1 + NOT 0`, `10 / NOT 1`) and some mixed `AND`/`OR`/`XOR` expressions evaluate correctly (they were rejected or wrong); a one-line `SUB a: ...: END SUB` followed by module code compiles. VM: a fused compare-and-branch could leave a comparison result unwritten when it was also used through a widening conversion; the QIR loader and validator refuse malformed payloads (sparse register numbers, signed hex bytes, non-null pointer constants, malformed `meta extfn`, bad extern-call, memory-variable and CHAIN operands) instead of panicking or allocating huge tables. Runtime: `INPUT$(n, #f)` no longer allocates n bytes before checking the file; `_OPENCLIENT` gives up connecting after 10 s and `CLOSE` of a TCP connection waits at most 10 s; the window input queue is bounded; `_MEM` blocks are copied unaligned; Windows `$CONSOLE`: `COLOR` with one colour keeps the other; VIEW PRINT rows after a page switch follow the `_LOADFONT` font's height; `GET` of a picture too wide for its header (more than 8191 pixels in 256-colour modes, 32767 in SCREEN 1) is error 5 instead of a truncated header; image files whose header claims far more pixels than the file holds, or more than 16384x16384 pixels for compressed formats, fail to load (0/-1) instead of allocating gigabytes.

### Security

- `_CONNECTIONADDRESS` of a host asks api.ipify.org over HTTPS instead of plain HTTP.
- CI actions are pinned to commit SHAs.
- Found in the third audit: the compiler passes the output path to `chmod` and the native linker as one quoted word (`-o 'x$(command)'` ran the command); the `http` module does not send `Authorization`, `Cookie` or a request body on to another server a redirect names, refuses a redirect from https to http, and its timeout covers the whole request; `QBNEX_CHAIN` is honoured only for a `qbnex-chain-*.bin` file of the temporary folder (a program started with it deleted the file it named); the bytecode validator rejects a bad argument list instead of panicking; the local HTTP server that `qb-ext` has for its tests answers only under the test runner (`QBNEX_TEST_HOOKS=1`), not in every compiled program. The manual says that `DECLARE LIBRARY "name"` without a path tries the current directory first.
- In 2.0.0-preview.3: a bare `DECLARE LIBRARY` name is not loaded from the current directory (library planting, see Changed); qb-dap only accepts the debuggee that presents a random per-session token, so another local process can no longer take over a debug session through the loopback port; Docker base images are pinned by digest and rustup is installed from a checksum-verified, pinned `rustup-init`, the dev image runs as your uid/gid (`QBNEX_UID`/`QBNEX_GID`) and docker-compose no longer uses the host network (publish ports with `ports:`); the VS Code extension's minimatch is 11.0.8 (GHSA-g3hv-75wg-m249); the archived `paste` crate is replaced by its maintained fork `pastey`, and the remaining unmaintained-crate notices are documented in `.cargo/audit.toml`; CI checkout keeps no credentials, workflow permissions are minimal, and new jobs test the VS Code extension and the Docker image.

### Removed

- The old back end and runtime: `internal/` (the old runtime library, its build scripts and toolchain downloads), the old build system and the generated-code output of `-z`.
- One-shot migration scripts (`scripts/reorg_crates*.ps1`), the legacy oracle helpers (`tests/conformance/oracle`), the corpus harness that needed the removed back end (`tests/corpus-run`) and the Inno Setup script (`packaging/installer`), replaced by `qbnex-setup`.
- From the standard library: `Stack`, `Queue` (use a `List` with `Pop$` and `Pop$(0)`), `StrBuilder` (add the pieces to a `List` and `Join$` them), `ext.collections` (`XList`, `XDict`), the `QBNex_Date` TYPE of `sys.datetime` (use `datetime`), the JSON text builders of `io.json` (`Json_Object3$`, ...), the `Result` source, cause, expect, describe and propagate procedures, the URL path helpers, `QBNex_StdLib_Version`/`QBNex_StdLib_Info`, `Zip_EntrySize`, and the import aliases `stdlib`, `all` and `qbnex_stdlib` with their files `stdlib.bas` and `qbnex_stdlib.bas`.

## [1.0.4] - 2026-05-05

### Added

#### Input Runtime
- Added Unicode key event handling for non-ASCII Windows `WM_CHAR` input so extended characters are routed through the QBNex keyboard path instead of being truncated to single-byte callbacks.

### Changed

#### Build System
- Enabled self-hosting by default in the build so the standard build target produces the final `qb`/`qb.exe` compiler.
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
- Refined the native build flow and documentation so builds are easier to follow across supported platforms.
- Removed the generated compiler output from the tracked build inputs so the build uses freshly regenerated output instead of a stale checked-in copy.

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
- Added Windows precompiled-header generation for the shared runtime header so repeated compiler runs can reuse a dependency-specific PCH.

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
- Fixed the macOS build of `sub_environ` with Apple toolchains by using a mutable string buffer pointer.

#### Documentation
- Updated `README.md` to reflect the current release assets, CI matrix, tag-based release flow, and the distinction between local smoke tests and default hosted CI behavior.

#### Core Features
- Complete QBasic/QB4.5 compatible compiler
- Self-hosting compiler written in QBNex BASIC (~26,000 lines)
- Compiles BASIC source to native binaries
- Command-line interface with comprehensive flag support
- Support for 150+ QBasic/QB64 keywords and functions
- TCP/IP networking support with `_OPENHOST`, `_OPENCLIENT`, and connection management
- OpenGL extension support with `_GLRENDER` and `_DISPLAYORDER` metacommands
- TrueType font rendering via FreeType library
- Image format loading support (BMP, PCX, PNG, JPEG, etc.) via STB Image
- Optional ZLIB compression support
- Game controller/gamepad input support
- Compiler settings management via INI configuration (`internal/config.ini`)
- `-z` flag to output generated code without building an executable
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
- **Windows**: Windows 7+ (32-bit and 64-bit), native compiler toolchain
- **Linux**: OpenGL, ALSA, FreeGLUT, X11, native compiler toolchain
- **macOS**: Xcode command line tools, OpenGL, CoreAudio
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
- Generated-code output without building (`-z` flag)
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

1.x kept these in `internal/config.ini`, where `DebugInfo` added GDB symbols to the native build.

### Dependencies (2.0)
- Building QBNex needs the Rust toolchain (`cargo`); compiled programs need nothing else.
- Linux builds need a system linker (the `gcc` package), ALSA, udev, xkbcommon, X11 and `pkg-config` development packages.
- Window, graphics, sound, fonts, images and compression are Rust crates (see `licenses/THIRD_PARTY.TXT`).

1.x needed an external compiler toolchain with OpenGL, FreeGLUT, GLEW, FreeType, ZLIB and the platform audio libraries.

---

[Unreleased]: https://github.com/thirawat27/QBNex/compare/v2.0.0...HEAD
[2.0.0]: https://github.com/thirawat27/QBNex/releases/tag/v2.0.0
[1.0.4]: https://github.com/thirawat27/QBNex/releases/tag/v1.0.4
[1.0.3]: https://github.com/thirawat27/QBNex/releases/tag/v1.0.3
[1.0.2]: https://github.com/thirawat27/QBNex/releases/tag/v1.0.2
[1.0.1]: https://github.com/thirawat27/QBNex/releases/tag/v1.0.1
[1.0.0]: https://github.com/thirawat27/QBNex/releases/tag/v1.0.0
