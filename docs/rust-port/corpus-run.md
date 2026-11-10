# Corpus run (real programs, headless)

Status: **partial** - the full run stopped after 33 of 277 programs because the shared machine ran out of commit memory ("The paging file is too small"). The runner now retries process starts; rerun with:

    pwsh tests\corpus-run\run.ps1 -Oracle <path-to-.oracle\qb.exe> -Timeout 6

Setup: rust-port ae57d8c. qb.exe built in this worktree (libqb_runtime.a and qb-exec.exe copied from the main checkout, same source tree, because rustc kept dying with os error 1453 / 0xC0000409 on the shared machine). Corpus: creationix/basic-games (129 files) + QB64Team/qb64 tag v1.4 programs/samples (148 files; HEAD no longer ships samples). Oracle: corpus-fixes worktree .oracle\qb.exe, console-only programs only (the legacy runtime ignores QBNEX_HEADLESS and would open a window).

Summary of the 33 programs run: binary-format: 2, compile-fail: 2, pass: 21, rt-error: 6, timeout: 1

## Findings so far

1. **qb-exec has no window / input script support.** A GUI (non console-only) program built with `--ir` prints its text screen to stdout and never receives QBNEX_INPUT_SCRIPT events or `close`. Repro: MAGIC.BAS (infinite `DO ... LOOP` drawing cards): the C++ build exits 0 after the scripted `close`, the QIR build runs until timeout writing 3.8 MB to stdout. Owner: R11a/R11b (qb-exec rt, not touched here).
2. **QIR emitter does not support `SCREEN` and `ON PLAY`** (`QIR: unsupported: line N: statement SCREEN`): most creationix games fail `--ir` compile. Owner: R11a emitter.
3. Executables of the C++ path chdir to their own folder at startup (QB64 behaviour); the runner copies each exe next to the program's data. Not a bug.
4. No qb-runtime bug found yet in the programs that ran; no runtime fix made.

## Results (partial)

| program | status | console | cpp | oracle | vm | jit | notes |
|---|---|---|---|---|---|---|---|
| creationix-basic-games/TALK/TALK.BAS | pass | no | 0 4.3s | - | timeout 6.1s | timeout 6.1s |  |
| creationix-basic-games/TALK/TOM.BAS | pass | no | 0 1.8s | - | 0 0.2s | 0 0.2s |  |


