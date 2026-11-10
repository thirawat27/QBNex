# qb-dap: debug adapter

Status: first version implemented (R13 debugger). This note records the
design, what works, and what still needs other parts of the port.

## Shape

```
VS Code  --DAP/stdio-->  qb-dap (adapter)  --loopback TCP, JSON lines-->  qb-dap --debuggee (child)
                                  ^                                            |  qb-exec VM + `debug` hook
                                  +------------- program stdout/stderr --------+  (stdin = launch "input" file)
```

- **Adapter** (`crates/qb-dap/src/adapter.rs`): DAP over stdio. On `launch`
  it gets QIR — `qir` given, or `qb <program.bas> --ir -o <temp exe>` and the
  QIR payload read back from the executable — writes it to a temp file and,
  after `configurationDone`, starts itself as the debuggee. Requests:
  initialize, launch, setBreakpoints, setExceptionBreakpoints (no-op),
  configurationDone, threads, stackTrace, scopes, variables, evaluate,
  continue, next, stepIn, stepOut, pause, terminate, disconnect. Events:
  initialized, stopped (entry/breakpoint/step/pause), output, exited,
  terminated.
- **Debuggee** (`debuggee.rs`): runs the program on the VM with the hook. Its
  stdin/stdout are the program's, so INPUT and PRINT never touch the DAP
  stream; output is forwarded as `output` events. When it stops it sends a
  snapshot (call stack, each frame's locals, globals) and waits for a resume
  command; breakpoint changes and pause arrive on a reader thread. If the
  adapter goes away the debuggee exits.
- **VM hook** (`crates/qb-exec/src/debug.rs`, feature `debug`, off by
  default): the dispatch loop calls `debug_statement(pc)` at the top of each
  instruction only when the feature is compiled in and a debugger is attached;
  it stops only at statement starts of the bytecode line table (`stmts`,
  from QIR `line N`). Attaching turns the JIT off. `rt/` and `lower.rs` are
  not touched: variable locations are found by repeating lower's allocation
  order (globals, statics, then per function params and locals; memory mode
  when the module has memory variables), and the unit tests check the result
  on real fixtures.
- **Stepping** (`Stepper`, unit-tested): step in = next statement; step over
  = next statement at the same or a shallower frame depth; step out = next
  statement at a shallower depth; a breakpoint fires when execution arrives
  on its line (not again for further statements of the same line).
- **Names**: QIR names are demangled (`$__INTEGER_A` -> `a%`,
  `$_SUB_BUMP_INTEGER_X` -> `x%`, `FUNC_FACT` -> `Fact`) and spelled as in the
  source (qb-syntax lexer).

## Limits and follow-ups

1. **Include files**: the QIR line table has one line number per statement
   but no file. Lines of `$INCLUDE`d code use the compiler's running numbering,
   so breakpoints work in the launched `.bas` only (others are reported as not
   verified). Needed from the emitter (R11a): a `file <hex path>` directive (or
   a file index on `line`) whenever the source file changes, so the VM line
   table can carry (file, line). The adapter already reports unverified
   breakpoints with a reason, so adding this is backward compatible.
2. Watches evaluate variable paths only (`name`, `name.field`, `name(i, j)`),
   not arbitrary expressions; editing variables is not supported.
3. GOSUB/RETURN does not create stack frames (only SUB/FUNCTION do), and
   ON ERROR handlers run in the module view as in the VM.
4. Arrays show at most 1000 elements per array in a stop snapshot.
5. INPUT reads the launch `input` file (or sees end of file); a
   `runInTerminal` launch for interactive programs is a later step.
6. Graphics programs run as usual (the window belongs to the debuggee).

Gate kept: without the `debug` feature the VM is unchanged; qb-dap enables
the feature only for itself.
