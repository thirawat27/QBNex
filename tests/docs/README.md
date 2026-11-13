# Documentation tests

`check_docs.ps1` compiles every ```` ```basic ```` block of `README.md` and
`docs/manual/**` with the real compiler and, where the docs show an output,
runs the program and compares its standard output.

```
pwsh tests/docs/check_docs.ps1 -Compiler path/to/qb.exe
pwsh tests/docs/check_docs.ps1 -Files docs/manual/en/stdlib.md -Match regex -Keep
pwsh tests/docs/check_docs.ps1 -List                               # blocks and their markers
```

`-Compiler` defaults to `QB_COMPILER`, then `qb(.exe)` at the repository root.
Work files go to `target/docs-test` (kept for failing blocks, or with `-Keep`).
Exit code 1 when a block fails.

## Markers

An HTML comment on the line(s) directly above a code fence:

| Marker | Meaning |
| --- | --- |
| (none) | Must compile. |
| `<!-- docs-test: output -->` | Compile, run, compare stdout with the next ```` ```text ```` block (trailing blanks and trailing empty lines ignored; numbers keep their leading space). |
| `<!-- docs-test: run -->` | Compile and run; exit code 0. |
| `<!-- docs-test: compile-error -->` | Must not compile: planned syntax. Fails once the compiler accepts it, so the doc gets updated. |
| `<!-- docs-test: unsupported <reason> -->` | Must not compile (or, for run blocks, must not run) yet: a feature the QIR back end does not have. Fails once it works, so the doc gets updated. |
| `<!-- docs-test: skip <reason> -->` | Not compiled (fragments, wrong syntax shown on purpose, needs a file). |
| `<!-- docs-test: define NAME -->` / `<!-- docs-test: prepend NAME -->` | Reuse a block (for example the import lines) as the start of later fragments in the same file. |

Programs run headless (`QBNEX_HEADLESS=1`, `QBNEX_NO_ERROR_DIALOG=1`, empty
stdin, 30 s timeout). `$CONSOLE:ONLY` is added when the block has none, and
`SYSTEM` is appended to blocks without SUB/FUNCTION, so a block ends the same
way whether or not it ends with `END` (in an interactive console `END` waits at
"Press any key to continue"; with stdin or stdout redirected it does not, D-R12b-6). The Thai and English pages share their examples, so identical
blocks are compiled once.
