# Local program corpus (not committed)

Put real BASIC programs you own a copy of here (for example the MS-DOS QBasic
samples GORILLA.BAS, NIBBLES.BAS, MONEY.BAS, or QB/QB64 community games).
Everything in this folder except this README is ignored by git, so copyrighted
programs never enter the repository.

`pwsh scripts/corpus_check.ps1` compiles every program here and in `examples/`,
starts each one headless for a few seconds and reports the ones that do not
compile, end with a run-time error or crash (internal error); the logs go to
`target/corpus-check`. It does not compare output: run a program by hand to see
it, for example `qb tests/corpus-local/GORILLA.BAS -o gorilla.exe`.
Committed regression tests belong in `tests/conformance/cases/`.
