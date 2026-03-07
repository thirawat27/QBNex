---
name: qbnex-completeness-roadmap
description: Use this skill when continuing QBNex compiler/interpreter architecture, stability, and QBasic compatibility work. It summarizes what is already done, the remaining gaps, the preferred implementation order, the critical files, and the verification workflow.
---

# QBNex Completeness Roadmap

Use this skill when the task is to continue QBNex compiler/interpreter work toward:
- broader QBasic statement coverage
- fewer backend fallbacks
- fewer silent semantic mismatches
- stronger runtime stability

This skill is a continuation map, not end-user documentation.

## Current Architecture

QBNex now has three practical execution paths:
- native compile via `native_codegen`
- VM/interpreter via `vm_engine`
- bundled-VM executable fallback via `cli_tool`

Selection is capability-driven through `syntax_tree::unsupported_statements(...)`, not by ad hoc feature guessing.

## What Is Already Done

Treat these as baseline, not TODO:

- quiet compile output in CLI
- capability-driven backend selection and bundled VM fallback
- VM support for user `SUB/FUNCTION` execution
- VM by-reference style copy-back for variable arguments and array-element arguments, including nested call chains and string array elements
- VM procedure-call lowering now keeps function/builtin call arguments as by-value expressions instead of misclassifying them as by-reference variable/array targets
- VM fixed-length string metadata/runtime enforcement now covers scalar `DIM`, string arrays, fixed-length procedure parameters, and fixed-length function result slots
- VM cursor/command compatibility semantics for `CSRLIN`, `POS`, and `COMMAND$`
- VM segmented memory compatibility for `DEF SEG`, `PEEK`, and `POKE`
- VM file-backed memory compatibility for `BLOAD` and `BSAVE`
- VM pseudo port-I/O compatibility for `INP` and `OUT`
- VM pseudo-address compatibility for `VARPTR`, `VARSEG`, `SADD`, and `VARPTR$`
- VM and native non-blocking `INKEY$` compatibility paths
- HAL/VM graphics helper paths no longer emit debug marker stdout for `VIEW/WINDOW/DRAW/PALETTE` operations during normal execution
- VM empty `SHELL` path is now a quiet no-op instead of leaking an interactive-shell marker into stdout
- VM negative `SLEEP` no longer fakes a one-second delay in non-interactive mode; it uses a key-wait path only when console interaction is actually available
- native pseudo-memory compatibility for `DEF SEG`, `PEEK`, `VARPTR`, `VARSEG`, `SADD`, `VARPTR$`, `CSRLIN`, and `POS`
- native pseudo port-I/O compatibility for `INP` and `OUT`
- printer compatibility aliases for `LPRINT` and `LPOS`
- statement-level `POKE` support wired through parser, VM, and native codegen
- statement-level `WAIT`, `BLOAD`, and `BSAVE` support wired through parser, VM, and native codegen
- native zero-arg builtin normalization for `DATE$`, `TIME$`, `COMMAND$`, and similar identifier-tokenized calls
- native basic string-array `DIM` allocation for bound/lookup semantics such as `LBOUND/UBOUND`
- native string-array `GET/PUT` support for element targets such as `B$(1)`
- parser support for line-number statements, `FOR EACH`, declared function calls, fixed-length `TYPE` fields, and raw `DATA` values
- native support for:
  - top-level, nested branch, and loop-local control-flow cases for `GOTO`, `GOSUB`, `RETURN`, `ON ... GOTO/GOSUB`
  - `ON ERROR` / `RESUME` top-level and loop-local model
  - `ON TIMER` top-level and loop-local model
  - `CHAIN`, `SHELL`, `PRINT USING`, `SOUND`, `PLAY`
  - text-mode file I/O
  - random file path with `FIELD/LSET/RSET`
  - many `GET/PUT` forms
  - nested UDT record flattening for primitive and fixed-length string leaves
  - array-of-UDT native storage/codegen for primitive and fixed-length string leaves, including `GET/PUT` on record elements and subfields
  - `STATIC` for native `SUB/FUNCTION`, including UDT-heavy locals and local array persistence across early exits
- native and VM `DATA/READ` no longer silently corrupt string data
- native structured `GET/PUT` now caches array-of-UDT index expressions once per statement, avoiding repeated side effects and silent misreads/miswrites on subscripted record targets
- native scalar/string array-element `GET/PUT` now also caches index expressions once per statement, avoiding repeated side effects on subscripted binary targets
- native capability gating now explicitly rejects multi-dimensional array-backed `GET/PUT` targets that the current flat array codegen cannot execute correctly, preventing silent native miscompile and preserving VM fallback
- native random-file `FIELD` string variables now honor declared field widths during direct `GET/PUT`, instead of using the current string length and risking zero-byte reads
- parser/AST now preserve `DIM/REDIM/PARAM AS STRING * n` fixed-length metadata, and native assignment plus binary/file paths honor that width for scalar and array string variables
- native `SUB/FUNCTION` parameter initialization and by-reference copy-back now preserve fixed-length string widths instead of widening silently across call boundaries
- parser/AST now also preserve `FUNCTION ... AS STRING * n` return-width metadata, and native function return paths honor that declared width
- native user `SUB/FUNCTION` calls now also copy back numeric and fixed-length string array-element arguments instead of only scalar variables
- native user `SUB/FUNCTION` calls now also copy back flattened UDT leaf fields, including array-of-UDT numeric and fixed-length string leaves

## Remaining High-Value Gaps

Prioritize these in order.

### 1. Native Structured `GET/PUT` Beyond Current Flattened Cases

Current native support is broad but not complete.

Still likely incomplete:
- deeper structured forms that are not simple variable/subrecord/primitive leaf access
- complex array-backed structured reads/writes
- unsupported expression forms currently rejected in backend validation
- full audit of structured target evaluation order outside the array/subscript caching cases already fixed
- remaining fixed-width string edge cases beyond current scalar/array/procedure/function-return/`FIELD`/UDT-leaf metadata-driven paths

Important rule:
- do not weaken validation until codegen/runtime semantics are correct

Critical files:
- `native_codegen/src/codegen.rs`
- `native_codegen/src/codegen/program.rs`
- `syntax_tree/src/backend.rs`

Acceptance:
- add positive tests for every newly-supported form
- keep negative tests for truly unsupported expression targets

### 2. Native `STATIC` Semantics Beyond Current Local Persistence

`STATIC` storage now persists local arrays/numbers/strings across calls, but semantics should be reviewed for:
- future loop-frame work

Critical files:
- `native_codegen/src/codegen/program.rs`
- `native_codegen/tests/codegen_test.rs`

### 3. VM Procedure Semantics Audit

VM `SUB/FUNCTION` execution and by-ref copy-back now work, but this area should be hardened further.

Review:
- mixed by-value / by-reference cases
- function result slot edge cases
- remaining call semantics where structured targets deeper than flattened primitive/fixed-string field leaves may still need explicit treatment

Critical files:
- `vm_engine/src/compiler.rs`
- `vm_engine/src/runtime.rs`
- `vm_engine/src/opcodes.rs`
- `vm_engine/tests/vm_test.rs`

### 4. Remaining Placeholder Compatibility Functions

The VM no longer fakes `CSRLIN`, `POS`, or `COMMAND$`, but lower-level compatibility functions still need explicit treatment.

Examples to audit:
- low-level helpers whose semantics are still intentionally approximate rather than hardware-accurate
- any remaining keyboard/input compatibility behavior that needs platform-specific refinement
- any builtin still acting as a stub rather than a deliberate compatibility surface

Important:
- decide explicitly whether each should remain a compatibility placeholder or gain real semantics
- if placeholder behavior remains, document it in code and ensure no silent corruption follows from using it

Critical files:
- `vm_engine/src/runtime.rs`
- `vm_engine/src/builtin_functions.rs`

## Preferred Work Order

When continuing this project, use this order:

1. Expand native structured `GET/PUT` support now that loop-local control flow is stable.
2. Audit native `STATIC` semantics against more cases.
3. Harden VM procedure semantics with more regression tests.
4. Revisit the remaining placeholders and low-priority compatibility functions.

## Implementation Rules

- Prefer capability-driven correctness over broad unsupported removals.
- Never allow silent no-op or silent miscompile when a statement is still unsupported.
- If a feature is not correct in native, keep fallback to VM/bundled VM.
- Add a regression test before or with each new capability.
- Verify with both unit-level and end-to-end CLI runs when the feature affects runtime semantics.

## Key Files By Area

Backend selection and fallback:
- `cli_tool/src/main.rs`
- `syntax_tree/src/backend.rs`

Parser and AST:
- `syntax_tree/src/parser.rs`
- `syntax_tree/src/ast_nodes.rs`
- `syntax_tree/tests/parser_test.rs`

Native backend:
- `native_codegen/src/codegen.rs`
- `native_codegen/src/codegen/program.rs`
- `native_codegen/src/codegen/expr.rs`
- `native_codegen/src/codegen/state.rs`
- `native_codegen/tests/codegen_test.rs`

VM backend:
- `vm_engine/src/compiler.rs`
- `vm_engine/src/runtime.rs`
- `vm_engine/src/opcodes.rs`
- `vm_engine/tests/vm_test.rs`

Semantic analysis:
- `analyzer/src/type_checker.rs`
- `analyzer/src/scope.rs`

## Verification Workflow

Run these after meaningful changes:

```powershell
cargo check --workspace --all-targets
cargo test -p syntax_tree
cargo test -p native_codegen
cargo test -p vm_engine
```

For runtime changes, also run targeted CLI samples:

```powershell
cargo run -p cli_tool -- -x target\tmp_vm_zeroarg_function.bas
cargo run -p cli_tool -- -x target\tmp_vm_byref_sub.bas
cargo run -p cli_tool -- -x target\tmp_vm_byref_func.bas
cargo run -p cli_tool -- -c target\tmp_native_static.bas
cmd /c .\tmp_native_static.exe
```

Keep adding focused `.bas` files in `target/` for new regression scenarios.

## Sample Scenarios Worth Keeping

Use or extend these patterns when testing:

- `DATA` + `READ` with mixed string/numeric values
- `SUB/FUNCTION` by-reference mutation
- `STATIC` sub/function local state persistence
- nested `IF` / `SELECT CASE` control flow
- loop-local jumps
- random/binary file I/O
- nested UDT `GET/PUT`
- `ON ERROR` resume paths
- timer-driven control flow

## Definition Of Done For A New Capability

A capability is only done when all of these are true:

- parser accepts the syntax
- analyzer does not regress existing valid programs
- capability gate advertises support accurately
- backend execution is correct
- tests cover both support and rejection boundaries
- CLI runtime path behaves the same as direct backend expectation

## Current Most Important Next Task

If you resume work without more guidance, start here:

1. expand native structured `GET/PUT` to array-of-UDT and deeper binary record forms
2. audit remaining structured target evaluation-order/side-effect cases beyond current subscript caching
3. audit native `STATIC` with UDT-heavy procedures and early-exit paths
4. add focused regression scenarios for:
   - subscripted nested subrecord `GET/PUT`
   - side-effecting record-target expressions beyond the cached array-index path
   - side-effecting scalar/string array target expressions beyond the cached subscript path
   - fixed-width string edge cases in paths that still infer width indirectly rather than from explicit metadata
   - `STATIC` procedures that mutate nested UDT fields
   - mixed VM by-value / by-reference string and array arguments
5. audit the remaining compatibility helpers that are still approximate rather than hardware-accurate, and either implement or mark them as intentional compatibility stubs
