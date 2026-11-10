# qb-dap: debug adapter plan (stub)

Status: plan only. Nothing is implemented yet; the VS Code extension
(`packaging/vscode`) does not register a debugger. This note fixes the shape
so the work can be picked up later (port-plan §9.4, "Tooling": debugger with
breakpoints, stepping and watches through qb-exec).

## Goal

`qb-dap`, a Debug Adapter Protocol server over stdio, so that VS Code (and any
DAP client) can run a `.bas` program under the qb-exec VM with:

- line breakpoints (with optional conditions and hit counts), pause, continue;
- step over / step into / step out at statement granularity;
- call stack (SUB/FUNCTION frames plus GOSUB return points);
- variables: module-level (SHARED) and locals of the selected frame, arrays
  (lazy, paged), UDT fields; watches and hover evaluation of simple
  expressions;
- program output in the debug console; `ON ERROR`/unhandled errors reported
  as stops with ERR/ERL.

Out of scope for the first version: data breakpoints, editing variables of
fixed-length string arrays, reverse debugging, attaching to a compiled
native executable (AOT output). Debug runs always use the VM with the JIT off.

## Pieces

| Piece | Where | Work |
| --- | --- | --- |
| Debug hooks in the VM | `crates/qb-exec` (`vm.rs`) | A `Debugger` trait object the VM consults only when a debug session is attached. Stops happen at statement starts, which the VM already knows from the `stmts` side table. |
| Source mapping | `qb-exec` bytecode + `qb-syntax` | Statement index -> (file, line) including `$INCLUDE`d files; breakpoint lines snap to the next statement start (qb-syntax tells which lines hold statements). |
| Variable view | `qb-exec` | Read-only views of the register banks (`nums`/`strs`/`arrs`) per frame, with names from the lowering's symbol table (needs keeping names in the module; today they are dropped). |
| Expression evaluation | `qb-exec` | Compile a watch expression with the same front end in the scope of the stopped frame, run it on a scratch frame with side-effect-free builtins only. |
| Adapter | new `crates/qb-dap` | DAP over stdio (Content-Length framing like qb-lsp; `serde_json`, pure Rust). Requests: initialize, launch, setBreakpoints, configurationDone, threads, stackTrace, scopes, variables, evaluate, continue, next, stepIn, stepOut, pause, disconnect. Events: initialized, stopped, output, terminated, exited. |
| VS Code side | `packaging/vscode` | `contributes.debuggers` type `qbnex` launching `qb-dap`, a default `launch.json` snippet, F5 runs under the debugger when a launch config exists (QBNex: Run stays the no-debug path). |

## VM hook design

- **Pause requests** reuse the event mechanism: the adapter thread sets a
  reserved bit in the VM's atomic event mask; the VM already checks it at
  safe points (backward jumps, NEXT, CALL), so pausing a busy loop costs
  nothing when not debugging.
- **Breakpoints and stepping** need a check at every statement start. Two
  options, to be measured:
  1. a debug flavour of the dispatch loop that compares the pc with the next
     statement start from `stmts` (no change to the bytecode);
  2. an explicit `Brk` instruction patched over the first instruction of a
     statement (needs a side table of the replaced instruction).
  Option 1 first: simpler, and debug runs have no speed target.
- **Step over/out** use the frame depth at the stop: stop at the next
  statement start whose depth is <= (over) or < (out) the recorded one.
  GOSUB counts as a frame for stepping and for the call stack.
- **Stopped state**: the VM thread blocks on a channel; the adapter thread
  answers stackTrace/variables/evaluate by sending closures to it, so the
  register banks are only touched from the VM thread.

## Milestones

1. VM: names kept in the module, statement -> (file, line) map, `Debugger`
   trait with stop-at-statement and pause; unit tests on the bytecode level.
2. `qb-dap`: launch, breakpoints, continue, next/stepIn/stepOut, stackTrace,
   scopes/variables (scalars, strings); stdio integration tests like
   `crates/qb-lsp/tests/stdio.rs`.
3. Arrays and UDTs in the variables view, evaluate for watches and hover,
   conditional breakpoints, hit counts.
4. VS Code: debugger contribution, launch configuration, docs in
   `packaging/vscode/README.md`.

Gate for each milestone: the conformance corpus still passes with no
debugger attached, and the hot loop benchmark (`cargo run --release -p qb-exec
--example bench`) shows no slowdown when not debugging.
