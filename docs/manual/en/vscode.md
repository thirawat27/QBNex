# VS Code, formatter, language server, debugger

[Manual index](../README.md) · [ภาษาไทย](../th/vscode.md)

## VS Code extension (Available)

The extension lives in `packaging/vscode`. Build and install it:

```bash
cd packaging/vscode
npm install
npx vsce package                      # qbnex-2.0.0.vsix
code --install-extension qbnex-2.0.0.vsix
```

It needs `qb` (or set `qbnex.compilerPath`) and, for the language features,
`qb-lsp` and `qb-dap` (`cargo build --release -p qb-lsp -p qb-dap`, then put
them on `PATH`, set `qbnex.lsp.path` / `qbnex.dap.path`, or bundle them with
`npm run bundle-server` before packaging).

| Feature | How |
| --- | --- |
| Highlighting | TextMate grammar from the same keyword tables the tools use (all built-ins of the compiler). |
| Snippets | `if`, `for`, `do`, `select`, `sub`, `function`, `type`, `openin`, `onerror`, `gameloop`, `include`, ... (17). |
| Diagnostics | As you type: syntax and block errors (`FOR` without `NEXT`...), missing `$INCLUDE` files. |
| Outline, go to definition, find references | SUB, FUNCTION, TYPE and fields, CONST, labels, line numbers; across `$INCLUDE` files. |
| Hover, completion | Keywords with syntax, standard library procedures with their module, your own symbols. |
| Format on save | On by default for QBasic files (`"[qbasic]": {"editor.formatOnSave": false}` to turn it off). |
| `QBNex: Run` (Ctrl+F5, play button) | Save, compile with `qb`, run in a terminal. `"qbnex.runMode": "qbRun"` uses `qb run` (Planned). |
| `QBNex: Compile` (Ctrl+Shift+B) | Compile; output in the *QBNex* panel. |
| Debug (F5) | See below. |

## Debugger

Press F5 on a `.bas` file (no `launch.json` needed). `qb-dap` compiles the
program with `qb`, runs it on the qb-exec VM (JIT off) and gives you:

- breakpoints by line in the launched file (a line without a statement moves
  to the next one), pause, continue, step over / into / out (per statement);
- the call stack of SUB/FUNCTION frames;
- variables of each frame and globals: numbers, strings, arrays (up to 1000
  elements shown), TYPE fields, expandable;
- watch and hover expressions: `name`, `name.field`, `name(i, j)`;
- program output in the Debug Console.

`launch.json` options: `program`, `qir` (a QIR file instead of compiling),
`compiler`, `cwd`, `stopOnEntry`, `input` (a file the program reads as
standard input).

Limits: breakpoints in `$INCLUDE`d files are not supported yet (QIR has no
file information yet); without `input`, INPUT sees end of file; GOSUB does
not add stack frames; watch expressions are variable paths only.

## Other editors

`qbnex lsp` and `qbnex dap` speak the standard protocols on stdin/stdout.
Example for Neovim (nvim-lspconfig style):

```lua
vim.lsp.config('qbnex', { cmd = { 'qbnex', 'lsp' }, filetypes = { 'basic', 'qbasic' } })
vim.lsp.enable('qbnex')
```

The formatter also works on its own: `qbnex fmt -w file.bas`
(see [commands](commands.md#qbnex-fmt-formatter)).
