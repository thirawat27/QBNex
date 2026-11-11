# Developer reference

How to build, test and change QBNex: [CONTRIBUTING.md](../../CONTRIBUTING.md).
User documentation: [docs/manual](../manual/README.md).

| Document | Contents |
| --- | --- |
| [qir.md](qir.md) | The QIR text format: types, instructions, the builtin table. The crate `crates/compiler/ir` is normative where they differ. |
| [deviations.md](deviations.md) | The `L-xx` (legacy defects) and `D-xx` (intentional deviations) IDs used in code comments, conformance cases and the manual. |
| [ffi-float80.md](ffi-float80.md) | `DECLARE LIBRARY` calls (`crates/engine/ffi`) and the software 80-bit `_FLOAT` (`crates/runtime/float80`). |
| [3d-api.md](3d-api.md) | Design of the high-level 3D API (`_MESH`, `_CAMERA`, `_LIGHT`). |

## Pipeline

```
program.bas --(compiler/qbnex.bas, run as qb.exe)--> program.qir
program.qir --appended to a copy of qb-exec--------> program.exe
program.exe: qb-exec loads its own QIR -> VM (crates/engine/exec)
             hot functions -> Cranelift JIT (crates/engine/jit)
             runtime calls -> qb-runtime, qb-audio, qb-ext, qb-gpu (crates/runtime/*)
```

The compiler is BASIC and is compiled by itself. `bootstrap/qbnex.qir` is that compiler
already in QIR; the setup scripts pack it into `qb-exec` (stage 0), which compiles
`compiler/qbnex.bas` into `qb`. `scripts/selfhost_fixpoint.*` checks that the next two
generations produce the same QIR and that it equals `bootstrap/qbnex.qir`.
