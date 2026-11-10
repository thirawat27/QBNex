# Corpus run: real programs, run headless

Real BASIC programs are compiled **and run**, through every backend, and their outputs are compared.
Runner: [`tests/corpus-run/run.ps1`](../../tests/corpus-run/run.ps1), inputs in `tests/corpus-run/inputs/`.

## Setup (2026-11-10)

- Build: branch `claude/run-tests-261ea4` = rust-port b97e32b + this work. `qb.exe` from `scripts\setup_win.cmd`
  (CARGO_BUILD_JOBS=1), `qb-exec` from `cargo build --release -p qb-exec`, copied to `crates\qb-exec\bin\windows`.
- Corpus (`tests/corpus-local`, gitignored): creationix/basic-games (129 files) and QB64Team/qb64 **tag v1.4**
  `programs/samples` (148 files; the current HEAD of that repository no longer ships the samples). 277 `.bas` files.
- Variants per program:
  - **cpp**: `qb file.bas` (C++ codegen + Rust qb-runtime). The primary result.
  - **oracle**: `.oracle\qb.exe` of the corpus-fixes worktree. Run only for console-only programs.
    Note: this oracle already uses the Rust window and `qb_gfx_*` graphics, so it is legacy only for the C++ parts.
  - **vm**: `qb file.bas --ir` (QIR), run with `QB_JIT=off`. **jit**: the same executable with `QB_JIT=eager`.
- Every run uses a fresh copy of the program folder, with the exe placed next to the data (QB64 executables
  chdir to their own folder). QB64 samples run from a QB64-root layout (they `CHDIR ".\programs\samples\..."`).
  Each run gets `QBNEX_HEADLESS=1`, `QBNEX_NO_ERROR_DIALOG=1`, stdin from `inputs/<id>.in` (default `_default.in`),
  keyboard events from `inputs/_default.input` (GUI programs; it ends with `close`), and a 6 s timeout.
- No window was opened on screen: GUI programs only run headless (Rust runtime), and the oracle only runs console-only builds.

Reproduce: `pwsh tests\corpus-run\run.ps1 -Oracle <corpus-fixes>\.oracle\qb.exe -Timeout 6` (`-Resume` continues a stopped
run; `-Filter 'cx__MISC__*'` selects programs). Results: `target\corpus-run\results.{json,md}` plus per-program logs.

## Summary (277 programs)

| status | count | meaning |
|---|---|---|
| pass | 203 | cpp exited 0 with no runtime error (GUI programs mostly after the scripted `close`); no difference against the variants that could be compared |
| rt-error | 40 | cpp stopped with a BASIC runtime error. Triage below: **none is a QBNex bug** (missing data files, DOS paths, input-driven) |
| binary-format | 13 | QuickBASIC fast-load binary files, not source |
| compile-fail | 7 | 2 CP437 sources, 2 `_GL`, 1 missing SUB, 1 `LCASE` redefinition, 1 `TYPE AS` (details below) |
| interactive-only | 7 | still waiting for input at the timeout (games / console prompts) |
| diff | 5 | all five: qb-exec does not print "Press any key to continue" at END (bug Q1) |
| timeout | 2 | SOUND loops; they end 2.6 s after the scripted `close`, beyond the 6 s timeout (see O1) |

- QIR: 133 of 277 compile with `--ir`; VM and JIT agree on all 95 programs where both finished (stdout + exit code).
- Console-only programs (12): the oracle ran on 10, identical stdout to cpp on 9 (the 10th is the END hang the oracle still has, fixed here as R1).
- GUI programs cannot be compared between cpp and vm/jit: qb-exec has no window yet (Q2), so only crashes/errors are reported for them.

## Bugs fixed here (qb-runtime, test-first)

**R1. END "Press any key to continue" and SLEEP spin forever when stdin is not a console** (commit f3c381a).
`_CONSOLEINPUT` (`func__getconsoleinput`) returned 0 forever when the input handle is a file or pipe:
`ReadConsoleInputA` fails and the zeroed record reads as "no event". The END wait and `SLEEP` (no argument) looped at 100% CPU.
Without a console there is no keyboard to wait for, so `console_input` now reports a key event at once.
Repro (console-only program, auto `$CONSOLE:ONLY`): `TYPE t: x AS INTEGER: END TYPE` saved as a .bas, run `prog.exe < NUL`:
before the fix it never exits (the oracle still hangs), now it prints the prompt and exits 0. Test: `console::stdio::console_input_tests`.
Found by ARTIF/ANT.BAS; also fixed MOSS/MUSIC, misc/replicat, misc/rot13, RPG2/QMIDI under the C++ path.

## Bugs for other sessions (not fixed here: compiler/ or qb-exec)

Each with a minimal repro; stdin redirected from a file.

**Q1. qb-exec: END does not print "Press any key to continue".** (5 diffs: ANT, MUSIC, QMIDI, replicat, rot13)
```basic
PRINT "x"
```
cpp: `x` + `\nPress any key to continue`, exit 0. QIR (vm/jit): `x`, exit 0.

**Q2. qb-exec has no window and ignores `QBNEX_INPUT_SCRIPT`.** A GUI program built with `--ir` prints its
text screen to stdout, its keyboard reads come from stdin, `close` never arrives. MAGIC/MAGIC.BAS (`DO ... LOOP`
drawing cards): cpp exits 0 after the scripted close, QIR runs to the timeout writing 3.8 MB to stdout. Side effects seen
in the corpus: `LOCATE 32, 28` / `LOCATE 29, 30` give error 5 (25-row console: alyman, nlcm), INPUT hits end of stdin (bj, move, rats1, nib64, mrguessit).

**Q3. qb-exec: INPUT with several variables reads values across lines.**
```basic
$CONSOLE:ONLY
INPUT "x"; a$, b, c
PRINT "["; a$; "]"; b; c
```
stdin `1\n7\n8\n`: cpp and oracle print `[1] 0  0` (missing values are 0, the rest of the line only),
QIR prints `[1] 7  8`. Seen in pete/averager (different averages from the second student on).

**Q4. qb-exec: INPUT at end of stdin raises error 62**; cpp and oracle return empty values (the program loops on its
prompt). wumpus, averager, intrprtr, battleship: cpp/oracle run to the timeout, QIR exits 1 with "Input past end of file".
Decide which is wanted; QBasic semantics would be the console behaviour (no EOF).

**Q5. qb-exec: `INPUT$(1)` without a file number gives error 52 "Bad file name or number".**
```basic
A$ = INPUT$(1)
PRINT ASC(A$)
```
QIR: error 52 at once (VNSTALL, misc/chess, pete/fight, hgm30e). cpp waits for a key (it reads the keyboard,
not redirected stdin, so with stdin from a file it waits until the timeout).

**Q6. QIR: ERASE frees a static array** (QBasic/cpp only clear `DIM a(constant)` arrays).
```basic
DIM b(3)
ERASE b: b(2) = 7: PRINT b(2)   ' cpp: 7    QIR: error 9 "Subscript out of range"
```
n54/3dsviewer (`ERASE result` then `result(j, i) = ...`). pete/25lines/3dcube25 also stops with error 9 under QIR
(arrays DIM'd in an ELSE branch that never runs, then READ into them); not minimised: `IF 0 THEN DIM a(5): a(3) = 1` and
`READ m!(v, 0)` into a 2-D array both work, so look at that program itself.

**Q7. QIR emitter: GOTO to a line label that has the same name as a SUB** ("undefined label").
```basic
DECLARE SUB title ()
title:
n = n + 1
CALL title
IF n < 2 THEN GOTO title
END
SUB title: PRINT "t": END SUB
```
cpp prints `t` twice. QIR: `qb-exec: invalid program: QIR: main: jmp L_TITLE: undefined label L_TITLE`
(pete/beatdown/bdt; qb45com/astrowars with `ON ERROR GOTO errcode` + `SUB errcode`). Without the GOTO it runs.
*Fixed (R11c-ir2):* the QIR was written before the compiler's repass that turns `title:` from a call into a label;
`--ir` now runs the label check first, and `IrInit` clears every function body for a repass.
Case `control/sub_name_label`.

**Q8. QIR emitter: unsupported statements** (programs that do not compile with `--ir`, count of programs):
BLOAD 23, OUT 22, BSAVE 20, RUN 17, WAIT 14, STICK 8, VARPTR of a non-STRING variable 5, CLEAR 4, TIME$ (statement) 3, INP 2,
CHAIN 1, REDIM of an unknown array 1.
*Status (R11c-ir2):* all but the last are lowered; qb-exec `rt/dosx.rs` has them.
- OUT/INP: VGA DAC ports &H3C7/&H3C8/&H3C9 (on the write page's palette), &H3DA (retrace bit, emulated at 60 Hz), &H60 (keyboard scancode queue). Other ports do nothing / read 0, as in the C++ runtime. WAIT polls INP and returns at once on other ports.
- STICK: 128 (no joystick device), argument checks as in the C++ runtime.
- BLOAD/BSAVE: DEF SEG memory (SCREEN 13 at &HA000 included). TIME$ = is a no-op (`sub_time` is a stub).
- VARPTR of a numeric variable / array element: QIR variables are not in conventional memory, so the bytes are mirrored in DBLOCK (the segment VARSEG returns). PEEK, BSAVE and CALL ABSOLUTE see the value at the time they run; POKE and BLOAD into an array mirror are written back to the array; a scalar mirror is a copy only.
- RUN, RUN label, CLEAR: `rt RUN k` / `rt CLEAR k` end the run like END; the VM restarts `main` with fresh variables and `RUNFROM` (first statement of main) jumps to target k. CLEAR keeps array dimensions with zero elements; RUN forgets them (the DIMs run again). Both close files and reset DATA, ON ERROR and GOSUB; RUN also resets RND and the screen (SCREEN 0, COLOR 7, 0).
- RUN file$ / CHAIN: run the other program, wait for it, then end. The C++ runtime does not wait (Windows); CHAIN does not pass COMMON variables.
- REDIM of an unknown array: `DIM [SHARED] name%()` (empty parentheses, L-21) leaves no current id after `dim2`, so neither the DIM nor a later REDIM found the array. The emitter now looks the array up by name (this SUB's own, then the module's). qb64 misc/gor64.bas is not in the local corpus; conformance `arrays/dim_empty_redim_in_function` has the same failure and now compiles with `--ir`.
Case `system/run_clear`, `graphics/ports_bload`.

**C1. compiler: CP437 sources are still rejected** ("Invalid UTF-8 byte sequence") although the corpus-fixes CP437
change is merged: MISC/SPACE.BAS (binary-looking first line) and misc/forest.bas (`DATA` line with bytes 0x80-0xFF in a string).

**C2. compiler: `LCASE` as a user SUB/FUNCTION name** is rejected ("'LCASE' is already defined", pete/upperlowerconversion; the program comes from the QB64 v1.4 samples).
Not checked against QB64 v1.4 itself. Low priority.

## Observations (not bugs)

- **O1.** In a loop of short `SOUND` calls, a window `close` takes ~2.6 s to end the program (SCORCH/SCORC, BOMB/BOMBMAN;
  `DO: SOUND 100, 2: LOOP` ends 2.2 s after `close`, a busy loop 1 s). It ends, so not a hang; maybe the sound queue drains first.
- **GET "Illegal function call"** in BANANA/PICS, NUKE/PICS, NUKE/CPICS, RPG2/CSHAPES, RPG2/SHAPES: the programs move
  `WINDOW SCREEN (0 - sx, 0 - sy)-(639 - sx, 479 - sy)` by 40 px per sprite, so with sx = 600 `GET (1,1)-(40,40)` maps to x = 640,
  off screen. QB64 v1.4 `sub_graphics_get` and WINDOW scaling compute exactly the same, so this is the programs' own bug.
- RPG2/SAMPLE.BAS reads the BIOS column count with `DEF SEG = &H40: PEEK(&H4A)`; QBNex (like QB64 v1.4) does not emulate
  that byte, PEEK gives 0 and `LOCATE CSRLIN, 0 - ...` is error 5. Emulating 0x44A (= `WIDTH`) would be a small enhancement.
- C/FONT.BAS is an editor: the default script types `1`, which becomes the DRAW command `"1"` (illegal in QBasic too).
  `DRAW "c=" + VARPTR$(c)` itself works (checked in the main module and in a DEFINT SUB).
- 49 runtime errors 76/53 are DOS paths (`CHDIR "\TIM"`, `C:\T1\`, `RUN "\tim\menu.bas"`), data files the
  programs create in an earlier step (thebob: `*gfx.bas` / K1 makes the `.bsv`/`.fnt` files, see their readme), or file names typed by the user.
- "Build failed" during a run was always the shared machine's commit limit (cc1plus dies); the runner retries once.
  Runs that die with "memory allocation of N bytes failed" are retried as well.
- The oracle continues after a runtime error ("Continue?" defaults to yes) where cpp exits; e.g. pete/battleship error 9 at line 24 in both.
- qb64/open_gl (`SUB _GL`) is deviation D-14; qb45com/assault declares `TYPE AS` (not valid in QB64 either).

## Results

`console` = console-only build (oracle comparable). Cells: exit code and run time; `timeout` = still running after 6 s.
| program | status | console | cpp | oracle | vm | jit | notes |
|---|---|---|---|---|---|---|---|
| creationix-basic-games/ARTIF/ANT.BAS | diff | yes | 0 1.1s | timeout 6.1s | 0 0.1s | 0 0.1s | oracle exit timeout vs 0; vm differs: stdout; jit differs: stdout |
| creationix-basic-games/ARTIF/MOUSE.BAS | pass | no | 0 4.4s | - | timeout 6.1s | timeout 6.1s |  |
| creationix-basic-games/BANANA/BANANA.BAS | pass | no | 0 1.7s | - | no compile | no compile | ir compile: QIR: unsupported: line 12: statement BLOAD |
| creationix-basic-games/BANANA/GRAPH.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/BANANA/PICS.BAS | rt-error | no | 1 0.9s | - | no compile | no compile | ir compile: QIR: unsupported: line 11: statement OUT |
| creationix-basic-games/BOMB/BOMBMAN.BAS | timeout | no | timeout 6.2s | - | timeout 6.3s | timeout 6.3s |  |
| creationix-basic-games/BOMB/GRAPH.BAS | pass | no | 0 1.4s | - | 0 0.1s | 0 0.1s |  |
| creationix-basic-games/BOMBMAN/BOMB.BAS | rt-error | no | 1 1.6s | - | no compile | no compile | ir compile: QIR: unsupported: line 22: function STICK |
| creationix-basic-games/BOMBMAN/COLOR.BAS | pass | no | 0 1.3s | - | 0 0.1s | 0 0.1s |  |
| creationix-basic-games/BOMBMAN/DRAW.BAS | pass | no | 0 4s | - | no compile | no compile | ir compile: QIR: unsupported: line 73: statement RUN |
| creationix-basic-games/BOMBMAN/LOAD.BAS | pass | no | 0 1.3s | - | 0 0.2s | 0 0.2s |  |
| creationix-basic-games/BOMBMAN/TESTING.BAS | rt-error | no | 1 0.7s | - | no compile | no compile | ir compile: QIR: unsupported: line 68: statement BLOAD |
| creationix-basic-games/BOMBMAN/VNSTALL.BAS | pass | no | 0 4.3s | - | 1 0s | 1 0.1s | vm (no window): exit 1 Unhandled Error #52 - qbnex_corpus_vm.exe: Line: 9 (in main module): Bad file name or number; jit (no window): exit 1 Unhandled Error #52 - qbnex_corpus_jit.exe: Line: 9 (in main module): Bad file name or number |
| creationix-basic-games/BOMB/NUKEM.BAS | pass | no | 0 1.4s | - | 0 0.1s | 0 0.1s |  |
| creationix-basic-games/BOMB/NUKER.BAS | pass | no | 0 5.8s | - | timeout 6.1s | timeout 6.1s |  |
| creationix-basic-games/C/CQUICK.BAS | pass | no | 0 5.1s | - | timeout 6s | timeout 6.1s |  |
| creationix-basic-games/C/FONT.BAS | rt-error | no | 1 2s | - | 1 0.8s | 1 0.9s | vm (no window): exit 1 Unhandled Error #5 - qbnex_corpus_vm.exe: Line: 55 (in main module): Illegal function call; jit (no window): exit 1 Unhandled Error #5 - qbnex_corpus_jit.exe: Line: 55 (in main module): Illegal function call |
| creationix-basic-games/C/QC.BAS | pass | no | 0 5.1s | - | timeout 6s | timeout 6.1s |  |
| creationix-basic-games/C/QUICK.BAS | pass | no | 0 5.1s | - | timeout 6.1s | timeout 6.1s |  |
| creationix-basic-games/DROIDS/DROIDS.BAS | pass | no | 0 5.2s | - | timeout 6.1s | timeout 6.1s |  |
| creationix-basic-games/FOWLER/EDIT.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/FOWLER/FOWLER.BAS | pass | no | 0 5.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 59: statement BLOAD |
| creationix-basic-games/FOWLER/GRAPH.BAS | pass | no | 0 1.5s | - | no compile | no compile | ir compile: QIR: unsupported: line 134: statement BSAVE |
| creationix-basic-games/FOWLER/WINDOW.BAS | pass | no | 0 4.4s | - | timeout 6s | timeout 6.1s |  |
| creationix-basic-games/GALAXY/GALAXY.BAS | pass | no | 0 4.8s | - | timeout 6s | timeout 6s |  |
| creationix-basic-games/GIRLS/WAR.BAS | pass | no | 0 4.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 51: function STICK |
| creationix-basic-games/LABYRINT/LABYRINT.BAS | pass | no | 0 5s | - | no compile | no compile | ir compile: QIR: unsupported: line 135: statement RUN |
| creationix-basic-games/LOGATH/CLOGATH.BAS | pass | no | 0 4.7s | - | no compile | no compile | ir compile: QIR: unsupported: line 114: statement RUN |
| creationix-basic-games/LOGATH/LOGATH.BAS | pass | no | 0 4.8s | - | no compile | no compile | ir compile: QIR: unsupported: line 112: statement RUN |
| creationix-basic-games/LOGATH/LOGATHG.BAS | pass | no | 0 1.3s | - | 0 0.1s | 0 0.1s |  |
| creationix-basic-games/LOGATH/STARS.BAS | pass | no | 0 4.4s | - | timeout 6s | timeout 6.1s |  |
| creationix-basic-games/MAGIC/SHUFFLE.BAS | rt-error | yes | 1 1s | no compile | no compile | no compile | ir compile: QIR: unsupported: line 39: statement RUN |
| creationix-basic-games/MASTER/BUILDING.BAS | pass | no | 0 4.4s | - | timeout 6s | timeout 6.1s |  |
| creationix-basic-games/MASTER/CMASTER.BAS | pass | no | 0 5.2s | - | no compile | no compile | ir compile: QIR: unsupported: line 445: statement TIME$ |
| creationix-basic-games/MASTER/CREATURE.BAS | pass | no | 0 4.3s | - | 0 2.6s | 0 2.6s |  |
| creationix-basic-games/MASTER/MASTER.BAS | pass | no | 0 5.3s | - | no compile | no compile | ir compile: QIR: unsupported: line 431: statement TIME$ |
| creationix-basic-games/MASTER/SPELLS.BAS | pass | no | 0 4.4s | - | timeout 6s | timeout 6.1s |  |
| creationix-basic-games/MENU.BAS | rt-error | no | 1 0.7s | - | no compile | no compile | ir compile: QIR: unsupported: line 6: statement BLOAD |
| creationix-basic-games/MISC/ARTIF.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/MISC/CFROGGER.BAS | pass | no | 0 5.4s | - | timeout 6s | timeout 6s |  |
| creationix-basic-games/MISC/CGAEXPL.BAS | pass | no | 0 4.3s | - | no compile | no compile | ir compile: QIR: unsupported: line 582: statement OUT |
| creationix-basic-games/MISC/CGAMINE.BAS | pass | no | 0 5.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 1: statement CLEAR |
| creationix-basic-games/MISC/CLOUD.BAS | pass | no | 0 1.5s | - | no compile | no compile | ir compile: QIR: unsupported: line 6: statement OUT |
| creationix-basic-games/MISC/CTETRIS.BAS | pass | no | 0 5.3s | - | timeout 6s | timeout 6.1s |  |
| creationix-basic-games/MISC/EXPLORER.BAS | pass | no | 0 4.3s | - | no compile | no compile | ir compile: QIR: unsupported: line 585: statement OUT |
| creationix-basic-games/MISC/FROGGER.BAS | pass | no | 0 5.4s | - | timeout 6.1s | timeout 6.1s |  |
| creationix-basic-games/MISC/GIFLOAD.BAS | rt-error | no | 1 0.8s | - | no compile | no compile | ir compile: QIR: unsupported: line 20: statement OUT |
| creationix-basic-games/MISC/HEXAGON.BAS | pass | no | 0 4.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 29: function STICK |
| creationix-basic-games/MISC/HEXMINE.BAS | pass | no | 0 5.2s | - | no compile | no compile | ir compile: QIR: unsupported: line 182: function STICK |
| creationix-basic-games/MISC/LAND2.BAS | pass | no | 0 1.5s | - | no compile | no compile | ir compile: QIR: unsupported: line 11: statement OUT |
| creationix-basic-games/MISC/LAND3.BAS | pass | no | 0 2.5s | - | 0 0.7s | 0 0.7s |  |
| creationix-basic-games/MISC/LANDSKP.BAS | pass | no | 0 1.4s | - | 0 0.1s | 0 0.2s |  |
| creationix-basic-games/MISC/MINESWEP.BAS | pass | no | 0 4.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 66: function STICK |
| creationix-basic-games/MISC/MOUSE.BAS | pass | no | 0 4.3s | - | 0 0.1s | 0 0.2s |  |
| creationix-basic-games/MISC/MULE.BAS | rt-error | no | 1 3.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 36: statement RUN |
| creationix-basic-games/MISC/MULTIPLE.BAS | pass | no | 0 4.7s | - | no compile | no compile | ir compile: QIR: unsupported: line 1: statement CLEAR |
| creationix-basic-games/MISC/RESCUE.BAS | interactive-only | no | timeout 6.2s | - | timeout 6.1s | timeout 6.2s |  |
| creationix-basic-games/MISC/SCOUTS.BAS | pass | no | 0 4.4s | - | 0 0.6s | 0 0.4s |  |
| creationix-basic-games/MISC/SHIPWAR.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/MISC/SPACE.BAS | compile-fail | no | no compile | - | no compile | no compile | [!!] QBNex :: Fatal [E0000]  Invalid UTF-8 byte sequence detected in source file (or UTF-16 without BOM) /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\tests\corpus-local\creationix-basic-games\misc\space.bas(1) /   [#] source; ir compile: [!!] QBNex :: Fatal [E0000]  Invalid UTF-8 byte sequence detected in source file (or UTF-16 without BOM) /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\tests\corpus-local\creationix-basic-games\misc\space.bas(1) /   [#] source |
| creationix-basic-games/MISC/TETRIS.BAS | pass | no | 0 4.3s | - | timeout 6.1s | timeout 6s |  |
| creationix-basic-games/MISC/TRIMINE.BAS | pass | no | 0 5.3s | - | no compile | no compile | ir compile: QIR: unsupported: line 1: statement CLEAR |
| creationix-basic-games/MOSS/MOSS.BAS | pass | no | 0 5.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 22: statement BLOAD |
| creationix-basic-games/MOSS/MOSSPIC.BAS | pass | no | 0 1.5s | - | no compile | no compile | ir compile: QIR: unsupported: line 20: statement BSAVE |
| creationix-basic-games/MOSS/MUSIC.BAS | diff | yes | 0 0.8s | timeout 6.1s | 0 0s | 0 0s | oracle exit timeout vs 0; vm differs: stdout; jit differs: stdout |
| creationix-basic-games/MOUSE.BAS | pass | no | 0 1.6s | - | 0 0s | 0 0s |  |
| creationix-basic-games/NUKE/CNUKE.BAS | pass | no | 0 4.8s | - | no compile | no compile | ir compile: QIR: unsupported: line 173: function STICK |
| creationix-basic-games/NUKE/CPICS.BAS | rt-error | no | 1 0.9s | - | no compile | no compile | ir compile: QIR: unsupported: line 153: statement BSAVE |
| creationix-basic-games/NUKE/NUKE.BAS | pass | no | 0 4.8s | - | no compile | no compile | ir compile: QIR: unsupported: line 171: function STICK |
| creationix-basic-games/NUKE/PICS.BAS | rt-error | no | 1 0.9s | - | no compile | no compile | ir compile: QIR: unsupported: line 5: statement OUT |
| creationix-basic-games/QUESTLND/BACK.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/QUESTLND/GRAPHICS.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/QUESTLND/QUEST.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/QUESTLND/QUESTLND.BAS | compile-fail | no | no compile | - | no compile | no compile | [!!] QBNex :: Fatal [E0000]  Missing subprogram: SUB COLORPALETTE is declared, but this program has no SUB COLORPALETTE ... END SUB; it may be in a Quick Library or another module /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\tests\corpus-local\creationix-basic-games\questlnd\questlnd.bas(11) /   [#] source; ir compile: [!!] QBNex :: Fatal [E0000]  Missing subprogram: SUB COLORPALETTE is declared, but this program has no SUB COLORPALETTE ... END SUB; it may be in a Quick Library or another module /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\tests\corpus-local\creationix-basic-games\questlnd\questlnd.bas(11) /   [#] source |
| creationix-basic-games/QUESTLND/SHIP.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/QUESTLND/TESTBACK.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/QUESTLND/TEST.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/QUESTLND/TEXTURE3.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/QUESTLND/TEXTURE4.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/RACE/RACE.BAS | interactive-only | no | timeout 6.1s | - | no compile | no compile | ir compile: QIR: unsupported: line 41: function STICK |
| creationix-basic-games/RPG2/CHARS.BAS | pass | no | 0 2s | - | no compile | no compile | ir compile: QIR: unsupported: line 144: statement OUT |
| creationix-basic-games/RPG2/CONVERT.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/RPG2/CSHAPES.BAS | rt-error | no | 1 0.8s | - | no compile | no compile | ir compile: QIR: unsupported: line 139: statement BSAVE |
| creationix-basic-games/RPG2/DMAPLAY.BAS | pass | no | 0 4.3s | - | no compile | no compile | ir compile: QIR: unsupported: line 69: VARPTR of a non-STRING variable |
| creationix-basic-games/RPG2/MAPEDIT.BAS | rt-error | no | 1 1.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 3: statement FILES |
| creationix-basic-games/RPG2/PARTY.BAS | pass | no | 0 1.7s | - | 0 0.6s | 0 0.6s |  |
| creationix-basic-games/RPG2/QMIDI.BAS | diff | yes | 0 0.8s | no compile | 0 0.1s | 0 0.1s | vm differs: stdout; jit differs: stdout; uses TIMER/RANDOMIZE: may be nondeterministic |
| creationix-basic-games/RPG2/RPG2.BAS | rt-error | no | 1 1.7s | - | no compile | no compile | ir compile: QIR: unsupported: line 200: VARPTR of a non-STRING variable |
| creationix-basic-games/RPG2/SAMPLE.BAS | rt-error | no | 1 0.7s | - | no compile | no compile | ir compile: QIR: unsupported: line 134: VARPTR of a non-STRING variable |
| creationix-basic-games/RPG2/SHAPES.BAS | rt-error | no | 1 0.7s | - | no compile | no compile | ir compile: QIR: unsupported: line 139: statement BSAVE |
| creationix-basic-games/RPG2/TEST.BAS | pass | no | 0 1.9s | - | no compile | no compile | ir compile: QIR: unsupported: line 9: statement OUT |
| creationix-basic-games/RPG2/TRANS.BAS | rt-error | no | 1 0.6s | - | no compile | no compile | ir compile: QIR: unsupported: line 80: statement BLOAD |
| creationix-basic-games/RPG3/BACK.BAS | pass | no | 0 4.5s | - | no compile | no compile | ir compile: QIR: unsupported: line 24: statement BSAVE |
| creationix-basic-games/RPG3/MOUSE.BAS | pass | no | 0 4.4s | - | 0 0s | 0 0.2s |  |
| creationix-basic-games/RPG3/PARTY.BAS | pass | no | 0 1.7s | - | 0 0.6s | 0 0.6s |  |
| creationix-basic-games/RPG3/SPELLS.BAS | pass | no | 0 5s | - | no compile | no compile | ir compile: QIR: unsupported: line 16: statement OUT |
| creationix-basic-games/RPG/CCONVERT.BAS | pass | no | 0 4.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 43: statement BSAVE |
| creationix-basic-games/RPG/CLIGHT.BAS | rt-error | no | 1 4.1s | - | no compile | no compile | ir compile: QIR: unsupported: line 307: statement RUN |
| creationix-basic-games/RPG/CONVERT.BAS | pass | no | 0 4.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 38: statement BSAVE |
| creationix-basic-games/RPG/ITEMS.BAS | pass | no | 0 4.3s | - | timeout 6.1s | timeout 6.1s |  |
| creationix-basic-games/RPG/LIGHT.BAS | rt-error | no | 1 4s | - | no compile | no compile | ir compile: QIR: unsupported: line 307: statement RUN |
| creationix-basic-games/RPG/ULTIMAX.BAS | rt-error | no | 1 3.2s | - | no compile | no compile | ir compile: QIR: unsupported: line 7: statement BLOAD |
| creationix-basic-games/RPG/WORLD.BAS | pass | no | 0 3.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 45: statement BLOAD |
| creationix-basic-games/SCORCH/CGA.BAS | pass | no | 0 5.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 47: statement BLOAD |
| creationix-basic-games/SCORCH/SCORC.BAS | timeout | no | timeout 6.1s | - | timeout 6.1s | timeout 6.1s |  |
| creationix-basic-games/SCORCH/SCORCH.BAS | pass | no | 0 4.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 47: statement BLOAD |
| creationix-basic-games/SHIP/SHIP.BAS | pass | no | 0 1.5s | - | 0 0.1s | 0 0.1s |  |
| creationix-basic-games/SHIP/TEXT.BAS | pass | no | 0 2.1s | - | 0 0.1s | 0 0.1s |  |
| creationix-basic-games/SORCERER/CMAKER.BAS | pass | no | 0 1.2s | - | 0 0.1s | 0 0.3s |  |
| creationix-basic-games/SORCERER/CSORCER.BAS | rt-error | no | 1 3s | - | no compile | no compile | ir compile: QIR: unsupported: line 116: statement RUN |
| creationix-basic-games/SORCERER/MAKER.BAS | pass | no | 0 1.2s | - | 0 0.1s | 0 0.3s |  |
| creationix-basic-games/SORCERER/SORCERER.BAS | rt-error | no | 1 4.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 110: statement RUN |
| creationix-basic-games/SORCERER/SORCER-G.BAS | pass | no | 0 1.2s | - | 0 0.1s | 0 0.1s |  |
| creationix-basic-games/TALK/TALK.BAS | pass | no | 0 4.3s | - | 1 6s | timeout 6.1s | vm (no window): exit 1 'tran' is not recognized as an internal or external command, / operable program or batch file. |
| creationix-basic-games/TALK/TOM.BAS | pass | no | 0 1.8s | - | 0 0.1s | 0 0.1s |  |
| creationix-basic-games/WARS/COOLMAP.BAS | pass | no | 0 1.5s | - | no compile | no compile | ir compile: QIR: unsupported: line 86: statement BSAVE |
| creationix-basic-games/WARS/MAPEDIT.BAS | pass | no | 0 4.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 158: statement BLOAD |
| creationix-basic-games/WARS/MOUSEWAR.BAS | rt-error | no | 1 2.5s | - | no compile | no compile | ir compile: QIR: unsupported: line 21: statement RUN |
| creationix-basic-games/WARS/WARS.BAS | rt-error | no | 1 2.2s | - | no compile | no compile | ir compile: QIR: unsupported: line 8: statement RUN |
| creationix-basic-games/ZENITH/BACKS.BAS | pass | no | 0 1.3s | - | no compile | no compile | ir compile: QIR: unsupported: line 43: statement BSAVE |
| creationix-basic-games/ZENITH/PLANETS.BAS | pass | no | 0 1.5s | - | no compile | no compile | ir compile: QIR: unsupported: line 17: statement BSAVE |
| creationix-basic-games/ZENITH/SHIPS.BAS | pass | no | 0 1.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 29: statement BSAVE |
| creationix-basic-games/ZENITH/SPACE.BAS | rt-error | no | 1 0.9s | - | 1 0s | 1 0.1s | vm (no window): exit 1 Unhandled Error #4 - qbnex_corpus_vm.exe: Line: 80 (in main module): Out of DATA; jit (no window): exit 1 Unhandled Error #4 - qbnex_corpus_jit.exe: Line: 80 (in main module): Out of DATA |
| creationix-basic-games/ZENITH/TEST.BAS | pass | no | 0 4.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 3: statement BLOAD |
| creationix-basic-games/ZENITH/ZENITH.BAS | rt-error | no | 1 1.3s | - | no compile | no compile | ir compile: QIR: unsupported: line 465: statement BLOAD |
| qb64-samples/programs/samples/misc/3dballs.bas | pass | no | 0 1.7s | - | 0 0.6s | 0 0.6s |  |
| qb64-samples/programs/samples/misc/3dexp2.bas | pass | no | 0 3.3s | - | no compile | no compile | ir compile: QIR: unsupported: line 165: statement OUT |
| qb64-samples/programs/samples/misc/acalc.bas | pass | no | 0 3s | - | 0 2.3s | 0 2.3s |  |
| qb64-samples/programs/samples/misc/aklabeth.bas | pass | no | 0 4s | - | timeout 6.1s | timeout 6.1s |  |
| qb64-samples/programs/samples/misc/ascipong.bas | pass | no | 0 3.5s | - | 0 2.3s | 0 2.3s |  |
| qb64-samples/programs/samples/misc/audio.bas | pass | no | 0 5.4s | - | 0 2.3s | 0 2.3s |  |
| qb64-samples/programs/samples/misc/cabsmous.bas | pass | no | 0 4.3s | - | no compile | no compile | ir compile: QIR: unsupported: line 243: VARPTR of a non-STRING variable |
| qb64-samples/programs/samples/misc/carols.bas | pass | no | 0 4.8s | - | timeout 6.3s | timeout 6.2s |  |
| qb64-samples/programs/samples/misc/chess.bas | pass | no | 0 4.2s | - | 1 0s | 1 0.2s | vm (no window): exit 1 Unhandled Error #52 - qbnex_corpus_vm.exe: Line: 61 (in main module): Bad file name or number; jit (no window): exit 1 Unhandled Error #52 - qbnex_corpus_jit.exe: Line: 61 (in main module): Bad file name or number |
| qb64-samples/programs/samples/misc/cuberot.bas | pass | no | 0 1.8s | - | no compile | no compile | ir compile: QIR: unsupported: line 13: statement WAIT |
| qb64-samples/programs/samples/misc/djsok.bas | pass | no | 0 3.2s | - | 0 2.3s | 0 2.4s |  |
| qb64-samples/programs/samples/misc/flrmp.bas | pass | no | 0 1.7s | - | 0 0.6s | 0 0.6s |  |
| qb64-samples/programs/samples/misc/forest.bas | compile-fail | no | no compile | - | no compile | no compile | [!!] QBNex :: Fatal [E0000]  Invalid UTF-8 byte sequence detected in source file (or UTF-16 without BOM) /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\tests\corpus-local\qb64-samples\programs\samples\misc\forest.bas(13) /   [#] source; ir compile: [!!] QBNex :: Fatal [E0000]  Invalid UTF-8 byte sequence detected in source file (or UTF-16 without BOM) /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\tests\corpus-local\qb64-samples\programs\samples\misc\forest.bas(13) /   [#] source |
| qb64-samples/programs/samples/misc/frac1.bas | pass | no | 0 3.4s | - | 0 2.3s | 0 2.3s |  |
| qb64-samples/programs/samples/misc/frac2.bas | pass | no | 0 1.4s | - | 0 0.1s | 0 0.1s |  |
| qb64-samples/programs/samples/misc/frac3.bas | pass | no | 0 3.4s | - | 0 2.3s | 0 2.5s |  |
| qb64-samples/programs/samples/misc/frog.bas | pass | no | 0 5.2s | - | no compile | no compile | ir compile: QIR: unsupported: line 333: statement WAIT |
| qb64-samples/programs/samples/misc/gor64.bas | pass | no | 0 5.1s | - | no compile | no compile | ir compile: QIR: unsupported: line 183: REDIM of an unknown array |
| qb64-samples/programs/samples/misc/gujero.bas | pass | no | 0 1.7s | - | no compile | no compile | ir compile: QIR: unsupported: line 10: statement OUT |
| qb64-samples/programs/samples/misc/intrprtr.bas | interactive-only | yes | timeout 6.1s | timeout 6.1s | 1 0.1s | 1 0.1s | vm exit 1 vs timeout; jit exit 1 vs timeout |
| qb64-samples/programs/samples/misc/kite.bas | pass | no | 0 4.4s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/misc/lissaj.bas | pass | no | 0 1.7s | - | 0 0.6s | 0 0.7s |  |
| qb64-samples/programs/samples/misc/mandala.bas | pass | no | 0 1.7s | - | 0 0.5s | 0 0.6s |  |
| qb64-samples/programs/samples/misc/mandelb.bas | pass | no | 0 1.7s | - | 0 0.5s | 0 0.6s |  |
| qb64-samples/programs/samples/misc/matrix.bas | interactive-only | yes | timeout 6s | timeout 6.1s | timeout 6.1s | timeout 6s |  |
| qb64-samples/programs/samples/misc/mclock.bas | pass | no | 0 1.7s | - | 0 0.6s | 0 0.6s |  |
| qb64-samples/programs/samples/misc/message.bas | interactive-only | no | timeout 6.1s | - | 0 2.3s | 0 2.3s |  |
| qb64-samples/programs/samples/misc/mzupd2.bas | pass | no | 0 4.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 46: statement TIME$ |
| qb64-samples/programs/samples/misc/nib64.bas | pass | no | 0 5.3s | - | 1 1.2s | 1 1.1s | vm (no window): exit 1 Unhandled Error #62 - qbnex_corpus_vm.exe: Line: 238 (in main module): Input past end of file; jit (no window): exit 1 Unhandled Error #62 - qbnex_corpus_jit.exe: Line: 238 (in main module): Input past end of file |
| qb64-samples/programs/samples/misc/pattern.bas | pass | no | 0 1.7s | - | no compile | no compile | ir compile: QIR: unsupported: line 6: statement WAIT |
| qb64-samples/programs/samples/misc/rc-ent6.bas | pass | no | 0 4.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 94: statement OUT |
| qb64-samples/programs/samples/misc/replicat.bas | diff | yes | 0 0.6s | timeout 6s | 0 0s | 0 0s | oracle exit timeout vs 0; vm differs: stdout; jit differs: stdout |
| qb64-samples/programs/samples/misc/ripples.bas | pass | no | 0 1.8s | - | no compile | no compile | ir compile: QIR: unsupported: line 47: statement OUT |
| qb64-samples/programs/samples/misc/rot13.bas | diff | yes | 0 0.7s | timeout 6s | 0 0s | 0 0s | oracle exit timeout vs 0; vm differs: stdout; jit differs: stdout |
| qb64-samples/programs/samples/misc/rotozoom.bas | pass | no | 0 1.7s | - | 0 0.6s | 0 0.6s |  |
| qb64-samples/programs/samples/misc/scramble.bas | pass | no | 0 3.5s | - | 0 2.3s | 0 2.3s |  |
| qb64-samples/programs/samples/misc/shoot.bas | pass | no | 0 5.3s | - | no compile | no compile | ir compile: QIR: unsupported: line 85: statement _MOUSEHIDE |
| qb64-samples/programs/samples/misc/sinecube.bas | pass | no | 0 3.4s | - | 0 2.2s | 0 2.3s |  |
| qb64-samples/programs/samples/misc/ssaver.bas | pass | no | 0 3.4s | - | 0 2.2s | 0 2.2s |  |
| qb64-samples/programs/samples/misc/strfld.bas | pass | no | 0 1.7s | - | 0 0.6s | 0 0.5s |  |
| qb64-samples/programs/samples/misc/temple.bas | pass | no | 0 4.7s | - | no compile | no compile | ir compile: QIR: unsupported: line 534: ON ( |
| qb64-samples/programs/samples/misc/tower.bas | pass | no | 0 3.4s | - | 0 2.3s | 0 2.3s |  |
| qb64-samples/programs/samples/misc/twirl2.bas | pass | no | 0 1.8s | - | 0 0.6s | 0 0.6s |  |
| qb64-samples/programs/samples/misc/vortex.bas | pass | no | 0 1.7s | - | no compile | no compile | ir compile: QIR: unsupported: line 9: statement OUT |
| qb64-samples/programs/samples/misc/wumpus.bas | interactive-only | yes | timeout 6s | timeout 6s | 1 0.1s | 1 0.4s | vm exit 1 vs timeout; jit exit 1 vs timeout |
| qb64-samples/programs/samples/misc/xwing.bas | pass | no | 0 4.8s | - | timeout 6s | timeout 6s |  |
| qb64-samples/programs/samples/n54/big/3dsviewer/3dsviewer.bas | pass | no | 0 3.3s | - | 1 0.8s | 1 0.8s | vm (no window): exit 1 Unhandled Error #9 - qbnex_corpus_vm.exe: Line: 304 (in main module): Subscript out of range; jit (no window): exit 1 Unhandled Error #9 - qbnex_corpus_jit.exe: Line: 304 (in main module): Subscript out of range |
| qb64-samples/programs/samples/n54/big/jpegmake.bas | pass | yes | 0 0.3s | timeout 6s | no compile | no compile | oracle exit timeout vs 0; ir compile: QIR: unsupported: line 553: VARPTR of a non-STRING variable |
| qb64-samples/programs/samples/n54/big/tictac.bas | pass | no | 0 4.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 334: statement OUT |
| qb64-samples/programs/samples/open_gl/3d_model_viewer.bas | compile-fail | no | no compile | - | no compile | no compile | [!!] QBNex :: Fatal [E0000]  SUB _GL (OpenGL) is not supported by this QBNex runtime (deviation D-14) /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\tests\corpus-local\qb64-samples\programs\samples\open_gl\3d_model_viewer.bas(158) /   [#] source; ir compile: [!!] QBNex :: Fatal [E0000]  SUB _GL (OpenGL) is not supported by this QBNex runtime (deviation D-14) /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\tests\corpus-local\qb64-samples\programs\samples\open_gl\3d_model_viewer.bas(158) /   [#] source |
| qb64-samples/programs/samples/open_gl/simple_example.bas | compile-fail | no | no compile | - | no compile | no compile | [!!] QBNex :: Fatal [E0000]  SUB _GL (OpenGL) is not supported by this QBNex runtime (deviation D-14) /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\tests\corpus-local\qb64-samples\programs\samples\open_gl\simple_example.bas(43) /   [#] source; ir compile: [!!] QBNex :: Fatal [E0000]  SUB _GL (OpenGL) is not supported by this QBNex runtime (deviation D-14) /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\tests\corpus-local\qb64-samples\programs\samples\open_gl\simple_example.bas(43) /   [#] source |
| qb64-samples/programs/samples/pete/25lines/3dcube25.bas | pass | no | 0 1.7s | - | 1 0.1s | 1 0.3s | vm (no window): exit 1 Unhandled Error #9 - qbnex_corpus_vm.exe: Line: 9 (in main module): Subscript out of range; jit (no window): exit 1 Unhandled Error #9 - qbnex_corpus_jit.exe: Line: 9 (in main module): Subscript out of range |
| qb64-samples/programs/samples/pete/25lines/npplasma.bas | pass | no | 0 1.8s | - | 0 0.6s | 0 0.6s |  |
| qb64-samples/programs/samples/pete/25lines/starfild.bas | pass | no | 0 1.8s | - | 0 0.6s | 0 0.6s |  |
| qb64-samples/programs/samples/pete/alyman.bas | pass | no | 0 5.5s | - | 1 0.1s | 1 0.1s | vm (no window): exit 1 Unhandled Error #5 - qbnex_corpus_vm.exe: Line: 122 (in main module): Illegal function call; jit (no window): exit 1 Unhandled Error #5 - qbnex_corpus_jit.exe: Line: 122 (in main module): Illegal function call |
| qb64-samples/programs/samples/pete/asciiconversion.bas | pass | no | 0 3.9s | - | 0 0.1s | 0 0.1s |  |
| qb64-samples/programs/samples/pete/averager.bas | interactive-only | yes | timeout 6.1s | timeout 6.1s | 1 0.1s | 1 0.1s | vm exit 1 vs timeout; jit exit 1 vs timeout |
| qb64-samples/programs/samples/pete/battleship.bas | rt-error | yes | 1 0.7s | timeout 6.2s | 1 0.1s | 1 0.1s | oracle exit timeout vs 1 |
| qb64-samples/programs/samples/pete/beatdown/bdt.bas | pass | no | 0 5.3s | - | 1 0.1s | 1 0.1s | vm (no window): exit 1 qb-exec: invalid program: QIR: main: `jmp L_TITLE`: undefined label L_TITLE; jit (no window): exit 1 qb-exec: invalid program: QIR: main: `jmp L_TITLE`: undefined label L_TITLE |
| qb64-samples/programs/samples/pete/beatdown/beatdown.bas | pass | no | 0 5.1s | - | timeout 6.1s | timeout 6s |  |
| qb64-samples/programs/samples/pete/beatdown/clickme.bas | pass | no | 0 1.5s | - | no compile | no compile | ir compile: QIR: unsupported: line 4: statement CHAIN |
| qb64-samples/programs/samples/pete/bj.bas | pass | no | 0 3.7s | - | 1 0.1s | 1 1s | vm (no window): exit 1 Unhandled Error #62 - qbnex_corpus_vm.exe: Line: 81 (in main module): Input past end of file; jit (no window): exit 1 Unhandled Error #62 - qbnex_corpus_jit.exe: Line: 68 (in main module): Input past end of file |
| qb64-samples/programs/samples/pete/booger/booger.bas | pass | no | 0 4.1s | - | no compile | no compile | ir compile: QIR: unsupported: line 168: statement WAIT |
| qb64-samples/programs/samples/pete/bush.bas | pass | no | 0 4.7s | - | 0 3s | 0 3s |  |
| qb64-samples/programs/samples/pete/calc_2.bas | pass | no | 0 3.7s | - | timeout 6.1s | timeout 6s |  |
| qb64-samples/programs/samples/pete/calc.bas | pass | no | 0 1.2s | - | 0 0.1s | 0 0.1s |  |
| qb64-samples/programs/samples/pete/carracingenglish.bas | pass | no | 0 4.9s | - | no compile | no compile | ir compile: QIR: unsupported: line 108: statement RUN |
| qb64-samples/programs/samples/pete/cc2.bas | rt-error | no | 1 0.9s | - | 1 0.1s | 1 0.1s | vm (no window): exit 1 Unhandled Error #53 - qbnex_corpus_vm.exe: Line: 10 (in main module): File not found; jit (no window): exit 1 Unhandled Error #53 - qbnex_corpus_jit.exe: Line: 10 (in main module): File not found |
| qb64-samples/programs/samples/pete/coolscr.bas | pass | no | 0 2.3s | - | 0 0.6s | 0 0.7s |  |
| qb64-samples/programs/samples/pete/cr/cr.bas | pass | no | 0 4.8s | - | timeout 6.1s | timeout 6s |  |
| qb64-samples/programs/samples/pete/darokin.bas | pass | no | 0 1.1s | - | 0 0.6s | 0 0.6s |  |
| qb64-samples/programs/samples/pete/darpong/darpong.bas | pass | no | 0 4.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 121: statement WAIT |
| qb64-samples/programs/samples/pete/degrees.bas | pass | no | 0 1.2s | - | 0 0.1s | 0 0.1s |  |
| qb64-samples/programs/samples/pete/diceit.bas | pass | no | 0 0.9s | - | 0 0.1s | 0 0.1s |  |
| qb64-samples/programs/samples/pete/drug3d.bas | pass | no | 0 4.6s | - | timeout 6.1s | timeout 6.1s |  |
| qb64-samples/programs/samples/pete/fight.bas | pass | no | 0 2.8s | - | 1 0.1s | 1 0.1s | vm (no window): exit 1 Unhandled Error #52 - qbnex_corpus_vm.exe: Line: 110 (in main module): Bad file name or number; jit (no window): exit 1 Unhandled Error #52 - qbnex_corpus_jit.exe: Line: 110 (in main module): Bad file name or number |
| qb64-samples/programs/samples/pete/gravity.bas | pass | no | 0 1.4s | - | 0 0.6s | 0 0.6s |  |
| qb64-samples/programs/samples/pete/hgm30e/hgm30e.bas | rt-error | no | 1 2.9s | - | 1 0.8s | 1 1.2s | vm (no window): exit 1 Unhandled Error #52 - qbnex_corpus_vm.exe: Line: 173 (in main module): Bad file name or number; jit (no window): exit 1 Unhandled Error #52 - qbnex_corpus_jit.exe: Line: 173 (in main module): Bad file name or number |
| qb64-samples/programs/samples/pete/html/test.bas | pass | no | 0 4.4s | - | 1 0.1s | 1 0.1s | vm (no window): exit 1 Unhandled Error #70 - qbnex_corpus_vm.exe: Line: 13 (in main module): Permission denied; jit (no window): exit 1 Unhandled Error #70 - qbnex_corpus_jit.exe: Line: 13 (in main module): Permission denied |
| qb64-samples/programs/samples/pete/invader1/invader.bas | pass | no | 0 2.8s | - | no compile | no compile | ir compile: QIR: unsupported: line 147: statement WAIT |
| qb64-samples/programs/samples/pete/kbdparse.bas | pass | no | 0 1.8s | - | 0 0.6s | 0 0.6s |  |
| qb64-samples/programs/samples/pete/lineswin.bas | pass | no | 0 3.7s | - | timeout 6.1s | timeout 6.1s |  |
| qb64-samples/programs/samples/pete/makebig.bas | pass | no | 0 1.1s | - | 0 0.1s | 0 0.1s |  |
| qb64-samples/programs/samples/pete/monopoly/monopoly.bas | pass | no | 0 5.1s | - | timeout 6.1s | timeout 6.1s |  |
| qb64-samples/programs/samples/pete/mooncr/mooncr.bas | pass | no | 0 3.9s | - | no compile | no compile | ir compile: QIR: unsupported: line 168: function INP |
| qb64-samples/programs/samples/pete/move.bas | pass | no | 0 4.7s | - | 1 0.9s | 1 0.9s | vm (no window): exit 1 Unhandled Error #62 - qbnex_corpus_vm.exe: Line: 67 (in main module): Input past end of file; jit (no window): exit 1 Unhandled Error #62 - qbnex_corpus_jit.exe: Line: 67 (in main module): Input past end of file |
| qb64-samples/programs/samples/pete/mrguessit.bas | pass | no | 0 3.8s | - | 1 0.1s | 1 0.1s | vm (no window): exit 1 Unhandled Error #62 - qbnex_corpus_vm.exe: Line: 9 (in main module): Input past end of file; jit (no window): exit 1 Unhandled Error #62 - qbnex_corpus_jit.exe: Line: 9 (in main module): Input past end of file |
| qb64-samples/programs/samples/pete/nlcm2006/nlcm.bas | pass | no | 0 4.3s | - | 1 1.6s | 1 3.8s | vm (no window): exit 1 Unhandled Error #5 - qbnex_corpus_vm.exe: Line: 190 (in main module): Illegal function call; jit (no window): exit 1 Unhandled Error #5 - qbnex_corpus_jit.exe: Line: 190 (in main module): Illegal function call |
| qb64-samples/programs/samples/pete/optimus/deedsaks.bas | pass | no | 0 4.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 363: statement WAIT |
| qb64-samples/programs/samples/pete/pall2.bas | pass | no | 0 1.1s | - | 0 0.6s | 0 0.6s |  |
| qb64-samples/programs/samples/pete/picdimreader.bas | rt-error | no | 1 0.8s | - | 0 0.1s | 0 0.1s |  |
| qb64-samples/programs/samples/pete/pongsource.bas | pass | no | 0 2.8s | - | 0 2.3s | 0 2s |  |
| qb64-samples/programs/samples/pete/pongtennis/tennis.bas | pass | no | 0 3.7s | - | no compile | no compile | ir compile: QIR: unsupported: line 344: unknown name in expression: changeAmount |
| qb64-samples/programs/samples/pete/pp256/usersubs.bas | pass | no | 0 3.9s | - | no compile | no compile | ir compile: QIR: unsupported: line 456: statement WAIT |
| qb64-samples/programs/samples/pete/qbb.bas | pass | no | 0 0.8s | - | 0 0.1s | 0 0.1s |  |
| qb64-samples/programs/samples/pete/rats1.bas | pass | no | 0 4.9s | - | 1 0.4s | 1 0.4s | vm (no window): exit 1 Unhandled Error #62 - qbnex_corpus_vm.exe: Line: 41 (in main module): Input past end of file; jit (no window): exit 1 Unhandled Error #62 - qbnex_corpus_jit.exe: Line: 41 (in main module): Input past end of file |
| qb64-samples/programs/samples/pete/raytra1b.bas | pass | no | 0 1.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 24: statement OUT |
| qb64-samples/programs/samples/pete/rect2.bas | pass | no | 0 3.9s | - | timeout 6.1s | timeout 6s |  |
| qb64-samples/programs/samples/pete/rectong/rectong.bas | pass | no | 0 4.3s | - | no compile | no compile | ir compile: QIR: unsupported: line 522: statement WAIT |
| qb64-samples/programs/samples/pete/roboraid/roboraid.bas | pass | no | 0 5.4s | - | timeout 6.1s | timeout 6s |  |
| qb64-samples/programs/samples/pete/rotate.bas | pass | no | 0 1s | - | 0 0.6s | 0 0.6s |  |
| qb64-samples/programs/samples/pete/rug.bas | pass | no | 0 1.2s | - | 0 0.6s | 0 0.6s |  |
| qb64-samples/programs/samples/pete/screen.bas | pass | no | 0 1.2s | - | 0 0.1s | 0 0.1s |  |
| qb64-samples/programs/samples/pete/simpire/ssimpire.bas | pass | no | 0 5.6s | - | no compile | no compile | ir compile: QIR: unsupported: line 1261: statement BLOAD |
| qb64-samples/programs/samples/pete/skydiver.bas | pass | no | 0 2.8s | - | no compile | no compile | ir compile: QIR: unsupported: line 84: statement WAIT |
| qb64-samples/programs/samples/pete/space1.bas | pass | no | 0 4.8s | - | timeout 6.1s | timeout 6.1s |  |
| qb64-samples/programs/samples/pete/stones.bas | pass | no | 0 3.9s | - | 0 0.1s | 0 0.1s |  |
| qb64-samples/programs/samples/pete/su2/shoote.bas | pass | no | 0 3.4s | - | 0 2.3s | 0 2.3s |  |
| qb64-samples/programs/samples/pete/su2/su2t.bas | pass | no | 0 4.3s | - | 0 3.3s | 0 3.3s |  |
| qb64-samples/programs/samples/pete/sudoku.bas | pass | no | 0 1.2s | - | no compile | no compile | ir compile: QIR: unsupported: line 13: statement RUN |
| qb64-samples/programs/samples/pete/tank/tcv2.bas | pass | no | 0 3.3s | - | no compile | no compile | ir compile: QIR: unsupported: line 186: statement BLOAD |
| qb64-samples/programs/samples/pete/time.bas | pass | no | 0 1.1s | - | 0 0.6s | 0 0.6s |  |
| qb64-samples/programs/samples/pete/torndc.bas | pass | no | 0 4.9s | - | timeout 6.1s | timeout 6.1s |  |
| qb64-samples/programs/samples/pete/tor/qbdemo.bas | pass | no | 0 4.3s | - | no compile | no compile | ir compile: QIR: unsupported: line 24: statement OUT |
| qb64-samples/programs/samples/pete/upperlowerconversion.bas | compile-fail | no | no compile | - | no compile | no compile | [!!] QBNex :: Fatal [E1202]  'LCASE' is already defined /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\tests\corpus-local\qb64-samples\programs\samples\pete\upperlowerconversion.bas(5,5) /   [#] source; ir compile: [!!] QBNex :: Fatal [E1202]  'LCASE' is already defined /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\tests\corpus-local\qb64-samples\programs\samples\pete\upperlowerconversion.bas(5,5) /   [#] source |
| qb64-samples/programs/samples/pete/vektor.bas | pass | no | 0 3.9s | - | timeout 6.1s | timeout 6.1s |  |
| qb64-samples/programs/samples/pete/vsphere.bas | pass | no | 0 1.1s | - | 0 0.1s | 0 0.1s |  |
| qb64-samples/programs/samples/pete/wetspot/wetspot.bas | pass | no | 0 4.5s | - | no compile | no compile | ir compile: QIR: unsupported: line 714: statement BLOAD |
| qb64-samples/programs/samples/pete/zodiac4.bas | pass | no | 0 4.7s | - | timeout 6s | timeout 6s |  |
| qb64-samples/programs/samples/qb45com/action/4pong.bas | pass | no | 0 4.7s | - | 0 2.6s | 0 2.7s |  |
| qb64-samples/programs/samples/qb45com/action/arcdemo/arcdemo.bas | pass | no | 0 3.9s | - | timeout 6.1s | timeout 6s |  |
| qb64-samples/programs/samples/qb45com/action/arqanoid/arqanoid.bas | pass | no | 0 5s | - | no compile | no compile | ir compile: QIR: unsupported: line 502: statement WAIT |
| qb64-samples/programs/samples/qb45com/action/arqanoid/lvldesgn.bas | pass | no | 0 4.1s | - | no compile | no compile | ir compile: QIR: unsupported: line 604: function INP |
| qb64-samples/programs/samples/qb45com/action/assault/assault.bas | compile-fail | no | no compile | - | no compile | no compile | [!!] QBNex :: Fatal [E0000]  Expected element-name AS type, AS type element-list, or END TYPE /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\tests\corpus-local\qb64-samples\programs\samples\qb45com\action\assault\assault.bas(84) /   [#] source; ir compile: [!!] QBNex :: Fatal [E0000]  Expected element-name AS type, AS type element-list, or END TYPE /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\tests\corpus-local\qb64-samples\programs\samples\qb45com\action\assault\assault.bas(84) /   [#] source |
| qb64-samples/programs/samples/qb45com/action/astrowars.bas | pass | no | 0 4.3s | - | 1 0s | 1 0s | vm (no window): exit 1 qb-exec: invalid program: QIR: main: `onerror L_ERRCODE`: undefined label L_ERRCODE; jit (no window): exit 1 qb-exec: invalid program: QIR: main: `onerror L_ERRCODE`: undefined label L_ERRCODE |
| qb64-samples/programs/samples/qb45com/action/sfb2/sfb2.bas | pass | no | 0 4.9s | - | no compile | no compile | ir compile: QIR: unsupported: line 439: statement CLEAR |
| qb64-samples/programs/samples/thebob/abacus/a12gfx.bas | pass | no | 0 1.3s | - | no compile | no compile | ir compile: QIR: unsupported: line 76: statement BSAVE |
| qb64-samples/programs/samples/thebob/abacus/abacus12.bas | rt-error | no | 1 1s | - | no compile | no compile | ir compile: QIR: unsupported: line 31: statement BLOAD |
| qb64-samples/programs/samples/thebob/animax/animax.bas | rt-error | no | 1 0.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 78: statement BLOAD |
| qb64-samples/programs/samples/thebob/animax/axgfx.bas | pass | no | 0 1.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 31: statement BSAVE |
| qb64-samples/programs/samples/thebob/biochart/bcgfx.bas | pass | no | 0 1.7s | - | no compile | no compile | ir compile: QIR: unsupported: line 58: statement BSAVE |
| qb64-samples/programs/samples/thebob/biochart/biochart.bas | rt-error | no | 1 0.9s | - | no compile | no compile | ir compile: QIR: unsupported: line 23: statement BLOAD |
| qb64-samples/programs/samples/thebob/chess/chess.bas | pass | no | 0 1.7s | - | no compile | no compile | ir compile: QIR: unsupported: line 13: statement BSAVE |
| qb64-samples/programs/samples/thebob/chess/chesssubs.bas | rt-error | no | 1 1s | - | no compile | no compile | ir compile: QIR: unsupported: line 195: statement BLOAD |
| qb64-samples/programs/samples/thebob/chopper/chopfile.bas | pass | no | 0 4.1s | - | no compile | no compile | ir compile: QIR: unsupported: line 34: statement OUT |
| qb64-samples/programs/samples/thebob/chopper/chopper.bas | pass | no | 0 4.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 100: statement BLOAD |
| qb64-samples/programs/samples/thebob/kong/k1.bas | pass | no | 0 1.1s | - | no compile | no compile | ir compile: QIR: unsupported: line 78: statement BSAVE |
| qb64-samples/programs/samples/thebob/kong/k2.bas | rt-error | no | 1 1s | - | no compile | no compile | ir compile: QIR: unsupported: line 22: statement OUT |
| qb64-samples/programs/samples/thebob/kong/kong.bas | rt-error | no | 1 0.8s | - | no compile | no compile | ir compile: QIR: unsupported: line 81: statement BLOAD |
| qb64-samples/programs/samples/thebob/leapfrog/leapfrog.bas | rt-error | no | 1 1.9s | - | no compile | no compile | ir compile: QIR: unsupported: line 69: statement OUT |
| qb64-samples/programs/samples/thebob/leapfrog/lfgx.bas | pass | no | 0 1.7s | - | no compile | no compile | ir compile: QIR: unsupported: line 113: statement BSAVE |
| qb64-samples/programs/samples/thebob/minigolf/mg.bas | pass | no | 0 2.7s | - | no compile | no compile | ir compile: QIR: unsupported: line 225: statement WAIT |
| qb64-samples/programs/samples/thebob/monopoly/mboard.bas | pass | no | 0 3s | - | no compile | no compile | ir compile: QIR: unsupported: line 83: statement WAIT |
| qb64-samples/programs/samples/thebob/pongg/pgfx.bas | pass | no | 0 1.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 144: statement BSAVE |
| qb64-samples/programs/samples/thebob/pongg/pongg.bas | rt-error | no | 1 0.8s | - | no compile | no compile | ir compile: QIR: unsupported: line 132: statement WAIT |
| qb64-samples/programs/samples/thebob/rattler/rattler.bas | pass | no | 0 3.8s | - | no compile | no compile | ir compile: QIR: unsupported: line 129: statement OUT |
| qb64-samples/programs/samples/thebob/sol/s3gfx.bas | pass | no | 0 1.5s | - | no compile | no compile | ir compile: QIR: unsupported: line 95: statement BSAVE |
| qb64-samples/programs/samples/thebob/sol/sol3.bas | rt-error | no | 1 1.7s | - | no compile | no compile | ir compile: QIR: unsupported: line 88: statement BLOAD |
| creationix-basic-games/BOMB/VECTOCAT.BAS | pass | no | 0 5.1s | - | no compile | no compile | ir compile: QIR: unsupported: line 138: statement RUN |
| creationix-basic-games/BOMB/WARZONE.BAS | rt-error | no | 1 3.3s | - | no compile | no compile | ir compile: QIR: unsupported: line 104: statement RUN |
| creationix-basic-games/CONQUEST/CONQUEST.BAS | rt-error | no | 1 2s | - | no compile | no compile | ir compile: QIR: unsupported: line 244: statement RUN |
| creationix-basic-games/MAGIC/MAGIC.BAS | pass | no | 0 5.6s | - | timeout 6.1s | timeout 6.1s |  |


