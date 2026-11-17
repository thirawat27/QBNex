# QBNex BASIC for Visual Studio Code

Language support for QBasic, QB64 and QBNex source files (`.bas`, `.bi`, `.bm`).

## Features

- **Syntax highlighting**: a TextMate grammar generated from the same keyword
  tables the QBNex tools use (language keywords plus every built-in statement
  and function of the compiler). It knows comments (`'`, `REM`, `#`),
  `$` metacommands and `'$INCLUDE`, line numbers, labels, `&H`/`&O`/`&B`
  literals, type suffixes, and SUB/FUNCTION/TYPE names.
- **Snippets**: `if`, `ifelse`, `for`, `do`, `dowhile`, `while`, `select`,
  `sub`, `function`, `type`, `dimshared`, `openin`, `openout`, `onerror`,
  `declib`, `gameloop`, `include`, and for the standard library `import`,
  `list`, `dict`, `json` (in the method style: `DIM names AS List`,
  `names.Add "x"`).
- **Language server** (`qb-lsp`): diagnostics as you type (syntax errors,
  unmatched blocks, include files that are missing), outline (SUB, FUNCTION,
  TYPE and its fields, CONST, labels, line numbers that are jump targets),
  go to definition and find references across the file and its `$INCLUDE`s,
  hover help for built-in keywords and standard library procedures,
  completion (after `names.` on a `DIM names AS List` it offers `Add`,
  `Count`, ... and hover on `names.Add` shows `List_Add`), and formatting.
- **Format on save** through `qb-lsp` (it uses `qb-fmt`): keyword case,
  block indentation, spacing. Turned on by default for QBasic files; switch it
  off with `"[qbasic]": { "editor.formatOnSave": false }`.
- **Commands**:
  - `QBNex: Run` (Ctrl+F5, or the play button in the editor title): saves, compiles
    with `qb`, then starts the program in a terminal (like `qb run file.bas`
    on the command line). Running again first closes the last run's terminal.
  - `QBNex: Compile` (Ctrl+Shift+B): saves and runs `qb` with
    `qbnex.compileArgs`; the compiler output goes to the *QBNex* output panel
    and its errors and warnings to the *Problems* panel.
  - `QBNex: Restart Language Server`.

## Debugging

F5 debugs the current `.bas` file with `qb-dap` (no `launch.json` needed;
*Add Configuration* offers a *QBNex: Debug program* snippet). The program is
compiled with `qb` and runs on the qb-exec VM (JIT off) with:
breakpoints by line in the launched file, continue, pause, step over / into /
out (per statement), the call stack of SUB/FUNCTION frames, variables of each
frame and globals (numbers, strings, arrays, TYPE fields; arrays show up to
1000 elements), and watch/hover expressions `name`, `name.field`,
`name(i, j)`. Launch options: `program`, `qir` (skip compiling),
`compiler`, `cwd`, `stopOnEntry`, `input` (file used as standard input;
without it INPUT sees end of file). Program output goes to the Debug Console.

## Requirements

- The QBNex compiler `qb` (or `qbnex`) on `PATH`, or set `qbnex.compilerPath`.
  On Windows `qb.cmd` / `qbnex.cmd` are front ends: the extension starts the
  `qb.exe` next to them, or set `qbnex.compilerPath` to `qb.exe` itself.
- `qb-lsp` and `qb-dap`: set `qbnex.lsp.path` / `qbnex.dap.path`, or the
  extension uses the copy bundled in it (see below), then the one on `PATH`,
  then the one in the compiler's folder (the install folder; on Linux and
  macOS the installer puts only `qb` and `qbnex` on `PATH`), then `qb lsp` /
  `qb dap`. Build them from the repository with
  `cargo build --release -p qb-lsp -p qb-dap`.

## Settings

| Setting | Default | Meaning |
| --- | --- | --- |
| `qbnex.compilerPath` | `qb` | Compiler command or full path (on Windows a `.cmd` front end is replaced by the `qb.exe` next to it). |
| `qbnex.compileArgs` | `["${file}", "-o", "${fileDirname}/${fileBasenameNoExtension}${exeExtension}"]` | Compiler arguments; Run starts the `-o` file, else the executable next to the source. Variables: `${file}`, `${fileDirname}`, `${fileBasename}`, `${fileBasenameNoExtension}`, `${fileExtname}`, `${exeExtension}`, `${workspaceFolder}`, `${userHome}`, `${env:NAME}`. |
| `qbnex.lsp.path` | (empty) | Path of `qb-lsp`; empty means the bundled copy, then `PATH`, then the compiler's folder, then `qb lsp`. |
| `qbnex.lsp.enabled` | `true` | Start the language server. |
| `qbnex.dap.path` | (empty) | Path of `qb-dap`; empty means the bundled copy, then `PATH`, then the compiler's folder, then `qb dap`. |

## Building the extension

From `packaging/vscode`:

```
npm install
npm test                 # compiles the TypeScript, tests the grammar and helpers
npx vsce package         # writes qbnex-<version>.vsix (finds qb-lsp as in Requirements)
```

To ship the server inside the package (one package per platform):

```
cargo build --release -p qb-lsp -p qb-dap   # from the repository root
npm run bundle-server -- --target win32-x64 # copies them to server/ (release builds only)
npx vsce package --target win32-x64          # or linux-x64, darwin-arm64, ...
```

Install a package with `code --install-extension qbnex-<version>.vsix`.

The tools also run from the command line through the compiler, on every
system: `qb fmt file.bas`, `qb lsp`, `qb dap`.

`syntaxes/qbasic.tmLanguage.json` is generated: after the keyword tables in
`crates/compiler/syntax` change, run `npm run grammar`.

