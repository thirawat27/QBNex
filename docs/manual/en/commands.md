# The `qb` / `qbnex` commands

[Manual index](../README.md) · [ภาษาไทย](../th/commands.md)

`qb` and `qbnex` are the same compiler. `qbnex` (the launch script) also runs
the developer tools.

## Compile

```bash
qb hello.bas                  # hello.exe on Windows, ./hello elsewhere
qb hello.bas -o tools/hi.exe  # choose the output file
qb hello.bas -w               # also show warnings
```

Without `-o` the executable goes to the QBNex folder (the one that holds
`qb` and `crates/`), as in QB64; `-s:exewithsource=true` puts it next to
the source instead. Give the source as an absolute path when you work in
another folder (see [known defects](differences.md#known-defects-not-fixed-yet)).

| Switch | Meaning |
| --- | --- |
| `<file>` | Source file (UTF-8, or CP437 bytes as QBasic saved them). |
| `-c` | Compile (the default). |
| `-o <file>` | Output executable. A name ending in `.qir` writes the program's QIR text instead (what [qb-web](../../../crates/qb-web/README.md) runs). |
| `--emit-ir` | Write the QIR text instead of an executable (`-o x.exe` becomes `x.qir`). |
| `-x` | Compile and send the compiler's output to the console. |
| `-w` | Show warnings. |
| `-Werror`, `--warnings-as-errors` | Treat warnings as errors. |
| `-q` | Quiet (warnings and errors are still shown). |
| `-m` | No colours in the compiler output. |
| `-k`, `--compact-errors` | Short diagnostics (detailed ones are the default; `-d` is the old alias). |
| `-e` | `OPTION _EXPLICIT` for this compilation. |
| `-s[:setting=true/false]` | Show or change compiler settings (`exewithsource`; `debuginfo` is kept but has no effect). |
| `-p`, `-z`, `--ir` | Accepted for old scripts; they change nothing (there are no precompiled objects and no C++ step, and QIR is the only back end). |
| `-h`, `--help` · `-v`, `--version` · `-i`, `--info` · `-g`, `--examples` | Help, version, project information, examples. |

Exit code 0 means the program compiled. Diagnostics name the file, line and
column and quote the source line.

## Run

Run the executable you built: `hello.exe` or `./hello`.

- An unhandled runtime error in an interactive console shows the classic
  "Unhandled Error #n ... Continue?" dialog. When output is redirected, or
  `QBNEX_NO_ERROR_DIALOG=1` is set, it prints one line to stderr instead and
  the program ends with exit code 1. Scripts and CI should set the variable.
- `qb run file.bas` (run on the VM without writing an executable) is
  **Planned**.

## How a program is built

The compiler writes QIR (the QBNex intermediate representation,
`docs/rust-port/qir-v1.md`) and appends it to a copy of the prebuilt Rust
runner `crates/qb-exec/bin/<os>/qb-exec`. No C or C++ compiler and no linker
run. The runner lowers QIR to bytecode, runs it on a register VM and
compiles hot loops to native code with Cranelift
(`QB_JIT=off|eager|<threshold>`). A statement the QIR back end does not
cover yet stops the build with `QIR: unsupported: line N: ...`.

Native executables (**Preview**): when a native kit sits next to the runner
(`crates/qb-exec/bin/<os>/native/kit.txt`, built on Windows by
`scripts/native-kit.ps1`, on Linux/macOS by `scripts/native-kit.sh`), the
compiler links an ordinary executable with `rust-lld` instead of appending
the QIR, and falls back to the runner copy if the link fails.

## `qbnex fmt`: formatter

```bash
qbnex fmt file.bas                 # print the formatted file
qbnex fmt -w *.bas                 # rewrite files in place
qbnex fmt --check src/*.bas        # exit 1 and list files that would change
type file.bas | qbnex fmt          # stdin to stdout (Windows; `cat` elsewhere)
```

Options: `--keyword-case upper|lower|preserve` (default `upper`),
`--indent N` (default 4), `--tabs`, `--line-ending auto|lf|crlf`.

The formatter changes keyword case, indentation of blocks (CASE one level
inside SELECT; line numbers and labels in column 0) and spacing. It never
changes comments, strings, `DATA` items, metacommands or the names in
`DECLARE LIBRARY`, keeps the file's encoding (CP437 stays CP437), and
formatting twice gives the same result. On Windows `qb.cmd fmt` works too.

## `qbnex lsp`: language server

Starts the Language Server Protocol server on stdin/stdout, for editors
other than VS Code (Neovim, Helix, Emacs, Sublime LSP...). Configure the
editor to run `qbnex lsp` (or `qb-lsp`) for `.bas`, `.bi`, `.bm` files.
Features: diagnostics, outline, go to definition and references across
`$INCLUDE` files, hover help for keywords and standard library procedures,
completion, formatting. See [VS Code and tools](vscode.md).

## `qbnex dap`: debugger

Starts the Debug Adapter Protocol server on stdin/stdout. Editors start it
for you (VS Code: F5). It compiles the program with `qb` and runs it on
the VM with breakpoints, stepping and variable views. See
[VS Code and tools](vscode.md#debugger).

## Environment variables

| Variable | Effect |
| --- | --- |
| `QBNEX_NO_ERROR_DIALOG=1` | Runtime errors go to stderr, exit code 1, no dialog. |
| `QB_JIT=off` / `eager` / `<n>` | QIR runner: no JIT, compile at once, or after n loop iterations. |
| `QB_GPU=off` / `verbose` | Builds with the GPU feature: never use the GPU / log the choice. |
| `QBNEX_HEADLESS` | Run graphics programs without a window (tests). |
