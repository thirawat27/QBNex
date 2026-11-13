# Limitations and differences from QB64

[Manual index](../README.md) · [ภาษาไทย](../th/limitations.md)

QBNex compiles BASIC to QIR and runs it with the Rust runner `qb-exec`.
There is no C or C++ compiler anywhere in the pipeline. This page lists
what that means for programs written for QB64, and the defects that are
still open. Each item was checked against the code; fixed items are not
listed. Intentional changes and fixed QB64 bugs are on
[Differences from QB64 and QBasic](differences.md).

## Building QBNex and building programs

- **Building QBNex needs the Rust toolchain.** `scripts/setup_win.cmd`
  (and `setup_lnx.sh`, `setup_osx.command`) build the runner with `cargo`
  and stop with "cargo was not found" otherwise. Setting
  `QBNEX_QB_EXEC` to a prebuilt `qb-exec` skips that step. No MinGW or other
  C compiler is needed.
- **Building BASIC programs needs nothing else.** A built QBNex (or the
  release bundle) compiles programs without Rust installed: the compiler
  copies the prebuilt runner from `crates/engine/exec/bin/<os>/` and appends
  the program.
- `qb fmt`, `qb lsp` and `qb dap` need their tools built with `cargo`
  first.
- **Release files are not code-signed**, except `qb-fmt`, `qb-lsp` and
  `qb-dap` when the release was built with a signing certificate. `qb`, the
  installers and compiled programs carry data appended to the executable,
  which a signature would break, so SmartScreen and Gatekeeper may warn on
  first run.
- **macOS is tested in CI only** (arm64, the `macos-latest` runner: build,
  tests, smoke tests, conformance, self-host fixpoint). No macOS machine
  has run QBNex by hand, and Intel Macs are not tested.

## Two kinds of executable

| Kind | What it is | Status |
| --- | --- | --- |
| Runner + QIR (default) | A copy of `qb-exec` with the program's QIR appended. At start it runs the program on the VM and compiles hot code with Cranelift (JIT). | Available |
| Native | `qb-exec build` links machine code made ahead of time into an executable with `rust-lld`. The compiler does this only when a native kit exists in `crates/engine/exec/bin/<os>/native/`. | Preview |

Native executables:

- The native kit is built with the Rust toolchain
  (`scripts/native-kit.ps1` / `native-kit.sh`). Neither the setup scripts
  nor the release bundle build it. When the kit is missing, or a link
  fails, the compiler makes the default kind.
- They are console programs only. A program with a window still uses the
  runner + QIR kind.
- They still contain the VM and Cranelift. The machine code uses the
  instruction set extensions of the CPU that built it.
- Windows is tested on the development machine. Linux (ELF) and macOS
  (Mach-O) are built and tested in CI only.

Both kinds are about the same size and speed: on Windows x64 a hello world
is about 12.7 MB (the whole runtime: graphics, sound, network, JIT) and
starts in under 0.1 s.

## DECLARE LIBRARY

Calls go from the runner straight to machine code through `crates/engine/ffi`
(Win64, SysV x86-64 and AArch64 calling conventions). Without a C
compiler:

- **C and C++ sources and headers cannot be used.** QB64 compiled a
  header (`name.h`) or C/C++ source next to `DECLARE LIBRARY "name"`
  into the program; QBNex cannot.
- **Static libraries cannot be used** (`.a`, `.lib`, `.o`): there is no
  linker step.
- **Shared libraries are loaded by name.** `DECLARE LIBRARY "name"` and
  `DECLARE DYNAMIC LIBRARY "name"` load the shared library when one of
  its functions is first called (in the VM, the JIT and native `.exe`
  alike). The name is looked for as in QB64:
  - Windows: `name.dll` next to the path given in the name, in
    `System32`, then on the normal DLL search path;
  - Linux: `libname.so` (or `libname.so.version` for
    `"name:version"`) next to the path, in `/usr/lib64` and `/usr/lib`;
  - macOS: `libname.dylib` the same way.

  On Windows the compiler stops with `LIBRARY not found` when no such
  `.dll` exists; on Linux and macOS a missing library shows up as
  error 453 at the first call.
- **Where functions are found.** A declared function is looked up, in
  this order, in the runner (the `ext.*` functions of the standard
  library), in the library named in its `DECLARE LIBRARY` block, in the
  program itself, and in the system libraries:
  - Windows: ucrtbase, msvcrt, kernel32, user32, gdi32, advapi32,
    shell32, ole32, oleaut32, comdlg32, winmm, ws2_32 and ntdll;
  - Linux: libc, libm, libdl and libpthread;
  - macOS: libSystem.

  A function that is not found raises error 453 when it is first called.
- **Types.** Integers of every size, SINGLE, DOUBLE, `_OFFSET`, strings
  (passed as a pointer to their bytes) and BYREF arguments (passed as an
  address) work. Not supported: `_FLOAT` arguments or results (D-112),
  C functions with a variable number of arguments, and callbacks from C
  into BASIC.

## CALL ABSOLUTE

`CALL ABSOLUTE` runs the machine code in a small emulator of a 16-bit x86
in real mode, as QB64's runtime did. It covers what typical QBasic
routines use to pass values: MOV, PUSH, POP, segment prefixes, the 32-bit
operand and address prefixes, `INT` and RETF. Any other instruction
(arithmetic, compare, jump, loop, string instructions) stops with error 86.

Only `INT 33h` (mouse) is answered, for functions 0 (reset) and 3
(position and buttons). Other interrupts, such as `INT 21h` and
`INT 10h`, do nothing. `CALL INTERRUPT` and `CALL INTERRUPTX` answer the
same way; for other interrupts the output registers get the input registers.

## PEEK, POKE and DEF SEG

Conventional memory is 1 MB plus 64 KB. These areas behave as on a PC:

| Address | Contents |
| --- | --- |
| `&H417`, `&H418`, `&H496` | keyboard shift flags |
| `&H41A`, `&H41C`, `&H41E`... | keyboard buffer (head, tail, keys) |
| `&HA0000` | SCREEN 13 pixels (320x200) |
| `&HB8000` | SCREEN 0 text pages (character, attribute) |

Not emulated: the video mode byte `&H449` and the timer tick at `&H46C`.
Other BIOS bytes read 0 unless the program POKEs them.

BASIC variables do not live in conventional memory. `VARPTR` copies a
variable there; a POKE into that copy changes an array element, but not
a scalar variable.

## INP, OUT and WAIT

| Port | INP | OUT |
| --- | --- | --- |
| `&H3C7` | - | DAC read index |
| `&H3C8` | - | DAC write index |
| `&H3C9` | palette R, G, B of the read index | palette R, G, B (6-bit values) |
| `&H3DA` | bit 3 set once per 60 Hz frame (retrace) | - |
| `&H60` | keyboard scan codes | - |

Every other port reads 0, and OUT to it does nothing. This includes the
timer (`&H40`-`&H43`), the speaker (`&H61`) and the joystick (`&H201`).
`WAIT` polls INP: `WAIT &H3DA, 8` waits for the next frame; on other ports
it returns at once. Without a game controller `STICK` returns 128 and `STRIG` 0.

## _FLOAT

`_FLOAT` is an 80-bit x87 extended float computed in software, bit for bit
equal to the x87 unit. `EXP`, `LOG` and `^` with a non-integer exponent
use the x87 unit on x86/x86-64. On other CPUs they use a software version
that differs from the x87 unit on about 3% of inputs, by at most 1 unit
in the last place (D-111). Code using `_FLOAT` is not JIT-compiled, so
it runs slower than SINGLE and DOUBLE code.

## Statements that are not supported, or behave differently

The build stops with `QIR: unsupported: line N: ...` on:

- `ON TIMER(n) sub argument` (a SUB with an argument) inside a SUB or FUNCTION;
- `_MEM` of an array of variable-length STRINGs (its elements are not stored
  together; `_MEM` of one STRING variable works).

Every other statement and function the compiler knows is compiled (checked by
compiling a one-line program for each keyword; forms with optional arguments
were not all tried).

Compiled, but different:

- A program that uses no screen, window, mouse or key-state keyword (`SCREEN`,
  `LINE`, `LOCATE`, `_KEYHIT`, `_TITLE`, `_DEVICES`, ...) is built as a console
  program, as if it began with `$CONSOLE:ONLY`. With `$CONSOLE` a program has its
  window and a console; on Windows a program with a window and no `$CONSOLE`
  opens no console window.
- `STICK`, `STRIG` and the `_DEVICES` functions read game controllers in a fixed
  layout (19 buttons, 8 axes), whatever order the driver reports. Device 1 is
  the keyboard, with one button per scan code (`_BUTTON(scancode + 1)`).
- `PEN`, `COM(n)` and `UEVENT` with `ON`/`OFF`/`STOP`, and `ON PEN`, `ON COM(n)`
  and `ON UEVENT GOSUB`, compile and do nothing: there is no light pen, serial
  or user event. `PEN(n)` is 0.
- `FRE` returns fixed sizes: 16777216 for `FRE(-1)` and `SETMEM(n)`, 30000 for
  `FRE(-2)`, 60000 for anything else. `TRON` and `TROFF` do nothing.
  `FILEATTR(n, 2)` returns a number that stands for the open file, not a DOS
  handle. `IOCTL` and `IOCTL$` are an illegal function call (error 5) on an open
  file.
- `_CONSOLE ON|OFF`, `_CONSOLEFONT`, `_CONSOLECURSOR`, `_CONSOLEINPUT`, `_CINP`,
  `_CONSOLETITLE`, `_WINDOWHANDLE` and setting `_CAPSLOCK`, `_NUMLOCK` or
  `_SCROLLLOCK` work on Windows only; `_SCREENIMAGE` gives a blank image and
  `_SCREENCLICK`/`_SCREENPRINT` do nothing on Linux. `_DIR$` returns `./` off
  Windows.
- `_PRINTSTRING STEP(x, y)` moves on from the position of the last
  `_PRINTSTRING` (QB64 reads `STEP` and then ignores it).
- `_MOUSEPIPECLOSE` removes the pipe (QB64 leaves it in place).
- `TIME$ = ...` and `DATE$ = ...` set a clock of the program only: TIME$,
  DATE$ and TIMER then count on from the set time, but the system clock is not
  changed (QBasic set the DOS clock; QB64 ignores the statement). A badly formed
  value is an illegal function call.
- `_MEM(a$)` of a variable-length STRING is allowed (QB64 rejects it): a block
  over the string's current characters (SIZE = `LEN(a$)`, ELEMENTSIZE 1), valid
  until the string is assigned again, like `_OFFSET(a$)`. `_MEMFILL` of a value
  that is not a variable without `AS type` uses the value's own type
  (integer arithmetic such as `x% * 2` is an `_INTEGER64`).
- `SUB _GL` needs an OpenGL context (WGL, EGL/GLX or CGL; headless runs use an
  offscreen one): without one (no driver, `QB_GL=off`) it is not run and the
  runner says so on stderr. `_glGetString` returns an `_OFFSET`
  (QB64: a STRING).
- `LPRINT` prints pages of 60 lines: on Windows as an image of 8x16
  characters on the default printer, on Linux and macOS through `lp`.
  `QBNEX_LPRINT=<file>` appends the text to a file instead. Without a printer
  the page is dropped.
- `SOUND`, `BEEP` and `PLAY` follow QBasic's music modes: in `MF` (the
  default) the statement returns once its notes have played; after
  `PLAY "MB"` it returns at once and the notes play in the background, but a
  statement waits while more than 32 notes (the QBasic music buffer) or 10
  seconds of sound per voice are queued.
  QB64 never waited. A window close during a wait ends the program at once.
  In the browser build only the `MB` limit applies.
- Without an output device, sound plays on a silent device that keeps real
  time, so `PLAY(n)`, `ON PLAY(n) GOSUB` and the waits above behave as with
  speakers (`QB_AUDIO_DEVICE=null` selects it on purpose).
  `QB_AUDIO_DEVICE=none` starts no audio at all, for tests: sound statements
  do nothing and never wait, and `ON PLAY(n)` never fires.
- `RUN file$` and `CHAIN` replace the program on Linux and macOS. Windows cannot
  replace a running program: this one closes its files, stops its sound, hides
  its window and waits in the background for the other program, then ends with
  that program's exit code. Each program opens its own window.
- `$EXEICON` and `$VERSIONINFO` are checked and accepted, but the executable
  gets neither the icon nor the version information: it is the runner with the
  program's QIR appended, and no Windows resource step runs. `$CHECKING:OFF` is
  accepted and changes nothing; the checks stay on.

## Programs that time themselves with loops

Some old programs measure the speed of the PC with an empty loop and then wait by
counting loop iterations, or scale their waits by that speed. They were written for
PCs about a thousand times slower and break the same way under QBasic on a fast PC
and under QB64: a loop of 1000 iterations takes no measurable time (dividing by it
fails or gives 0), and a pause scaled by the loop count can last hours. Such programs
compile and run; change their delay code to wait on the clock, for example
`t# = TIMER(.001): DO: LOOP UNTIL TIMER(.001) - t# >= .05`.

## Known defects

| ID | Defect |
| --- | --- |
| L-01 | **Fixed.** Module code after the SUBs (with `ON ERROR` or `GOSUB` labels) is moved in front of the first SUB, also from included files. When the first SUB is in an included file, that code lands inside the file, after the `DIM SHARED`, `TYPE` and `CONST` it declares before that SUB. A runtime error in included code names the line in the included file. |

L-08 (with the old C++ runtime about 1 run in 300-400 ended with exit code `0xC0000409`) does not happen on the QIR runner: 5000 runs of `control/gosub_goto` all ended normally. Only that console program was run this many times.
