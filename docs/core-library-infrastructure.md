# QBNex Core Library Infrastructure

## Scope

This phase establishes the compiler and runtime foundation required for higher-level programming in QBNex while keeping the execution model close to classic QBasic.

Delivered layers:

- Root-based stdlib imports with Python-like dotted module paths via `'$IMPORT:'module.name'`
- Integrated `qbnex` core entrypoint under `source/stdlib/qbnex_stdlib.bas`
- Modular stdlib layout for OOP, collections, strings, system, I/O, math, and result helpers
- Compiler-backed native class lowering for `CLASS ... END CLASS`

## Import Model

Bundled libraries resolve from `source/stdlib/` relative to the compiler root.

Examples:

```basic
'$IMPORT:'strings.text'
'$IMPORT:'io.csv'
'$IMPORT:'qbnex'
```

`qbnex` maps to the integrated core and is designed to be imported at the top of files that define `TYPE`, `SUB`, `FUNCTION`, or `CLASS`.

## Native Class Syntax

The compiler now lowers high-level class declarations into classic BASIC-compatible structures during both prepass and main pass so generated code stays consistent across analysis and compilation.

Supported syntax:

```basic
CLASS Dog EXTENDS Animal IMPLEMENTS IPet
    Breed AS STRING * 32

    CONSTRUCTOR (petName AS STRING, breedName AS STRING)
        THIS.Name = petName
        THIS.Breed = breedName
    END CONSTRUCTOR

    FUNCTION Describe$ ()
        Describe$ = "Dog:" + RTRIM$(ME.Name)
    END FUNCTION
END CLASS
```

Lowering model:

- `CLASS` lowers to `TYPE`
- Every class receives `Header AS QBNex_ObjectHeader`
- `EXTENDS BaseType` flattens inherited fields into the derived `TYPE`
- Constructors and methods lower to generated global procedures with `self AS ClassType` injected as the first parameter
- `ME.` and `THIS.` lower to `self.`
- `object.method(...)` is rewritten to the lowered generated procedure for known class variables
- Function-style method return assignments are rewritten to the generated lowered symbol
- Class registration and interface registration are emitted through generated helper functions backed by the `QBNEX_*` runtime

Inherited members are addressable directly from derived classes, for example `ME.Name`.

## Standard Library Surface

Current bundled modules:

- `qbnex`
- `collections.list`
- `collections.stack`
- `collections.queue`
- `collections.set`
- `collections.dictionary`
- `strings.strbuilder`
- `strings.text`
- `sys.env`
- `sys.args`
- `sys.datetime`
- `io.path`
- `io.csv`
- `io.json`
- `math.numeric`
- `error.result`
- `oop.class`
- `oop.interface`

## Verified Examples

Compile-checked examples:

- `source/stdlib/examples/import_smoke.bas`
- `source/stdlib/examples/ecosystem_smoke.bas`
- `source/stdlib/examples/data_smoke.bas`
- `source/stdlib/examples/stdlib_demo.bas`
- `source/stdlib/examples/class_syntax_demo.bas`
- `source/stdlib/examples/runtime_smoke.bas`

## Current Boundary

The main remaining ergonomic gap is runtime execution of integrated-core examples as pure top-level smoke files when the file relies on top-imported `TYPE` and generated procedures. Those cases compile-check cleanly, while runtime smoke is currently easiest with function-only modules.
