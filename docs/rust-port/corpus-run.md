# Corpus run: real programs, run headless

Real BASIC programs are compiled **and run**, through every backend, and their outputs are compared.
Runner: [`tests/corpus-run/run.ps1`](../../tests/corpus-run/run.ps1), inputs in `tests/corpus-run/inputs/`.

## Setup (run 2, 2026-11-11)

- Build: branch `claude/run-tests-2` = rust-port 989c3c3 (qb-exec with a window, Q1-Q8 and C1/C2 fixed) + runner changes.
  `qb.exe` from `scripts\setup_win.cmd` (CARGO_BUILD_JOBS=1), `qb-exec` from `cargo build --release -p qb-exec`,
  copied to `crates\engine\exec\bin\windows` (no native kit, so `--ir` writes the runner with the QIR appended).
- Corpus (`tests/corpus-local`, gitignored): creationix/basic-games (129 files) and QB64Team/qb64 **tag v1.4**
  `programs/samples` (148 files; the current HEAD of that repository no longer ships the samples). 277 `.bas` files.
- Variants per program (`-Variants cpp,vm,jit`; the oracle was not run this time):
  - **cpp**: `qb file.bas` (C++ codegen + Rust qb-runtime). The reference result.
  - **vm**: `qb file.bas --ir` (QIR), run with `QB_JIT=off`. **jit**: the same executable with `QB_JIT=eager`.
- Every run uses a fresh copy of the program folder, with the exe placed next to the data (QB64 executables
  chdir to their own folder). QB64 samples run from a QB64-root layout (they `CHDIR ".\programs\samples\..."`).
  Each run gets `QBNEX_HEADLESS=1`, `QBNEX_NO_ERROR_DIALOG=1`, stdin from `inputs/<id>.in` (default `_default.in`),
  keyboard events from `inputs/_default.input` (GUI programs; it ends with `close`), and a 6 s timeout.
  No window was opened on screen.
- Compared: stdout (normalised) and exit code, between runs that both finished. GUI programs are now compared too
  (qb-exec has the window). The framebuffer capture is only noted ("framebuffer differs", 116 programs): it holds every
  `_DISPLAY` frame, so its length depends on timing, and it never changes the status.
- Runner workaround for **Q9** (below): a console-only program's QIR gets `meta console only` added (what the C++ build
  got) and is repacked with `scripts/qir_pack.ps1`; such rows carry the note `ir: meta console only added (Q9)` (12 programs).
  Without it every console program opens a (headless) window under qb-exec, prints nothing to stdout and waits at END.

Reproduce: `pwsh tests\corpus-run\run.ps1 -Variants cpp,vm,jit -Timeout 6` (`-Resume` continues a stopped run;
`-Filter 'cx__MISC__*'` selects programs). Results: `target\corpus-run\results.{json,md}` plus per-program logs.
Do not compile anything else with the same `qb.exe` while it runs: `internal\temp` (and `program.qir`) is shared.

## Summary (277 programs)

| status | count | meaning |
|---|---|---|
| pass | 192 | cpp exited 0 with no runtime error; vm and jit gave the same stdout and exit code (or could not be compared, see below) |
| rt-error | 40 | cpp stopped with a BASIC runtime error: missing data files, DOS paths, input-driven (run-1 triage, still true) |
| diff | 18 | vm/jit differ from cpp: bugs Q10-Q17 below |
| binary-format | 13 | QuickBASIC fast-load binary files, not source |
| interactive-only | 7 | still waiting for input at the timeout (games / console prompts) |
| compile-fail | 5 | cpp does not compile: 2 `_GL` (D-14), 1 missing SUB (QUESTLND), 1 `TYPE AS` (assault), 1 binary file (MISC/SPACE) |
| timeout | 2 | SOUND loops end 2.6 s after the scripted `close` under cpp (O1); vm/jit exit 0 there |

**QIR (`--ir`):** 241 of 264 programs compile (run 1: 133 of 277; the 13 binary files are not compiled).
The 23 that do not: 5 that cpp rejects too, 16 unsupported statements (Q18), and `CASE IS >= name` / `ON (expr) GOTO` (Q19, Q20).

| | finished with cpp | same stdout + exit as cpp |
|---|---|---|
| vm | 232 | **210** |
| jit | 231 | **209** |
| vm vs jit | 235 | 230 |

"Finished" = neither run hit the 6 s timeout. Of the 22 vm mismatches, 18 have a minimal repro (Q10, Q11, Q12, Q13, Q14,
Q15, Q16), 1 is timing (RPG/LIGHT), 3 are not minimised (FOWLER and pete/pp256/usersubs, likely Q10; ZENITH, likely Q12).
vm and jit disagree on 5 programs: the 3 C/QUICK editors (Q11: the amount of text that reaches stdout before the close
depends on speed), RPG/LIGHT (timing), and optimus/deedsaks (Q17, JIT crash). sfb2 times out under the JIT only (Q17).

## Bugs fixed here (qb-runtime, test-first)

**R1. END "Press any key to continue" and SLEEP spin forever when stdin is not a console** (run 1, commit f3c381a).
`_CONSOLEINPUT` (`func__getconsoleinput`) returned 0 forever when the input handle is a file or pipe:
`ReadConsoleInputA` fails and the zeroed record reads as "no event". The END wait and `SLEEP` (no argument) looped at 100% CPU.
Without a console there is no keyboard to wait for, so `console_input` now reports a key event at once.
Test: `console::stdio::console_input_tests`.

## Open bugs (run 2; for the compiler / qb-exec sessions)

Each repro is a whole program. "cpp" is `qb x.bas`, "vm"/"jit" is `qb x.bas --ir` with `QB_JIT=off`/`eager`,
all run with `QBNEX_HEADLESS=1` and stdin from NUL. Console repros (Q10-Q14, Q18, Q19) need the Q9 workaround (or the Q9 fix)
to show their output under vm/jit.

**Q9. compiler: QIR of an automatic console-only program has no `meta console only`.** (all 12 console programs)
```basic
PRINT 5
```
cpp: auto `$CONSOLE:ONLY` build, prints `5`, exits. vm/jit: qb-exec opens a window (headless: nothing on stdout) and END
waits for a key until killed. Cause: `IrFinish` is called at `compiler/qbnex.bas:10863` (writes `program.qir`), but the
automatic console-only decision (`AutoConsoleOnlyActive`, `DEPENDENCY_CONSOLE_ONLY = 1`) is made later, at line ~11121.
Not seen in run 1 because qb-exec had no window then. Fix: decide auto console-only before `IrFinish` (or emit the meta
from the same condition).

**Q10. qb-exec memory mode: implicit arrays and DIMs that did not run are not allocated.** (error 9: LOGATH, CLOGATH,
VECTOCAT, misc/forest, pete/25lines/3dcube25, misc/mzupd2; WARZONE gets error 9 instead of cpp's error 76)
A program that needs real memory (`GET`/`PUT` of a whole array, `VARPTR`, `DEF SEG`, ...) is lowered to the memory model
(`mem.rs`), where `aaddr` of an array without data is error 9: there is no QBasic implicit `DIM name(10)` as in the
register model (`vm.rs` `arr_at`), and a static array whose `DIM` statement did not run has no data either.
```basic
EV(1) = 1: PRINT EV(1)
SYSTEM
DIM b1%(480)
GET (1, 1)-(32, 30), b1%
```
cpp: `1`. vm/jit: error 9 at line 1. Without the (never executed) GET line it prints `1`.
```basic
IF C& = 0 THEN PRINT "first" ELSE DIM M%(8)
M%(3) = 7: PRINT M%(3)
SYSTEM
GET (1, 1)-(2, 2), M%
```
cpp: `first 7` (`DIM M%(8)` with constant bounds is static: allocated at compile time). vm/jit: `first`, then error 9 at line 2.
FOWLER (error 9 at line 95, `l(x, y + 1)` on `DIM l(25, 201)` after `DEF SEG = VARSEG(l(0, 0))`) and pete/pp256/usersubs
(error 9 at line 1140, a SUB's local `DIM Temp(1 TO 1000)`) also run in memory mode; not minimised, check them after the fix.

**Q11. qb-exec: `PRINT USING` in a GUI program writes to stdout, not to the window.** (C/QUICK, C/QC, C/CQUICK, pete/monopoly)
```basic
SCREEN 12
PRINT USING "###"; 5
PRINT "plain"
SYSTEM
```
cpp: nothing on stdout (both lines go to the screen). vm/jit: stdout `  5`; `plain` goes to the window.
The QUICK editors write ~20 MB of `PRINT USING "³        ####:"` status lines to stdout in 3.5 s.

**Q12. qb-exec: the INT 33h mouse driver called through `CALL ABSOLUTE` reports no mouse.** (thebob/leapfrog, thebob/pongg,
thebob/chess/chesssubs: they print "no mouse" and end after 0.5 s with exit 0, so cpp's later error 53 is never reached;
ZENITH has its own MouseInit% and also differs, not checked)
```basic
DIM SHARED MouseDATA$
DATA 55,89,E5,8B,5E,0C,8B,07,50,8B,5E,0A,8B,07,50,8B,5E,08,8B
DATA 0F,8B,5E,06,8B,17,5B,58,1E,07,CD,33,53,8B,5E,0C,89,07,58
DATA 8B,5E,0A,89,07,8B,5E,08,89,0F,8B,5E,06,89,17,5D,CA,08,00
MouseDATA$ = SPACE$(57)
FOR i = 1 TO 57
READ h$
Hexxer$ = CHR$(VAL("&H" + h$))
MID$(MouseDATA$, i, 1) = Hexxer$
NEXT i
LB% = 0: RB% = 0: mX% = 0: mY% = 0
DEF SEG = VARSEG(MouseDATA$)
Mouse = SADD(MouseDATA$)
CALL ABSOLUTE(LB%, RB%, mX%, mY%, Mouse)
PRINT "mouse:"; LB%
SYSTEM
```
cpp: `mouse:-1` (driver present, from its x86 emulator). vm/jit: `mouse: 0`.

**Q13. qb-exec: `CALL ABSOLUTE` that fails leaks its argument space; the program panics.** (n54/big/tictac)
```basic
DIM Mx AS STRING: Mx = SPACE$(57)
ON ERROR GOTO h
FOR i& = 1 TO 300000
  CALL ABSOLUTE(2, click%, cH%, cV%, SADD(Mx))
NEXT
PRINT "done"
SYSTEM
h: RESUME NEXT
```
cpp: exit 86 "X86 Error: Unknown Opcode" (its emulator stops the program). vm/jit: panic
`crates\engine\exec\src\rt\builtins.rs:484:29: range end index 1114113 out of range for slice of length 1114112`
(exit 0xC0000409). `ABSARG.*` advances `rt.cmem_next` for every argument and nothing resets it when the call fails;
it needs a bounds check (error 7) and a reset per call.

**Q14. qb-exec: `DIM` inside a `SUB ... STATIC` gives "Duplicate definition" on the second call.** (qb45com/arqanoid)
```basic
S
S
PRINT "ok"
SYSTEM
SUB S STATIC
DIM E(7): E(1) = E(1) + 1: PRINT E(1);
END SUB
```
cpp: `1  2 ok`. vm/jit: `1`, then error 10 at line 6. (The message also says "in main module" for a line inside a SUB.)

**Q15. qb-exec: ERASE of a static array passed as a parameter frees it.** (n54/big/3dsviewer)
```basic
DIM t(3, 3)
t(1, 1) = 5
clr t()
t(1, 1) = t(1, 1) + 1
PRINT t(1, 1)
SYSTEM
SUB clr (a())
ERASE a
END SUB
```
cpp: `1` (ERASE clears a static array). vm/jit: error 9 at line 4. The Q6 fix covers `ERASE` of the array itself only;
in the SUB the emitter does not know the argument is static.

**Q16. qb-exec: `LOCATE` past row 25 after printing on row 30.** (pete/alyman, pete/nlcm2006/nlcm)
```basic
ON ERROR GOTO h
COLOR 3, 0: CLS
LOCATE 30, 28: r1 = CSRLIN
PRINT "a"
LOCATE 32, 28: r2 = CSRLIN
PRINT "b"
OPEN "r16.txt" FOR OUTPUT AS #1: PRINT #1, r1; r2; e: CLOSE #1
SYSTEM
h: e = ERR: RESUME NEXT
```
r16.txt: cpp `30  32  0`; vm `30  24  5` (error 5 at the second LOCATE). Neither is QBasic (a 25-row SCREEN 0 gives error 5
at `LOCATE 30` already); the backends should at least agree. The programs were written for a 50-row screen and then
stop under vm/jit only.

**Q17. JIT only: optimus/deedsaks crashes (0xC0000005), qb45com/action/sfb2 runs to the timeout.** Both reproducible
(`run.ps1 -Filter '*deedsaks*'` / `'*sfb2*'`); vm exits 0 like cpp. Not minimised.

**Q18. QIR emitter: unsupported statements** (programs that do not compile with `--ir`): `STRIG` 11 (BOMBMAN/BOMB,
GIRLS/WAR, MISC/CGAMINE, HEXAGON, HEXMINE, MINESWEP, TRIMINE, SORCERER, CSORCER, WARS, MOUSEWAR), `ON KEY` 3 (MISC/MULTIPLE,
SCORCH, CGA), `FILES` 1 (RPG2/MAPEDIT), `_MOUSEHIDE` 1 (misc/shoot).

**Q19. QIR emitter: a variable whose first appearance is in `CASE IS`.** (pete/pongtennis/tennis)
```basic
x = 5
SELECT CASE x
CASE IS >= lim: PRINT "ge"
CASE ELSE: PRINT "lt"
END SELECT
lim = 3
SYSTEM
```
cpp: `ge`. `--ir`: "QIR: unsupported: line 3: unknown name in expression: lim".

**Q20. QIR emitter: `ON (expr) GOTO`** with the expression in parentheses. (misc/temple)
```basic
ST = 0
ON (1 - (ST < 1)) GOTO 10, 20
10 PRINT "ten": SYSTEM
20 PRINT "twenty": SYSTEM
```
cpp: `twenty`. `--ir`: "QIR: unsupported: line 2: ON (".

## Run-1 bugs: status

Run 1 (2026-11-10, rust-port b97e32b; qb-exec without a window) found Q1-Q8 and C1, C2. Fixed and merged in rust-port
989c3c3; checked again in this run:
- Q1 END prompt (ANT, MUSIC, QMIDI, replicat, rot13), Q5 `INPUT$(1)` (VNSTALL, misc/chess, fight, hgm30e): same as cpp now.
  Q3/Q4 (averager, wumpus, intrprtr): all three backends wait for input until the timeout, so they are not compared; battleship same as cpp.
- Q2 window / input script: GUI programs now run in qb-exec's (headless) window and end on the scripted `close`;
  202 of the 224 compared GUI programs give the same stdout and exit code as cpp.
- Q6 ERASE of a static array: fixed for the array itself; the parameter case is Q15.
- Q7 GOTO label named like a SUB: bdt and astrowars compile and run.
- Q8 unsupported statements (BLOAD, OUT, BSAVE, RUN, WAIT, STICK, VARPTR, CLEAR, TIME$, INP, CHAIN, REDIM): all compile now;
  what is left is Q18.
- C1 CP437 sources: misc/forest compiles. MISC/SPACE.BAS is really binary (30 NUL bytes in 11.9 KB, no text), so its
  "File contains binary data" is right.
- C2 `LCASE` as a SUB name: pete/upperlowerconversion compiles.

## Observations (not bugs)

- **O1.** In a loop of short `SOUND` calls, a window `close` takes ~2.6 s to end the cpp program (SCORCH/SCORC,
  BOMB/BOMBMAN), beyond the 6 s timeout; the vm/jit builds end at once.
- **GET "Illegal function call"** in BANANA/PICS, NUKE/PICS, NUKE/CPICS, RPG2/CSHAPES, RPG2/SHAPES: the programs move
  `WINDOW SCREEN (0 - sx, 0 - sy)-(639 - sx, 479 - sy)` by 40 px per sprite, so with sx = 600 `GET (1,1)-(40,40)` maps to x = 640,
  off screen. QB64 v1.4 computes exactly the same, so this is the programs' own bug. vm/jit agree.
- RPG2/SAMPLE.BAS reads the BIOS column count with `DEF SEG = &H40: PEEK(&H4A)`; QBNex (like QB64 v1.4) does not emulate
  that byte, PEEK gives 0 and `LOCATE CSRLIN, 0 - ...` is error 5.
- Runtime errors 76/53 are DOS paths (`CHDIR "\TIM"`, `C:\T1\`, `RUN "\tim\menu.bas"`), data files the
  programs create in an earlier step (thebob: `*gfx.bas` makes the `.bsv`/`.fnt` files, see their readme), or file names typed by the user.
- RPG/LIGHT: cpp and jit reach error 53 at line 887 before the scripted close, vm (slower start) does not. Timing, not a bug.
- "Build failed" / "memory allocation of N bytes failed" during a run is the shared machine's commit limit; the runner
  retries (pete/lineswin's `--ir` compile hit it once and passed on a rerun). A compile that fails with no output at all
  (the app restarted during the first attempt of this run) is retried too.
- qb64/open_gl (`SUB _GL`) is deviation D-14; qb45com/assault declares `TYPE AS` (not valid in QB64 either).

## Results

`console` = console-only build. Cells: exit code and run time; `timeout` = still running after 6 s. The oracle column is empty in run 2.
| program | status | console | cpp | oracle | vm | jit | notes |
|---|---|---|---|---|---|---|---|
| creationix-basic-games/ARTIF/ANT.BAS | pass | yes | 0 0.9s | - | 0 0s | 0 0s | ir: meta console only added (Q9) |
| creationix-basic-games/ARTIF/MOUSE.BAS | pass | no | 0 4.5s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/BANANA/BANANA.BAS | pass | no | 0 1.4s | - | 0 3.5s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/BANANA/GRAPH.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/BANANA/PICS.BAS | rt-error | no | 1 0.9s | - | 1 0.1s | 1 0.1s |  |
| creationix-basic-games/BOMB/BOMBMAN.BAS | timeout | no | timeout 6.2s | - | 0 3.7s | 0 3.7s | vm exit 0 vs timeout; jit exit 0 vs timeout |
| creationix-basic-games/BOMB/GRAPH.BAS | pass | no | 0 1s | - | 0 3.6s | 0 3.6s |  |
| creationix-basic-games/BOMBMAN/BOMB.BAS | rt-error | no | 1 1.7s | - | no compile | no compile | ir compile: QIR: unsupported: line 58: function STRIG |
| creationix-basic-games/BOMBMAN/COLOR.BAS | pass | no | 0 0.9s | - | 0 3.5s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/BOMBMAN/DRAW.BAS | pass | no | 0 4.1s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/BOMBMAN/LOAD.BAS | pass | no | 0 0.8s | - | 0 3.6s | 0 3.6s |  |
| creationix-basic-games/BOMBMAN/TESTING.BAS | rt-error | no | 1 0.8s | - | 1 0.1s | 1 0.1s |  |
| creationix-basic-games/BOMBMAN/VNSTALL.BAS | pass | no | 0 4.3s | - | 0 3.6s | 0 3.6s |  |
| creationix-basic-games/BOMB/NUKEM.BAS | pass | no | 0 0.8s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/BOMB/NUKER.BAS | pass | no | 0 5.1s | - | 0 3.5s | 0 3.5s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/BOMB/VECTOCAT.BAS | diff | no | 0 5.2s | - | 1 0.1s | 1 0.1s | vm framebuffer differs; vm differs: exit 1 vs 0; jit framebuffer differs; jit differs: exit 1 vs 0; uses TIMER/RANDOMIZE: may be nondeterministic |
| creationix-basic-games/BOMB/WARZONE.BAS | rt-error | no | 1 4.3s | - | 1 0.1s | 1 0.1s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/C/CQUICK.BAS | diff | no | 0 4.1s | - | 0 3.5s | 0 3.5s | vm framebuffer differs; vm differs: stdout; jit framebuffer differs; jit differs: stdout |
| creationix-basic-games/C/FONT.BAS | rt-error | no | 1 2.1s | - | 1 1.2s | 1 1.2s |  |
| creationix-basic-games/CONQUEST/CONQUEST.BAS | rt-error | no | 1 1.4s | - | 1 0.1s | 1 0.2s |  |
| creationix-basic-games/C/QC.BAS | diff | no | 0 5.1s | - | 0 3.6s | 0 3.6s | vm differs: stdout; jit differs: stdout |
| creationix-basic-games/C/QUICK.BAS | diff | no | 0 5.1s | - | 0 3.5s | 0 3.6s | vm differs: stdout; jit differs: stdout |
| creationix-basic-games/DROIDS/DROIDS.BAS | pass | no | 0 5.1s | - | 0 3.5s | 0 3.5s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/FOWLER/EDIT.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/FOWLER/FOWLER.BAS | diff | no | 0 5.4s | - | 1 0.1s | 1 0.1s | vm framebuffer differs; vm differs: exit 1 vs 0; jit framebuffer differs; jit differs: exit 1 vs 0; uses TIMER/RANDOMIZE: may be nondeterministic |
| creationix-basic-games/FOWLER/GRAPH.BAS | pass | no | 0 1s | - | 0 3.6s | 0 3.5s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/FOWLER/WINDOW.BAS | pass | no | 0 4.4s | - | 0 3.5s | 0 3.5s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/GALAXY/GALAXY.BAS | pass | no | 0 4.7s | - | 0 3.5s | 0 3.5s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/GIRLS/WAR.BAS | pass | no | 0 5.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 53: function STRIG |
| creationix-basic-games/LABYRINT/LABYRINT.BAS | pass | no | 0 4.9s | - | 0 3.6s | 0 3.5s |  |
| creationix-basic-games/LOGATH/CLOGATH.BAS | diff | no | 0 5.6s | - | 1 0.1s | 1 0.1s | vm framebuffer differs; vm differs: exit 1 vs 0; jit framebuffer differs; jit differs: exit 1 vs 0; uses TIMER/RANDOMIZE: may be nondeterministic |
| creationix-basic-games/LOGATH/LOGATH.BAS | diff | no | 0 5.4s | - | 1 0.1s | 1 0.1s | vm framebuffer differs; vm differs: exit 1 vs 0; jit framebuffer differs; jit differs: exit 1 vs 0; uses TIMER/RANDOMIZE: may be nondeterministic |
| creationix-basic-games/LOGATH/LOGATHG.BAS | pass | no | 0 0.7s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/LOGATH/STARS.BAS | pass | no | 0 4.4s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/MAGIC/MAGIC.BAS | pass | no | 0 5.6s | - | 0 3.6s | 0 3.6s |  |
| creationix-basic-games/MAGIC/SHUFFLE.BAS | rt-error | yes | 1 1s | - | 1 0.1s | 1 0.1s | ir: meta console only added (Q9) |
| creationix-basic-games/MASTER/BUILDING.BAS | pass | no | 0 4.4s | - | 0 3.6s | 0 3.6s |  |
| creationix-basic-games/MASTER/CMASTER.BAS | pass | no | 0 5.3s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/MASTER/CREATURE.BAS | pass | no | 0 4.3s | - | 0 3.6s | 0 3.6s |  |
| creationix-basic-games/MASTER/MASTER.BAS | pass | no | 0 5.3s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/MASTER/SPELLS.BAS | pass | no | 0 4.4s | - | 0 3.6s | 0 3.6s |  |
| creationix-basic-games/MENU.BAS | rt-error | no | 1 0.7s | - | 1 0.1s | 1 0.1s |  |
| creationix-basic-games/MISC/ARTIF.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/MISC/CFROGGER.BAS | pass | no | 0 5.4s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/MISC/CGAEXPL.BAS | pass | no | 0 4.3s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/MISC/CGAMINE.BAS | pass | no | 0 5.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 32: function STRIG |
| creationix-basic-games/MISC/CLOUD.BAS | pass | no | 0 1s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/MISC/CTETRIS.BAS | pass | no | 0 5.3s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/MISC/EXPLORER.BAS | pass | no | 0 4.3s | - | 0 3.5s | 0 3.5s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/MISC/FROGGER.BAS | pass | no | 0 5.3s | - | 0 3.5s | 0 3.5s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/MISC/GIFLOAD.BAS | rt-error | no | 1 0.9s | - | 1 0s | 1 0.1s |  |
| creationix-basic-games/MISC/HEXAGON.BAS | pass | no | 0 4.3s | - | no compile | no compile | ir compile: QIR: unsupported: line 132: function STRIG |
| creationix-basic-games/MISC/HEXMINE.BAS | pass | no | 0 5.3s | - | no compile | no compile | ir compile: QIR: unsupported: line 184: function STRIG |
| creationix-basic-games/MISC/LAND2.BAS | pass | no | 0 1s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/MISC/LAND3.BAS | pass | no | 0 2s | - | 0 3.6s | 0 3.6s |  |
| creationix-basic-games/MISC/LANDSKP.BAS | pass | no | 0 1s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/MISC/MINESWEP.BAS | pass | no | 0 4.3s | - | no compile | no compile | ir compile: QIR: unsupported: line 68: function STRIG |
| creationix-basic-games/MISC/MOUSE.BAS | pass | no | 0 4.4s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/MISC/MULE.BAS | rt-error | no | 1 3.3s | - | 1 2.6s | 1 2.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/MISC/MULTIPLE.BAS | pass | no | 0 4.7s | - | no compile | no compile | ir compile: QIR: unsupported: line 4: ON KEY |
| creationix-basic-games/MISC/RESCUE.BAS | interactive-only | no | timeout 6.1s | - | 0 3.6s | 0 3.6s | vm exit 0 vs timeout; jit exit 0 vs timeout |
| creationix-basic-games/MISC/SCOUTS.BAS | pass | no | 0 5.4s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/MISC/SHIPWAR.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/MISC/SPACE.BAS | compile-fail | no | no compile | - | no compile | no compile | [!!] QBNex :: Fatal [E0000]  File contains binary data, not BASIC source text /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\run-tests-2\tests\corpus-local\creationix-basic-games\misc\space.bas(1) /   [>] next     Review the syntax and compare with QBasic documentation; ir compile: [!!] QBNex :: Fatal [E0000]  File contains binary data, not BASIC source text /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\run-tests-2\tests\corpus-local\creationix-basic-games\misc\space.bas(1) /   [>] next     Review the syntax and compare with QBasic documentation |
| creationix-basic-games/MISC/TETRIS.BAS | pass | no | 0 5.4s | - | 0 3.6s | 0 3.5s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/MISC/TRIMINE.BAS | pass | no | 0 5.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 302: function STRIG |
| creationix-basic-games/MOSS/MOSS.BAS | pass | no | 0 5.3s | - | 0 3.5s | 0 3.5s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/MOSS/MOSSPIC.BAS | pass | no | 0 1s | - | 0 3.6s | 0 3.6s |  |
| creationix-basic-games/MOSS/MUSIC.BAS | pass | yes | 0 1s | - | 0 0s | 0 0s | ir: meta console only added (Q9) |
| creationix-basic-games/MOUSE.BAS | pass | no | 0 0.9s | - | 0 3.6s | 0 3.6s |  |
| creationix-basic-games/NUKE/CNUKE.BAS | pass | no | 0 4.8s | - | 0 3.5s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/NUKE/CPICS.BAS | rt-error | no | 1 1s | - | 1 0s | 1 0.1s |  |
| creationix-basic-games/NUKE/NUKE.BAS | pass | no | 0 4.9s | - | 0 3.5s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/NUKE/PICS.BAS | rt-error | no | 1 0.9s | - | 1 0.1s | 1 0.1s |  |
| creationix-basic-games/QUESTLND/BACK.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/QUESTLND/GRAPHICS.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/QUESTLND/QUEST.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/QUESTLND/QUESTLND.BAS | compile-fail | no | no compile | - | no compile | no compile | [!!] QBNex :: Fatal [E0000]  Missing subprogram: SUB COLORPALETTE is declared, but this program has no SUB COLORPALETTE ... END SUB; it may be in a Quick Library or another module /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\run-tests-2\tests\corpus-local\creationix-basic-games\questlnd\questlnd.bas(11) /   [#] source; ir compile: [!!] QBNex :: Fatal [E0000]  Missing subprogram: SUB COLORPALETTE is declared, but this program has no SUB COLORPALETTE ... END SUB; it may be in a Quick Library or another module /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\run-tests-2\tests\corpus-local\creationix-basic-games\questlnd\questlnd.bas(11) /   [#] source |
| creationix-basic-games/QUESTLND/SHIP.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/QUESTLND/TESTBACK.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/QUESTLND/TEST.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/QUESTLND/TEXTURE3.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/QUESTLND/TEXTURE4.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/RACE/RACE.BAS | interactive-only | no | timeout 6.2s | - | 0 3.6s | 0 3.6s | vm exit 0 vs timeout; jit exit 0 vs timeout |
| creationix-basic-games/RPG2/CHARS.BAS | pass | no | 0 1.7s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/RPG2/CONVERT.BAS | binary-format | no | - | - | - | - | QuickBASIC binary (fast-load) format, not source |
| creationix-basic-games/RPG2/CSHAPES.BAS | rt-error | no | 1 0.8s | - | 1 0.1s | 1 0.1s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/RPG2/DMAPLAY.BAS | pass | no | 0 4.4s | - | 0 3.5s | 0 3.5s |  |
| creationix-basic-games/RPG2/MAPEDIT.BAS | rt-error | no | 1 1.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 3: statement FILES |
| creationix-basic-games/RPG2/PARTY.BAS | pass | no | 0 1.4s | - | 0 3.5s | 0 3.6s |  |
| creationix-basic-games/RPG2/QMIDI.BAS | pass | yes | 0 0.7s | - | 0 0s | 0 0s | ir: meta console only added (Q9) |
| creationix-basic-games/RPG2/RPG2.BAS | rt-error | no | 1 1.7s | - | 1 1.1s | 1 1.2s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/RPG2/SAMPLE.BAS | rt-error | no | 1 0.6s | - | 1 0.1s | 1 0.1s |  |
| creationix-basic-games/RPG2/SHAPES.BAS | rt-error | no | 1 0.8s | - | 1 0.1s | 1 0.1s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/RPG2/TEST.BAS | pass | no | 0 1.6s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/RPG2/TRANS.BAS | rt-error | no | 1 0.7s | - | 1 0.1s | 1 0.1s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/RPG3/BACK.BAS | pass | no | 0 4.5s | - | 0 3.5s | 0 3.5s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/RPG3/MOUSE.BAS | pass | no | 0 4.4s | - | 0 3.5s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/RPG3/PARTY.BAS | pass | no | 0 1.5s | - | 0 3.6s | 0 3.6s |  |
| creationix-basic-games/RPG3/SPELLS.BAS | pass | no | 0 5s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/RPG/CCONVERT.BAS | pass | no | 0 4.4s | - | 0 3.5s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/RPG/CLIGHT.BAS | rt-error | no | 1 4.1s | - | 1 2.5s | 1 2.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/RPG/CONVERT.BAS | pass | no | 0 4.4s | - | 0 3.5s | 0 3.5s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/RPG/ITEMS.BAS | pass | no | 0 4.3s | - | 0 3.5s | 0 3.5s |  |
| creationix-basic-games/RPG/LIGHT.BAS | rt-error | no | 1 4.1s | - | 0 3.5s | 1 2.5s | vm framebuffer differs; vm differs: exit 0 vs 1; jit framebuffer differs |
| creationix-basic-games/RPG/ULTIMAX.BAS | rt-error | no | 1 3.2s | - | 1 2.5s | 1 2.5s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/RPG/WORLD.BAS | pass | no | 0 3.3s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/SCORCH/CGA.BAS | pass | no | 0 5.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 52: ON KEY |
| creationix-basic-games/SCORCH/SCORC.BAS | timeout | no | timeout 6.1s | - | 0 3.6s | 0 3.6s | vm exit 0 vs timeout; jit exit 0 vs timeout |
| creationix-basic-games/SCORCH/SCORCH.BAS | pass | no | 0 4.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 52: ON KEY |
| creationix-basic-games/SHIP/SHIP.BAS | pass | no | 0 1s | - | 0 3.5s | 0 3.6s |  |
| creationix-basic-games/SHIP/TEXT.BAS | pass | no | 0 1.8s | - | 0 3.6s | 0 3.5s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/SORCERER/CMAKER.BAS | pass | no | 0 1s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/SORCERER/CSORCER.BAS | rt-error | no | 1 4.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 180: function STRIG |
| creationix-basic-games/SORCERER/MAKER.BAS | pass | no | 0 0.8s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/SORCERER/SORCERER.BAS | rt-error | no | 1 4.3s | - | no compile | no compile | ir compile: QIR: unsupported: line 175: function STRIG |
| creationix-basic-games/SORCERER/SORCER-G.BAS | pass | no | 0 0.8s | - | 0 3.6s | 0 3.6s |  |
| creationix-basic-games/TALK/TALK.BAS | pass | no | 0 4.2s | - | 0 3.6s | 0 3.6s |  |
| creationix-basic-games/TALK/TOM.BAS | pass | no | 0 1.5s | - | 0 3.6s | 0 3.6s |  |
| creationix-basic-games/WARS/COOLMAP.BAS | pass | no | 0 0.9s | - | 0 3.6s | 0 3.6s |  |
| creationix-basic-games/WARS/MAPEDIT.BAS | pass | no | 0 4.4s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/WARS/MOUSEWAR.BAS | rt-error | no | 1 2.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 516: function STRIG |
| creationix-basic-games/WARS/WARS.BAS | rt-error | no | 1 2.4s | - | no compile | no compile | ir compile: QIR: unsupported: line 506: function STRIG |
| creationix-basic-games/ZENITH/BACKS.BAS | pass | no | 0 0.9s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/ZENITH/PLANETS.BAS | pass | no | 0 1s | - | 0 3.6s | 0 3.6s |  |
| creationix-basic-games/ZENITH/SHIPS.BAS | pass | no | 0 0.9s | - | 0 3.5s | 0 3.6s |  |
| creationix-basic-games/ZENITH/SPACE.BAS | rt-error | no | 1 0.9s | - | 1 0s | 1 0.1s |  |
| creationix-basic-games/ZENITH/TEST.BAS | pass | no | 0 4.5s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| creationix-basic-games/ZENITH/ZENITH.BAS | rt-error | no | 1 1.4s | - | 0 3.6s | 0 3.6s | vm differs: exit 0 vs 1; jit differs: exit 0 vs 1 |
| qb64-samples/programs/samples/misc/3dballs.bas | pass | no | 0 1.4s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/misc/3dexp2.bas | pass | no | 0 3.1s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/misc/acalc.bas | pass | no | 0 3s | - | 0 2.2s | 0 2.3s |  |
| qb64-samples/programs/samples/misc/aklabeth.bas | pass | no | 0 4.2s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/misc/ascipong.bas | pass | no | 0 3.1s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/misc/audio.bas | pass | no | 0 5.4s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/misc/cabsmous.bas | pass | no | 0 4.3s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/misc/carols.bas | pass | no | 0 4.8s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/misc/chess.bas | pass | no | 0 4.2s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/misc/cuberot.bas | pass | no | 0 1.4s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/misc/djsok.bas | pass | no | 0 3s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/misc/flrmp.bas | pass | no | 0 1.4s | - | 0 3.5s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/misc/forest.bas | diff | no | 0 1.3s | - | 1 0.1s | 1 0.1s | vm framebuffer differs; vm differs: exit 1 vs 0; jit framebuffer differs; jit differs: exit 1 vs 0 |
| qb64-samples/programs/samples/misc/frac1.bas | pass | no | 0 3.1s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/misc/frac2.bas | pass | no | 0 0.9s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/misc/frac3.bas | pass | no | 0 3.1s | - | 0 3.5s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/misc/frog.bas | pass | no | 0 5.2s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/misc/gor64.bas | pass | no | 0 5.2s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/misc/gujero.bas | pass | no | 0 1.4s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/misc/intrprtr.bas | interactive-only | yes | timeout 6.1s | - | timeout 6.1s | timeout 6s | ir: meta console only added (Q9) |
| qb64-samples/programs/samples/misc/kite.bas | pass | no | 0 4.4s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/misc/lissaj.bas | pass | no | 0 1.4s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/misc/mandala.bas | pass | no | 0 1.4s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/misc/mandelb.bas | pass | no | 0 1.4s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/misc/matrix.bas | interactive-only | yes | timeout 6s | - | timeout 6.1s | timeout 6.1s | ir: meta console only added (Q9) |
| qb64-samples/programs/samples/misc/mclock.bas | pass | no | 0 1.4s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/misc/message.bas | interactive-only | no | timeout 6s | - | 0 3.6s | 0 3.6s | vm exit 0 vs timeout; jit exit 0 vs timeout |
| qb64-samples/programs/samples/misc/mzupd2.bas | diff | no | 0 4s | - | 1 0.1s | 1 0.3s | vm framebuffer differs; vm differs: exit 1 vs 0; jit framebuffer differs; jit differs: exit 1 vs 0; uses TIMER/RANDOMIZE: may be nondeterministic |
| qb64-samples/programs/samples/misc/nib64.bas | pass | no | 0 5.2s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/misc/pattern.bas | pass | no | 0 1.4s | - | 0 3.5s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/misc/rc-ent6.bas | pass | no | 0 3.3s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/misc/replicat.bas | pass | yes | 0 0.9s | - | 0 0s | 0 0s | ir: meta console only added (Q9) |
| qb64-samples/programs/samples/misc/ripples.bas | pass | no | 0 1.4s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/misc/rot13.bas | pass | yes | 0 0.9s | - | 0 0s | 0 0.1s | ir: meta console only added (Q9) |
| qb64-samples/programs/samples/misc/rotozoom.bas | pass | no | 0 1.4s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/misc/scramble.bas | pass | no | 0 3.1s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/misc/shoot.bas | pass | no | 0 5.3s | - | no compile | no compile | ir compile: QIR: unsupported: line 85: statement _MOUSEHIDE |
| qb64-samples/programs/samples/misc/sinecube.bas | pass | no | 0 3.2s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/misc/ssaver.bas | pass | no | 0 3.1s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/misc/strfld.bas | pass | no | 0 1.4s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/misc/temple.bas | pass | no | 0 4.6s | - | no compile | no compile | ir compile: QIR: unsupported: line 534: ON ( |
| qb64-samples/programs/samples/misc/tower.bas | pass | no | 0 3.1s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/misc/twirl2.bas | pass | no | 0 1.4s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/misc/vortex.bas | pass | no | 0 1.4s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/misc/wumpus.bas | interactive-only | yes | timeout 6s | - | timeout 6s | timeout 6.1s | ir: meta console only added (Q9) |
| qb64-samples/programs/samples/misc/xwing.bas | pass | no | 0 4.7s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/n54/big/3dsviewer/3dsviewer.bas | diff | no | 0 3.3s | - | 1 0.8s | 1 0.8s | vm framebuffer differs; vm differs: exit 1 vs 0; jit framebuffer differs; jit differs: exit 1 vs 0; uses TIMER/RANDOMIZE: may be nondeterministic |
| qb64-samples/programs/samples/n54/big/jpegmake.bas | pass | yes | 0 0.8s | - | 0 0.2s | 0 0.2s | ir: meta console only added (Q9) |
| qb64-samples/programs/samples/n54/big/tictac.bas | diff | no | 0 4.4s | - | 0xC0000409 0.3s | 0xC0000409 0.4s | vm differs: exit 0xC0000409 vs 0; jit differs: exit 0xC0000409 vs 0; uses TIMER/RANDOMIZE: may be nondeterministic |
| qb64-samples/programs/samples/open_gl/3d_model_viewer.bas | compile-fail | no | no compile | - | no compile | no compile | [!!] QBNex :: Fatal [E0000]  SUB _GL (OpenGL) is not supported by this QBNex runtime (deviation D-14) /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\run-tests-2\tests\corpus-local\qb64-samples\programs\samples\open_gl\3d_model_viewer.bas(158) /   [#] source; ir compile: [!!] QBNex :: Fatal [E0000]  SUB _GL (OpenGL) is not supported by this QBNex runtime (deviation D-14) /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\run-tests-2\tests\corpus-local\qb64-samples\programs\samples\open_gl\3d_model_viewer.bas(158) /   [#] source |
| qb64-samples/programs/samples/open_gl/simple_example.bas | compile-fail | no | no compile | - | no compile | no compile | [!!] QBNex :: Fatal [E0000]  SUB _GL (OpenGL) is not supported by this QBNex runtime (deviation D-14) /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\run-tests-2\tests\corpus-local\qb64-samples\programs\samples\open_gl\simple_example.bas(43) /   [#] source; ir compile: [!!] QBNex :: Fatal [E0000]  SUB _GL (OpenGL) is not supported by this QBNex runtime (deviation D-14) /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\run-tests-2\tests\corpus-local\qb64-samples\programs\samples\open_gl\simple_example.bas(43) /   [#] source |
| qb64-samples/programs/samples/pete/25lines/3dcube25.bas | diff | no | 0 1.5s | - | 1 0s | 1 0.1s | vm framebuffer differs; vm differs: exit 1 vs 0; jit framebuffer differs; jit differs: exit 1 vs 0 |
| qb64-samples/programs/samples/pete/25lines/npplasma.bas | pass | no | 0 1.5s | - | 0 3.5s | 0 3.5s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/pete/25lines/starfild.bas | pass | no | 0 1.5s | - | 0 3.5s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/pete/alyman.bas | diff | no | 0 4.7s | - | 1 0.1s | 1 0.1s | vm differs: exit 1 vs 0; jit differs: exit 1 vs 0; uses TIMER/RANDOMIZE: may be nondeterministic |
| qb64-samples/programs/samples/pete/asciiconversion.bas | pass | no | 0 3.9s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/averager.bas | interactive-only | yes | timeout 6s | - | timeout 6s | timeout 6s | ir: meta console only added (Q9) |
| qb64-samples/programs/samples/pete/battleship.bas | rt-error | yes | 1 0.3s | - | 1 0.1s | 1 0.1s | ir: meta console only added (Q9) |
| qb64-samples/programs/samples/pete/beatdown/bdt.bas | pass | no | 0 5.1s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/beatdown/beatdown.bas | pass | no | 0 5.2s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/beatdown/clickme.bas | pass | no | 0 1.5s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/bj.bas | pass | no | 0 3.8s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/booger/booger.bas | pass | no | 0 4.2s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/bush.bas | pass | no | 0 3.8s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/calc_2.bas | pass | no | 0 3.9s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/calc.bas | pass | no | 0 1s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/carracingenglish.bas | pass | no | 0 4.9s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/pete/cc2.bas | rt-error | no | 1 0.9s | - | 1 0.6s | 1 0.6s |  |
| qb64-samples/programs/samples/pete/coolscr.bas | pass | no | 0 2s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/pete/cr/cr.bas | pass | no | 0 4.8s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/darokin.bas | pass | no | 0 0.8s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/pete/darpong/darpong.bas | pass | no | 0 4.5s | - | 0 3.5s | 0 3.5s |  |
| qb64-samples/programs/samples/pete/degrees.bas | pass | no | 0 1s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/diceit.bas | pass | no | 0 0.5s | - | 0 3.5s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/drug3d.bas | pass | no | 0 3.8s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/fight.bas | pass | no | 0 2.6s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/gravity.bas | pass | no | 0 1.1s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/pete/hgm30e/hgm30e.bas | rt-error | no | 1 1.9s | - | 1 1.2s | 1 1.2s |  |
| qb64-samples/programs/samples/pete/html/test.bas | pass | no | 0 4.4s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/invader1/invader.bas | pass | no | 0 2.8s | - | 0 2.3s | 0 2.3s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/pete/kbdparse.bas | pass | no | 0 1.4s | - | 0 3.6s | 0 4s |  |
| qb64-samples/programs/samples/pete/lineswin.bas | pass | no | 0 4s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/pete/makebig.bas | pass | no | 0 0.7s | - | 0 3.6s | 0 3.5s | jit hit the system memory limit (try 1) |
| qb64-samples/programs/samples/pete/monopoly/monopoly.bas | diff | no | 0 5.3s | - | 0 3.6s | 0 3.6s | vm differs: stdout; jit differs: stdout; uses TIMER/RANDOMIZE: may be nondeterministic |
| qb64-samples/programs/samples/pete/mooncr/mooncr.bas | pass | no | 0 3.8s | - | 0 3.6s | 0 3.7s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/pete/move.bas | pass | no | 0 4.9s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/mrguessit.bas | pass | no | 0 3.9s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/nlcm2006/nlcm.bas | diff | no | 0 4s | - | 1 1.5s | 1 2.1s | vm differs: exit 1 vs 0; jit differs: exit 1 vs 0; uses TIMER/RANDOMIZE: may be nondeterministic |
| qb64-samples/programs/samples/pete/optimus/deedsaks.bas | diff | no | 0 3.7s | - | 0 3.6s | 0xC0000005 3.7s | vm framebuffer differs; jit differs: exit 0xC0000005 vs 0; uses TIMER/RANDOMIZE: may be nondeterministic |
| qb64-samples/programs/samples/pete/pall2.bas | pass | no | 0 0.9s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/pete/picdimreader.bas | rt-error | no | 1 1s | - | 1 0.6s | 1 0.6s |  |
| qb64-samples/programs/samples/pete/pongsource.bas | pass | no | 0 2.5s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/pete/pongtennis/tennis.bas | pass | no | 0 3.8s | - | no compile | no compile | ir compile: QIR: unsupported: line 344: unknown name in expression: changeAmount |
| qb64-samples/programs/samples/pete/pp256/usersubs.bas | diff | no | 0 4.1s | - | 1 0.1s | 1 0.1s | vm framebuffer differs; vm differs: exit 1 vs 0; jit framebuffer differs; jit differs: exit 1 vs 0 |
| qb64-samples/programs/samples/pete/qbb.bas | pass | no | 0 0.4s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/rats1.bas | pass | no | 0 5s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/pete/raytra1b.bas | pass | no | 0 1.2s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/pete/rect2.bas | pass | no | 0 4s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/rectong/rectong.bas | pass | no | 0 4.2s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/roboraid/roboraid.bas | pass | no | 0 5s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/rotate.bas | pass | no | 0 0.9s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/pete/rug.bas | pass | no | 0 0.9s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/screen.bas | pass | no | 0 0.9s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/simpire/ssimpire.bas | pass | no | 0 5.2s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/pete/skydiver.bas | pass | no | 0 2.6s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/pete/space1.bas | pass | no | 0 5s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/stones.bas | pass | no | 0 3.8s | - | 0 3.6s | 0 3.5s |  |
| qb64-samples/programs/samples/pete/su2/shoote.bas | pass | no | 0 3.1s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/su2/su2t.bas | pass | no | 0 4.3s | - | 0 3.5s | 0 3.5s |  |
| qb64-samples/programs/samples/pete/sudoku.bas | pass | no | 0 1s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/tank/tcv2.bas | pass | no | 0 3.4s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/time.bas | pass | no | 0 1s | - | 0 3.6s | 0 3.5s |  |
| qb64-samples/programs/samples/pete/torndc.bas | pass | no | 0 5s | - | 0 3.5s | 0 3.5s |  |
| qb64-samples/programs/samples/pete/tor/qbdemo.bas | pass | no | 0 4.3s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/pete/upperlowerconversion.bas | pass | no | 0 3.9s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/vektor.bas | pass | no | 0 3.9s | - | 0 3.5s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/vsphere.bas | pass | no | 0 1s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/pete/wetspot/wetspot.bas | pass | no | 0 4.6s | - | 0 3.5s | 0 3.5s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/pete/zodiac4.bas | pass | no | 0 4.7s | - | 0 3.5s | 0 3.6s |  |
| qb64-samples/programs/samples/qb45com/action/4pong.bas | pass | no | 0 4.1s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/qb45com/action/arcdemo/arcdemo.bas | pass | no | 0 3.9s | - | 0 3.5s | 0 3.5s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/qb45com/action/arqanoid/arqanoid.bas | diff | no | 0 5.1s | - | 1 0.1s | 1 0.3s | vm framebuffer differs; vm differs: exit 1 vs 0; jit framebuffer differs; jit differs: exit 1 vs 0; uses TIMER/RANDOMIZE: may be nondeterministic |
| qb64-samples/programs/samples/qb45com/action/arqanoid/lvldesgn.bas | pass | no | 0 4.2s | - | 0 3.5s | 0 3.5s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/qb45com/action/assault/assault.bas | compile-fail | no | no compile | - | no compile | no compile | [!!] QBNex :: Fatal [E0000]  Expected element-name AS type, AS type element-list, or END TYPE /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\run-tests-2\tests\corpus-local\qb64-samples\programs\samples\qb45com\action\assault\assault.bas(84) /   [#] source; ir compile: [!!] QBNex :: Fatal [E0000]  Expected element-name AS type, AS type element-list, or END TYPE /   [@] d:\qbnex-rust\.claude\worktrees\r13-tools-4988b7\run-tests-2\tests\corpus-local\qb64-samples\programs\samples\qb45com\action\assault\assault.bas(84) /   [#] source |
| qb64-samples/programs/samples/qb45com/action/astrowars.bas | pass | no | 0 3.7s | - | 0 3.5s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/qb45com/action/sfb2/sfb2.bas | pass | no | 0 4.8s | - | 0 3.6s | timeout 6s | vm framebuffer differs; jit exit timeout vs 0 |
| qb64-samples/programs/samples/thebob/abacus/a12gfx.bas | pass | no | 0 1.4s | - | 0 0.5s | 0 0.5s |  |
| qb64-samples/programs/samples/thebob/abacus/abacus12.bas | rt-error | no | 1 1s | - | 1 0s | 1 0s |  |
| qb64-samples/programs/samples/thebob/animax/animax.bas | rt-error | no | 1 0.5s | - | 1 0.1s | 1 0.2s |  |
| qb64-samples/programs/samples/thebob/animax/axgfx.bas | pass | no | 0 1.1s | - | 0 3.6s | 0 3.5s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/thebob/biochart/bcgfx.bas | pass | no | 0 1.4s | - | 0 3.5s | 0 3.6s |  |
| qb64-samples/programs/samples/thebob/biochart/biochart.bas | rt-error | no | 1 0.9s | - | 1 0s | 1 0s |  |
| qb64-samples/programs/samples/thebob/chess/chess.bas | pass | no | 0 1.4s | - | 0 3.6s | 0 3.6s |  |
| qb64-samples/programs/samples/thebob/chess/chesssubs.bas | rt-error | no | 1 0.9s | - | 0 0.6s | 0 0.5s | vm framebuffer differs; vm differs: exit 0 vs 1; jit framebuffer differs; jit differs: exit 0 vs 1 |
| qb64-samples/programs/samples/thebob/chopper/chopfile.bas | pass | no | 0 4.2s | - | 0 3.5s | 0 3.6s |  |
| qb64-samples/programs/samples/thebob/chopper/chopper.bas | pass | no | 0 4.5s | - | 0 0.6s | 0 0.5s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/thebob/kong/k1.bas | pass | no | 0 0.6s | - | 0 3.5s | 0 3.6s |  |
| qb64-samples/programs/samples/thebob/kong/k2.bas | rt-error | no | 1 0.4s | - | 1 0.1s | 1 0.1s |  |
| qb64-samples/programs/samples/thebob/kong/kong.bas | rt-error | no | 1 0.7s | - | 1 0s | 1 0s |  |
| qb64-samples/programs/samples/thebob/leapfrog/leapfrog.bas | rt-error | no | 1 1.9s | - | 0 0.6s | 0 0.5s | vm framebuffer differs; vm differs: exit 0 vs 1; jit framebuffer differs; jit differs: exit 0 vs 1 |
| qb64-samples/programs/samples/thebob/leapfrog/lfgx.bas | pass | no | 0 1.4s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/thebob/minigolf/mg.bas | pass | no | 0 3.6s | - | 0 2.1s | 0 2.1s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/thebob/monopoly/mboard.bas | pass | no | 0 4s | - | 0 2.2s | 0 2.2s |  |
| qb64-samples/programs/samples/thebob/pongg/pgfx.bas | pass | no | 0 1.1s | - | 0 3.5s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/thebob/pongg/pongg.bas | rt-error | no | 1 1.8s | - | 0 0.5s | 0 0.6s | vm framebuffer differs; vm differs: exit 0 vs 1; jit framebuffer differs; jit differs: exit 0 vs 1 |
| qb64-samples/programs/samples/thebob/rattler/rattler.bas | pass | no | 0 3.9s | - | 0 2.2s | 0 2.2s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/thebob/sol/s3gfx.bas | pass | no | 0 1.2s | - | 0 3.6s | 0 3.6s | vm framebuffer differs; jit framebuffer differs |
| qb64-samples/programs/samples/thebob/sol/sol3.bas | rt-error | no | 1 1.7s | - | 1 0s | 1 0s |  |

