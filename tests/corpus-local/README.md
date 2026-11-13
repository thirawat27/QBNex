# Local program corpus (not committed)

Put real BASIC programs you own a copy of here (for example the MS-DOS QBasic
samples GORILLA.BAS, NIBBLES.BAS, MONEY.BAS, or QB/QB64 community games).
Everything in this folder except this README is ignored by git, so copyrighted
programs never enter the repository.

There is no automatic runner: the harness that compared these programs with the
old C++ build (`tests/corpus-run`) was removed together with that build. Compile
and run them by hand, for example `qb tests/corpus-local/GORILLA.BAS -o gorilla.exe`.
Committed regression tests belong in `tests/conformance/cases/`.
