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

Both kinds are about the same size and speed: a hello world is about
8.5 MB and starts in about 48 ms.

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
it returns at once. `STICK` returns 128 and `STRIG` 0 (no joystick).

## _FLOAT

`_FLOAT` is an 80-bit x87 extended float computed in software, bit for bit
equal to the x87 unit. `EXP`, `LOG` and `^` with a non-integer exponent
use the x87 unit on x86/x86-64. On other CPUs they use a software version
that differs from the x87 unit on about 3% of inputs, by at most 1 unit
in the last place (D-111). Code using `_FLOAT` is not JIT-compiled, so
it runs slower than SINGLE and DOUBLE code.

## Statements that are not supported, or behave differently

The build stops with `QIR: unsupported: line N: ...` on:

- `CHAIN` inside a SUB or FUNCTION while a COMMON variable is not SHARED;
- `_MEM` of a variable-length STRING or of an array of them, and `_MEMFILL`
  of a value that is not a variable without `AS type` (QB64 rejects these too).

Compiled, but different:

- `TIME$ = ...` does not set the clock (as in QB64).
- `SUB _GL` needs an OpenGL context: without one (headless, no driver) it is not
  run and the runner says so on stderr. `_glGetString` returns an `_OFFSET`
  (QB64: a STRING).
- `LPRINT` prints pages of 60 lines: on Windows as an image of 8x16
  characters on the default printer, on Linux and macOS through `lp`.
  `QBNEX_LPRINT=<file>` appends the text to a file instead. Without a printer
  the page is dropped.
- `ON PLAY(n) GOSUB` needs an audio device: without one (`QB_AUDIO_DEVICE=none`)
  no notes are queued, so the handler never runs.
- `CHAIN` passes COMMON variables but not COMMON arrays; `RUN file$` and `CHAIN` wait
  for the other program to end.

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
| L-01 | Module code after the SUBs (with `ON ERROR` or `GOSUB` labels) is moved in front of the first SUB, also from included files. When the first SUB is in an included file, that code lands before the `$INCLUDE` line, so it cannot use a `DIM SHARED` or `TYPE` that the same file declares before its first SUB. A runtime error in moved included code names the `$INCLUDE` line. |

L-08 (with the old C++ runtime about 1 run in 300-400 ended with exit code `0xC0000409`) does not happen on the QIR runner: 5000 runs of `control/gosub_goto` all ended normally. Only that console program was run this many times.
