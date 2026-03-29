<div align="center">
  <img src="assets/QBNex.ico" alt="QBNex Logo" width="256" height="256">
  
  # QBNex
  
  **Modern QBasic/QuickBASIC Compiler and Interpreter**
  
  > A modern QBasic/QuickBASIC compiler and interpreter written in Rust, bringing the classic BASIC programming experience to modern systems with enhanced performance and native code generation capabilities.
  
</div>

---

## Table of Contents

- [About](#about)
- [Features](#features)
- [Project Architecture](#project-architecture)
- [System Requirements](#system-requirements)
- [Installation](#installation)
- [Usage](#usage)
  - [Default Mode (Compile & Run)](#default-mode-compile--run)
  - [Run Program (Interpreter)](#run-program-interpreter)
  - [Compile Only](#compile-only)
  - [Command-Line Options](#command-line-options)
- [Code Examples](#code-examples)
- [Supported QBasic Commands](#supported-qbasic-commands)
  - [Control Flow](#control-flow)
  - [Variables & Data Types](#variables--data-types)
  - [Input/Output](#inputoutput)
  - [String Functions](#string-functions)
  - [Math Functions](#math-functions)
  - [Type Conversion](#type-conversion)
  - [Array Operations](#array-operations)
  - [File I/O](#file-io)
  - [Graphics & Sound](#graphics--sound)
  - [System & Memory](#system--memory)
  - [Error Handling](#error-handling)
  - [Advanced Features](#advanced-features)
- [Development](#development)
- [Contributing](#contributing)
- [License](#license)

---

## About

**QBNex** (QuickBasic Nexus) is a modern QBasic/QuickBASIC compiler and interpreter built with Rust. It brings the classic QBasic programming experience to modern operating systems with enhanced performance, native compilation, and comprehensive language support.

QBNex supports 150+ QBasic/QB64 keywords and functions, making it compatible with most legacy QBasic programs while providing modern features like fast bytecode interpretation, native executable generation, and automatic graphics/sound detection.

---

## Features

- **Dual Execution Modes**
  - Fast bytecode interpreter for quick testing and development
  - Native compilation to standalone executables via Rust compiler
  - Automatic mode selection based on program features

- **Comprehensive Language Support**
  - 150+ QBasic/QB64 keywords and functions
  - Full support for classic QBasic syntax
  - QB64-style commented metacommands and recursive `$INCLUDE` expansion
  - User-defined types (TYPE...END TYPE)
  - Subroutines and functions with parameters
  - Multi-dimensional arrays with REDIM PRESERVE

- **Advanced Graphics & Sound**
  - Automatic detection of graphics/sound features
  - SCREEN modes with VGA graphics support
  - Drawing primitives (LINE, CIRCLE, PAINT, DRAW)
  - Image manipulation (GET/PUT)
  - Sound synthesis (SOUND, PLAY, BEEP)
  - Automatic Cargo compilation for graphics-enabled programs

- **Robust Type System**
  - Static type checking and analysis
  - Scope resolution and symbol table management
  - Type inference and conversion
  - OPTION \_EXPLICIT support for strict variable declaration

- **Compiler Diagnostics**
  - Source-aware tokenizer/parser diagnostics rendered with `miette`
  - Highlighted source snippets for syntax failures in the CLI
  - Central parser/frontend and native-backend seams for staged library adoption
  - Experimental `Chumsky` frontend and `Cranelift-JIT` backend paths exposed through CLI flags
  - Production surface validation and preview gating in the CLI

- **File System Operations**
  - Sequential, random, and binary file access
  - Directory operations (MKDIR, CHDIR, RMDIR)
  - File management (KILL, NAME...AS, FILES)
  - Record-based I/O with FIELD, LSET, RSET

- **Performance Optimizations**
  - Optimized bytecode virtual machine
  - Fast parsing with integrated tokenizer
  - Release builds with LTO and strip optimizations
  - Minimal executable size (no debug symbols)

---

## Project Architecture

QBNex is organized as a Rust workspace with modular crates, each handling a specific phase of compilation or execution

```
QBNex/
├── cli_tool/          Command-line interface and main entry point
│   ├── main.rs        Argument parsing, mode selection, compilation pipeline
│   └── feature_check.rs  Graphics/sound feature detection
├── tokenizer/         Lexical analysis (source code → tokens)
│   ├── scanner.rs     Character-level scanning
│   └── tokens.rs      Token definitions and types
├── syntax_tree/       Syntax analysis (tokens → Abstract Syntax Tree)
│   ├── frontend.rs    Production frontend abstraction seam
│   ├── parser.rs      Recursive descent parser
│   └── ast_nodes.rs   AST node definitions
├── analyzer/          Semantic analysis (type checking, scope resolution)
│   ├── scope.rs       Symbol table and scope management
│   └── type_checker.rs  Type inference and validation
├── vm_engine/         Bytecode compilation and virtual machine
│   ├── compiler.rs    AST → Bytecode compiler
│   ├── runtime.rs     Bytecode interpreter/VM
│   ├── opcodes.rs     Bytecode instruction set
│   └── builtin_functions.rs  Built-in function implementations
├── native_codegen/    Native code generation (AST → Rust → executable)
│   ├── backend.rs     Native backend abstraction seam
│   ├── codegen.rs     Rust code generator
│   ├── llvm_builder.rs  LLVM IR generation (future)
│   └── linker.rs      Executable linking
├── hal_layer/         Hardware Abstraction Layer
│   ├── file_io.rs     Cross-platform file operations
│   ├── vga_graphics.rs  Graphics rendering
│   └── sound_synth.rs   Sound synthesis
├── core_types/        Shared data structures
│   ├── data_types.rs  QBasic type system
│   ├── errors.rs      Error handling
│   └── memory_map.rs  Memory management
└── test/              Centralized test suites and BASIC fixtures
    ├── integration/   Cross-crate integration and CLI regression tests
    └── fixtures/      Shared BASIC source fixtures
```

The production compiler still uses the in-tree tokenizer and recursive-descent parser for correctness, but the frontend and native backend seams now make it practical to stage alternate implementations such as `Chumsky`, `Cranelift`, or `Inkwell` behind the same pipeline. The current adoption plan lives in `docs/architecture/rust-compiler-libraries-roadmap.md`.

**Compilation Pipeline**

```
┌─────────────┐
│ Source Code │  hello.bas
│   (.bas)    │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Tokenizer  │  Lexical Analysis
│  (scanner)  │  "PRINT" → TokenPrint
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Parser    │  Syntax Analysis
│ (syntax_tree)│  Tokens → AST
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Analyzer   │  Semantic Analysis
│ (type check)│  Scope & Type Validation
└──────┬──────┘
       │
       ├─────────────────┐
       │                 │
       ▼                 ▼
┌─────────────┐   ┌─────────────┐
│  Bytecode   │   │   Native    │
│  Compiler   │   │  CodeGen    │
│ (vm_engine) │   │ (Rust code) │
└──────┬──────┘   └──────┬──────┘
       │                 │
       ▼                 ▼
┌─────────────┐   ┌─────────────┐
│  VM Runtime │   │   rustc     │
│  (Execute)  │   │  Compiler   │
└─────────────┘   └──────┬──────┘
                         │
                         ▼
                  ┌─────────────┐
                  │  Executable │
                  │   (.exe)    │
                  └─────────────┘
```

**Execution Modes**

1. **Default Mode (Compile & Run)** - Like QB64
   - Parses the fully expanded source tree, including commented QB64 `$INCLUDE` directives
   - Builds a native executable when the native backend fully supports the program
   - Falls back to a VM-backed executable when the VM is the compatible backend
   - Automatically runs the compiled program

2. **Run Program (-x flag)**
   - Builds a VM-backed executable and runs it immediately
   - Leaves the runnable file in the working directory
   - Honors `-o` so you can keep the generated runner under an explicit filename
   - Useful when VM compatibility is needed but you still want an executable artifact
   - Supports `--frontend chumsky` and `--native-backend cranelift-jit` for preview alternate paths when explicitly opted in

3. **Compile-Only Mode (-c flag)**
   - Generates a standalone executable without running it
   - Uses native codegen when possible, otherwise emits a VM-backed executable
   - Optimized release build
   - Can emit a preview Cranelift-backed runner executable for the supported subset via `--native-backend cranelift-jit`

---

## System Requirements

| Component        | Requirement                  |
| ---------------- | ---------------------------- |
| Operating System | Windows 10/11, Linux, macOS  |
| Rust Toolchain   | 1.75 or newer (2021 edition) |
| Cargo            | Included with Rust           |
| Memory           | 512 MB RAM minimum           |
| Disk Space       | 100 MB for installation      |

**Optional Dependencies**

- **For Graphics Programs** Cargo will automatically download `minifb` crate (0.24+)
- **For Native Compilation** rustc must be in PATH

---

## Installation

### 1. Install Rust Toolchain

Download and install Rust from [https//rustup.rs](https//rustup.rs)

**Windows**

```powershell
# Download and run rustup-init.exe
# Or use winget
winget install Rustlang.Rustup
```

**Linux/macOS**

```bash
curl --proto '=https' --tlsv1.2 -sSf https//sh.rustup.rs | sh
```

Verify installation

```bash
rustc --version
cargo --version
```

### 2. Clone Repository

```bash
git clone https//github.com/thirawat27/QBNex.git
cd QBNex
```

### 3. Build Project

**Release Build (Recommended)**

```bash
cargo build --release
```

The executable will be located at

- Windows `target\release\qb.exe`
- Linux/macOS `target/release/qb`

**Debug Build (for development)**

```bash
cargo build
```

### 4. Run with Docker

For containerized usage on Linux or in CI environments, build the bundled image

```bash
docker build -t qbnex .
```

Run the CLI from the current working directory

```bash
docker run --rm -it -v "$PWD:/workspace" -w /workspace qbnex -x test/fixtures/basic/test_all.bas
```

Notes

- The Docker image is intended for CLI/interpreter/native compilation workflows
- Graphics programs may still require host GUI/display integration outside the container

### 5. Install to System (Optional)

**Windows**

```powershell
# Copy to a directory in PATH
copy target\release\qb.exe C\Windows\System32\

# Or add to PATH
$envPATH += ";$PWD\target\release"
```

**Linux/macOS**

```bash
# Copy to /usr/local/bin
sudo cp target/release/qb /usr/local/bin/

# Or add to PATH in ~/.bashrc or ~/.zshrc
export PATH="$PATH$HOME/QBNex/target/release"
```

### 6. Verify Installation

```bash
qb --version
qb --help
```

### GitHub Actions Artifacts

The repository includes a multi-OS GitHub Actions workflow at [ci.yml](./.github/workflows/ci.yml) that

- runs `cargo test -q` on Windows, Linux, and macOS
- builds the release `qb` binary on each OS
- uploads per-OS build artifacts for download from the workflow run page
- verifies that the Docker image builds successfully on Linux

---

## Usage

### Default Mode (Compile & Run)

The default behavior compiles your program to an executable and runs it immediately (similar to QB64)

```bash
qb myprogram.bas
```

This will

1. Parse and analyze the source code
2. Expand commented QB64 `$INCLUDE` directives relative to the including file
3. Select the compatible backend (native or VM-backed executable)
4. Compile the executable automatically
5. Execute the program automatically
6. Display execution time

**With Graphics/Sound**

```bash
qb graphics_demo.bas
```

QBNex automatically detects graphics/sound features and switches to Cargo compilation mode with the `minifb` dependency.

### Run Program (Interpreter)

Build and run a VM-backed executable for the program

```bash
qb -x myprogram.bas
```

**Behavior**

- Uses the VM-compatible execution path
- Creates a runnable output file in the working directory
- Runs that executable immediately

**Quiet mode (suppress output)**

```bash
qb -x myprogram.bas -q
```

### Compile Only

Generate a standalone executable without running it

```bash
qb -c myprogram.bas
```

This creates `myprogram.exe` (Windows) or `myprogram` (Linux/macOS).

**Custom output filename**

```bash
qb -c myprogram.bas -o custom_name.exe
```

**Compilation features**

- Optimized release build (opt-level 3)
- LTO (Link-Time Optimization) enabled
- Debug symbols stripped
- Small executable size

### QB64 Includes

QBNex expands QB64-style include directives before parsing. Both direct and commented directives are supported, for example

```basic
'$INCLUDE:'utilities\strings.bas'
```

Include paths are resolved relative to the file that contains the directive, and nested includes are expanded recursively.
If you compile a project fragment that is only meant to be included by a larger root file, the CLI now reports that relationship explicitly instead of only surfacing the downstream backend error.
If that fragment has a single owning root, QBNex retries through the root automatically while still keeping the output filename based on the fragment you actually invoked.
If `-o` points at a nested path, QBNex now creates the parent output directories automatically for native and VM-backed build paths.
If `-o` points at an existing directory, QBNex places the generated executable in that directory using the source file stem.

### Command-Line Options

```
USAGE
    qb [OPTIONS] [FILE]

ARGUMENTS
    FILE                   Source file (.bas) to compile and run

OPTIONS
    -c                     Compile FILE to .exe only (do not run)
    -o <OUTPUT>            Set output filename  (default <FILE>.exe)
    -x                     Build a VM-backed executable and run it
    --frontend <NAME>      Select parser frontend: classic, chumsky
    --native-backend <NAME> Select native backend: rust, llvm-ir, cranelift-jit
    --allow-preview        Allow preview frontend/backend paths explicitly
    --list-pipelines       Show production/preview pipeline status and exit
    --explain-pipeline     Explain the selected pipeline for FILE and exit
    --validate-release     Run production-surface validation checks and exit
    --validate-pipeline    Run the release fixtures against the selected pipeline
    -e                     Enable OPTION _EXPLICIT (force variable declaration)
    -w                     Show warnings
    -q                     Quiet mode (suppress non-error output)
    -m                     Monochrome output
    -h, --help             Show this help message
    -v, --version          Show version

EXAMPLES
    qb hello.bas           Compile & run hello.bas (default)
    qb -x hello.bas        Build and run hello.bas via the VM runner
    qb --frontend chumsky --allow-preview -x hello.bas
                           Parse and run with the preview Chumsky frontend
    qb --frontend chumsky --native-backend cranelift-jit --allow-preview -x hello.bas
                           Parse and run with the preview Chumsky + Cranelift-JIT pipeline
    qb -x -o app.exe a.bas Build, run, and keep the VM runner as app.exe
    qb -c hello.bas        Compile  -->  hello.exe
    qb -c -o out.exe a.bas Compile  -->  out.exe
    qb -e main.bas         Compile & run with forced variable declaration
    qb --explain-pipeline main.bas
                           Explain which runtime/backend path QBNex will use
    qb --list-pipelines    Show frontend/backend stability levels
    qb --validate-release  Validate the production compiler surface
    qb --validate-pipeline --frontend classic
                           Validate the release fixtures with the selected pipeline
```

**Preview Alternate Pipelines**

- `--frontend chumsky`
  Uses the preview Chumsky-based frontend. The current supported subset is intentionally small: `PRINT`, `LET`/simple assignment, `END`, variables, string literals, and basic arithmetic. This path requires `--allow-preview`.
- `--native-backend cranelift-jit`
  Builds a runnable executable that embeds the source and executes the supported subset through Cranelift JIT at runtime. This path is currently limited to simple top-level text-mode programs with `PRINT`, numeric assignments, `END`, and basic arithmetic. This path requires `--allow-preview`.

**Production Validation**

- `qb --validate-release`
  Runs a fixture-driven validation pass over the production compiler surface: the production frontend, semantic analysis, type checking, VM bytecode compilation, runtime-backend selection, and the production native backend across text-mode, graphics-mode, file-I/O, and VM-fallback BASIC fixtures. Fixtures with companion `.out` files are also executed in-process through the VM, so release validation checks deterministic runtime output without rebuilding throwaway executables for every fixture. The command prints `[n/total]` progress lines plus a summary of graphics/runtime-output/VM-fallback coverage. Use this before tagging or packaging a release.
- `qb --validate-pipeline --frontend <NAME> [--native-backend <NAME>] [--allow-preview]`
  Runs the same release fixtures against the selected pipeline, so preview frontends and backends can only graduate after they pass the same regression set as the production path, including the shared `.out` runtime-output checks. It prints the same per-fixture progress and coverage summary as `--validate-release`. Failures report the exact fixture that broke and the backend/frontend gap that caused it.
- `qb --list-pipelines`
  Prints the current frontend/native-backend matrix with their production or preview status, plus the active release-fixture inventory and validation coverage counts, so release tooling and operators can inspect the active compiler surface directly from the binary.
- `qb --explain-pipeline FILE`
  Explains how QBNex classifies a BASIC file: which frontend is active, whether preview gating blocks it, whether the program will run natively or via VM fallback, and which native gaps caused that decision.
- The ignored QB64 source regression suite now sweeps every current `*.bas` file under `qb64/source/`, while a companion fragment-promotion regression keeps directly-invoked include fragments covered through their owning root program.

### Environment Variables

**QBNEX_EXPLICIT**

- Set to "1" to enable OPTION \_EXPLICIT globally
- Forces all variables to be declared before use

```bash
# Windows
set QBNEX_EXPLICIT=1
qb myprogram.bas

# Linux/macOS
export QBNEX_EXPLICIT=1
qb myprogram.bas
```

---

## Code Examples

### Hello World

```basic
' hello.bas
PRINT "Hello, World!"
PRINT "Welcome to QBNex!"
```

```bash
qb hello.bas
```

### Variables and Math

```basic
' calc.bas
DIM a AS INTEGER
DIM b AS INTEGER
DIM result AS SINGLE

a = 10
b = 20

PRINT "Addition "; a; " + "; b; " = "; a + b
PRINT "Subtraction "; a; " - "; b; " = "; a - b
PRINT "Multiplication "; a; " * "; b; " = "; a * b
PRINT "Division "; a; " / "; b; " = "; a / b
PRINT "Modulo "; a; " MOD 3 = "; a MOD 3
PRINT "Power "; a; " ^ 2 = "; a ^ 2
PRINT "Square root SQR("; a; ") = "; SQR(a)
```

### Loops and Conditionals

```basic
' loop.bas
PRINT "Even and Odd Numbers (1-20)"
PRINT

FOR i = 1 TO 20
    IF i MOD 2 = 0 THEN
        PRINT i; " is even"
    ELSE
        PRINT i; " is odd"
    END IF
NEXT i

PRINT
PRINT "Countdown"
count = 10
DO WHILE count > 0
    PRINT count
    count = count - 1
LOOP
PRINT "Blast off!"
```

### Subroutines and Functions

```basic
' functions.bas
DECLARE SUB Greet (name$)
DECLARE FUNCTION Square# (x AS SINGLE)
DECLARE FUNCTION Factorial& (n AS INTEGER)

CALL Greet("Alice")
CALL Greet("Bob")

PRINT "Square of 5 "; Square(5)
PRINT "Square of 12.5 "; Square(12.5)
PRINT "Factorial of 5 "; Factorial(5)
PRINT "Factorial of 10 "; Factorial(10)

END

SUB Greet (name$)
    PRINT "Hello, "; name$; "!"
    PRINT "Welcome to QBNex!"
    PRINT
END SUB

FUNCTION Square# (x AS SINGLE)
    Square = x * x
END FUNCTION

FUNCTION Factorial& (n AS INTEGER)
    IF n <= 1 THEN
        Factorial = 1
    ELSE
        Factorial = n * Factorial(n - 1)
    END IF
END FUNCTION
```

### Arrays and Data Processing

```basic
' arrays.bas
OPTION BASE 1
DIM numbers(10) AS INTEGER
DIM total AS INTEGER
DIM average AS SINGLE

' Fill array
FOR i = 1 TO 10
    numbers(i) = i * 10
NEXT i

' Calculate sum
total = 0
FOR i = 1 TO 10
    total = total + numbers(i)
NEXT i

average = total / 10

PRINT "Numbers ";
FOR i = 1 TO 10
    PRINT numbers(i);
    IF i < 10 THEN PRINT ", ";
NEXT i
PRINT
PRINT "Total "; total
PRINT "Average "; average

' Dynamic arrays
REDIM dynamic(5) AS INTEGER
dynamic(1) = 100
REDIM PRESERVE dynamic(10) AS INTEGER
PRINT "Dynamic array element "; dynamic(1)
```

### File I/O

```basic
' fileio.bas
DIM line$ AS STRING
DIM count AS INTEGER

' Write to file
OPEN "data.txt" FOR OUTPUT AS #1
PRINT #1, "Line 1 Hello"
PRINT #1, "Line 2 World"
PRINT #1, "Line 3 QBNex"
CLOSE #1

PRINT "File written successfully!"
PRINT

' Read from file
OPEN "data.txt" FOR INPUT AS #1
count = 0
WHILE NOT EOF(1)
    LINE INPUT #1, line$
    count = count + 1
    PRINT "Read line "; count; " "; line$
WEND
CLOSE #1

' Append to file
OPEN "data.txt" FOR APPEND AS #1
PRINT #1, "Line 4 Appended"
CLOSE #1

PRINT
PRINT "File operations completed!"

' Clean up
KILL "data.txt"
```

### User-Defined Types

```basic
' types.bas
TYPE Player
    Name AS STRING * 20
    Score AS LONG
    Health AS SINGLE
    Level AS INTEGER
END TYPE

DIM player1 AS Player
DIM player2 AS Player

player1.Name = "Alice"
player1.Score = 1500
player1.Health = 100.0
player1.Level = 5

player2.Name = "Bob"
player2.Score = 2300
player2.Health = 85.5
player2.Level = 7

PRINT "Player 1"
PRINT "  Name "; player1.Name
PRINT "  Score "; player1.Score
PRINT "  Health "; player1.Health
PRINT "  Level "; player1.Level
PRINT

PRINT "Player 2"
PRINT "  Name "; player2.Name
PRINT "  Score "; player2.Score
PRINT "  Health "; player2.Health
PRINT "  Level "; player2.Level
```

### Graphics Example

```basic
' graphics.bas
SCREEN 12  ' 640x480, 16 colors
COLOR 15, 1  ' White on blue

CLS
PRINT "QBNex Graphics Demo"
PRINT "Press any key to continue..."
SLEEP

' Draw shapes
LINE (50, 50)-(300, 200), 14, B  ' Yellow box
CIRCLE (400, 125), 75, 12  ' Red circle
LINE (100, 300)-(500, 400), 10, BF  ' Filled green rectangle

' Draw pattern
FOR i = 0 TO 639 STEP 20
    LINE (i, 0)-(639 - i, 479), 9
NEXT i

LOCATE 25, 1
PRINT "Graphics demo complete. Press any key..."
SLEEP

SCREEN 0  ' Return to text mode
```

---

## Supported QBasic Commands

QBNex supports 150+ QBasic/QB64 keywords and functions. Below is a comprehensive reference organized by category.

### Control Flow

| Command            | Description           | Example                              |
| ------------------ | --------------------- | ------------------------------------ |
| `IF...THEN...ELSE` | Conditional execution | `IF x > 0 THEN PRINT "Positive"`     |
| `ELSEIF`           | Additional condition  | `ELSEIF x < 0 THEN PRINT "Negative"` |
| `END IF`           | End conditional block | `END IF`                             |
| `SELECT CASE`      | Multi-way branch      | `SELECT CASE x`                      |
| `CASE`             | Case branch           | `CASE 1, 2, 3`                       |
| `CASE IS`          | Conditional case      | `CASE IS > 10`                       |
| `CASE TO`          | Range case            | `CASE 1 TO 10`                       |
| `END SELECT`       | End select block      | `END SELECT`                         |
| `FOR...TO...STEP`  | Counted loop          | `FOR i = 1 TO 10 STEP 2`             |
| `NEXT`             | End for loop          | `NEXT i`                             |
| `WHILE...WEND`     | While loop (legacy)   | `WHILE x < 10`                       |
| `DO...LOOP`        | Do loop               | `DO WHILE x < 10`                    |
| `DO WHILE`         | Do while condition    | `DO WHILE x < 10`                    |
| `DO UNTIL`         | Do until condition    | `DO UNTIL x >= 10`                   |
| `LOOP WHILE`       | Loop while condition  | `LOOP WHILE x < 10`                  |
| `LOOP UNTIL`       | Loop until condition  | `LOOP UNTIL x >= 10`                 |
| `EXIT FOR`         | Exit for loop         | `EXIT FOR`                           |
| `EXIT DO`          | Exit do loop          | `EXIT DO`                            |
| `GOTO`             | Jump to label         | `GOTO 100`                           |
| `GOSUB`            | Call subroutine       | `GOSUB 1000`                         |
| `RETURN`           | Return from gosub     | `RETURN`                             |
| `ON...GOTO`        | Computed goto         | `ON x GOTO 100, 200, 300`            |
| `ON...GOSUB`       | Computed gosub        | `ON x GOSUB 1000, 2000`              |
| `END`              | End program           | `END`                                |
| `STOP`             | Stop execution        | `STOP`                               |
| `SYSTEM`           | Exit to OS            | `SYSTEM`                             |

### Variables & Data Types

| Command           | Description              | Example                      |
| ----------------- | ------------------------ | ---------------------------- |
| `DIM`             | Declare variable/array   | `DIM x AS INTEGER`           |
| `REDIM`           | Redimension array        | `REDIM arr(20)`              |
| `REDIM PRESERVE`  | Redimension keeping data | `REDIM PRESERVE arr(30)`     |
| `CONST`           | Declare constant         | `CONST MAX = 100`            |
| `LET`             | Assign value (optional)  | `LET x = 10`                 |
| `COMMON SHARED`   | Share across modules     | `COMMON SHARED x AS INTEGER` |
| `SHARED`          | Share in sub/function    | `SHARED x`                   |
| `STATIC`          | Static variable          | `STATIC count AS INTEGER`    |
| `TYPE...END TYPE` | User-defined type        | `TYPE Player`                |
| `DEFINT`          | Default integer          | `DEFINT A-Z`                 |
| `DEFSTR`          | Default string           | `DEFSTR S`                   |
| `DEFSNG`          | Default single           | `DEFSNG A`                   |
| `DEFDBL`          | Default double           | `DEFDBL D`                   |
| `DEFLNG`          | Default long             | `DEFLNG L`                   |
| `OPTION BASE`     | Array base index         | `OPTION BASE 1`              |
| `ERASE`           | Erase array              | `ERASE arr`                  |
| `SWAP`            | Swap two variables       | `SWAP a, b`                  |
| `CLEAR`           | Clear variables          | `CLEAR`                      |

### Input/Output

| Command       | Description           | Example                     |
| ------------- | --------------------- | --------------------------- |
| `PRINT`       | Print to screen       | `PRINT "Hello"`             |
| `PRINT USING` | Formatted print       | `PRINT USING "##.##"; x`    |
| `INPUT`       | Get user input        | `INPUT "Name ", name$`      |
| `LINE INPUT`  | Get line of input     | `LINE INPUT "Text ", text$` |
| `WRITE`       | Write comma-separated | `WRITE #1, a, b, c`         |
| `CLS`         | Clear screen          | `CLS`                       |
| `LOCATE`      | Position cursor       | `LOCATE 10, 20`             |
| `TAB`         | Tab to column         | `PRINT TAB(10); "Text"`     |
| `SPC`         | Print spaces          | `PRINT SPC(5); "Text"`      |
| `BEEP`        | Sound beep            | `BEEP`                      |
| `SLEEP`       | Pause execution       | `SLEEP 2`                   |
| `INKEY$`      | Get key press         | `k$ = INKEY$`               |

### String Functions

| Function  | Description       | Example                       |
| --------- | ----------------- | ----------------------------- |
| `LEFT$`   | Left substring    | `LEFT$("Hello", 2)` → "He"    |
| `RIGHT$`  | Right substring   | `RIGHT$("Hello", 2)` → "lo"   |
| `MID$`    | Middle substring  | `MID$("Hello", 2, 3)` → "ell" |
| `LEN`     | String length     | `LEN("Hello")` → 5            |
| `INSTR`   | Find substring    | `INSTR("Hello", "ll")` → 3    |
| `LCASE$`  | Lowercase         | `LCASE$("HELLO")` → "hello"   |
| `UCASE$`  | Uppercase         | `UCASE$("hello")` → "HELLO"   |
| `LTRIM$`  | Trim left spaces  | `LTRIM$("  Hi")` → "Hi"       |
| `RTRIM$`  | Trim right spaces | `RTRIM$("Hi  ")` → "Hi"       |
| `TRIM$`   | Trim both sides   | `TRIM$("  Hi  ")` → "Hi"      |
| `STR$`    | Number to string  | `STR$(123)` → " 123"          |
| `VAL`     | String to number  | `VAL("123")` → 123            |
| `CHR$`    | ASCII to char     | `CHR$(65)` → "A"              |
| `ASC`     | Char to ASCII     | `ASC("A")` → 65               |
| `SPACE$`  | Create spaces     | `SPACE$(5)` → " "             |
| `STRING$` | Repeat character  | `STRING$(3, "*")` → "\*\*\*"  |
| `HEX$`    | Number to hex     | `HEX$(255)` → "FF"            |
| `OCT$`    | Number to octal   | `OCT$(8)` → "10"              |

### Math Functions

| Function    | Description          | Example            |
| ----------- | -------------------- | ------------------ |
| `ABS`       | Absolute value       | `ABS(-5)` → 5      |
| `SGN`       | Sign (-1, 0, 1)      | `SGN(-5)` → -1     |
| `SIN`       | Sine                 | `SIN(1.57)` → 1.0  |
| `COS`       | Cosine               | `COS(0)` → 1.0     |
| `TAN`       | Tangent              | `TAN(0.785)` → 1.0 |
| `ATN`       | Arctangent           | `ATN(1)` → 0.785   |
| `EXP`       | Exponential          | `EXP(1)` → 2.718   |
| `LOG`       | Natural logarithm    | `LOG(2.718)` → 1.0 |
| `SQR`       | Square root          | `SQR(16)` → 4      |
| `INT`       | Integer part (floor) | `INT(3.7)` → 3     |
| `FIX`       | Truncate decimal     | `FIX(-3.7)` → -3   |
| `RND`       | Random number        | `RND` → 0.0-1.0    |
| `RANDOMIZE` | Seed random          | `RANDOMIZE TIMER`  |
| `MOD`       | Modulo               | `10 MOD 3` → 1     |
| `^`         | Power                | `2 ^ 3` → 8        |
| `\`         | Integer division     | `10 \ 3` → 3       |

### Type Conversion

| Function | Description        | Example             |
| -------- | ------------------ | ------------------- |
| `CINT`   | Convert to integer | `CINT(3.7)` → 4     |
| `CLNG`   | Convert to long    | `CLNG(3.7)` → 4     |
| `CSNG`   | Convert to single  | `CSNG(3)` → 3.0     |
| `CDBL`   | Convert to double  | `CDBL(3)` → 3.0     |
| `CSTR`   | Convert to string  | `CSTR(123)` → "123" |
| `MKI$`   | Integer to string  | `MKI$(100)`         |
| `MKL$`   | Long to string     | `MKL$(100000)`      |
| `MKS$`   | Single to string   | `MKS$(value)`       |
| `MKD$`   | Double to string   | `MKD$(value)`       |
| `CVI`    | String to integer  | `CVI(s$)`           |
| `CVL`    | String to long     | `CVL(s$)`           |
| `CVS`    | String to single   | `CVS(s$)`           |
| `CVD`    | String to double   | `CVD(s$)`           |

### Array Operations

| Command          | Description         | Example                  |
| ---------------- | ------------------- | ------------------------ |
| `DIM arr(n)`     | Declare array       | `DIM arr(10) AS INTEGER` |
| `REDIM`          | Resize array        | `REDIM arr(20)`          |
| `REDIM PRESERVE` | Resize keeping data | `REDIM PRESERVE arr(30)` |
| `LBOUND`         | Lower bound         | `LBOUND(arr)` → 0 or 1   |
| `UBOUND`         | Upper bound         | `UBOUND(arr)` → 10       |
| `ERASE`          | Erase array         | `ERASE arr`              |
| `OPTION BASE`    | Set array base      | `OPTION BASE 1`          |

### File I/O

| Command        | Description            | Example                            |
| -------------- | ---------------------- | ---------------------------------- |
| `OPEN`         | Open file              | `OPEN "file.txt" FOR OUTPUT AS #1` |
| `CLOSE`        | Close file             | `CLOSE #1`                         |
| `PRINT #`      | Write to file          | `PRINT #1, "Data"`                 |
| `WRITE #`      | Write formatted        | `WRITE #1, a, b, c`                |
| `INPUT #`      | Read from file         | `INPUT #1, x`                      |
| `LINE INPUT #` | Read line              | `LINE INPUT #1, line$`             |
| `INPUT$`       | Read n characters      | `INPUT$(10, #1)`                   |
| `EOF`          | End of file            | `EOF(1)`                           |
| `LOF`          | Length of file         | `LOF(1)`                           |
| `LOC`          | Current position       | `LOC(1)`                           |
| `SEEK`         | Set file position      | `SEEK #1, 100`                     |
| `FREEFILE`     | Get free file number   | `f = FREEFILE`                     |
| `GET`          | Read record            | `GET #1, , record`                 |
| `PUT`          | Write record           | `PUT #1, , record`                 |
| `FIELD`        | Define record fields   | `FIELD #1, 10 AS name$`            |
| `LSET`         | Left-justify in field  | `LSET name$ = "John"`              |
| `RSET`         | Right-justify in field | `RSET name$ = "John"`              |
| `KILL`         | Delete file            | `KILL "file.txt"`                  |
| `NAME...AS`    | Rename file            | `NAME "old.txt" AS "new.txt"`      |
| `FILES`        | List files             | `FILES "*.bas"`                    |
| `CHDIR`        | Change directory       | `CHDIR "C\DATA"`                   |
| `MKDIR`        | Make directory         | `MKDIR "NEWDIR"`                   |
| `RMDIR`        | Remove directory       | `RMDIR "OLDDIR"`                   |

### Graphics & Sound

| Command          | Description      | Example                      |
| ---------------- | ---------------- | ---------------------------- |
| `SCREEN`         | Set screen mode  | `SCREEN 12`                  |
| `COLOR`          | Set colors       | `COLOR 15, 1`                |
| `CLS`            | Clear screen     | `CLS`                        |
| `LOCATE`         | Position cursor  | `LOCATE 10, 20`              |
| `PSET`           | Set pixel        | `PSET (100, 100), 15`        |
| `PRESET`         | Reset pixel      | `PRESET (100, 100)`          |
| `LINE`           | Draw line        | `LINE (0, 0)-(100, 100), 15` |
| `CIRCLE`         | Draw circle      | `CIRCLE (160, 100), 50, 14`  |
| `PAINT`          | Fill area        | `PAINT (160, 100), 9, 14`    |
| `DRAW`           | Draw with macro  | `DRAW "U50 R50 D50 L50"`     |
| `VIEW`           | Set viewport     | `VIEW (0, 0)-(320, 200)`     |
| `WINDOW`         | Set coordinates  | `WINDOW (-10, -10)-(10, 10)` |
| `PMAP`           | Map coordinates  | `PMAP(x, 0)`                 |
| `POINT`          | Get pixel color  | `POINT(100, 100)`            |
| `PALETTE`        | Set palette      | `PALETTE 1, 63`              |
| `WIDTH`          | Set screen width | `WIDTH 80, 25`               |
| `GET (graphics)` | Capture image    | `GET (0, 0)-(10, 10), arr`   |
| `PUT (graphics)` | Display image    | `PUT (50, 50), arr, PSET`    |
| `SOUND`          | Play sound       | `SOUND 440, 18`              |
| `PLAY`           | Play music       | `PLAY "MFT180 O3 C E G"`     |
| `BEEP`           | System beep      | `BEEP`                       |

### System & Memory

| Command/Function | Description            | Example                     |
| ---------------- | ---------------------- | --------------------------- |
| `TIMER`          | Seconds since midnight | `t = TIMER`                 |
| `DATE$`          | Current date           | `d$ = DATE$`                |
| `TIME$`          | Current time           | `t$ = TIME$`                |
| `COMMAND$`       | Command-line args      | `cmd$ = COMMAND$`           |
| `ENVIRON$`       | Environment variable   | `ENVIRON$("PATH")`          |
| `FRE`            | Free memory            | `FRE("")`                   |
| `CSRLIN`         | Current row            | `r = CSRLIN`                |
| `POS`            | Current column         | `c = POS(0)`                |
| `PEEK`           | Read memory byte       | `PEEK(&H417)`               |
| `POKE`           | Write memory byte      | `POKE &H417, 0`             |
| `DEF SEG`        | Set memory segment     | `DEF SEG = &HA000`          |
| `VARPTR`         | Variable pointer       | `VARPTR(x)`                 |
| `VARSEG`         | Variable segment       | `VARSEG(x)`                 |
| `SADD`           | String address         | `SADD(s$)`                  |
| `VARPTR$`        | Pointer as string      | `VARPTR$(x)`                |
| `BLOAD`          | Load binary file       | `BLOAD "file.bin", 0`       |
| `BSAVE`          | Save binary file       | `BSAVE "file.bin", 0, 1000` |
| `SHELL`          | Execute command        | `SHELL "DIR"`               |
| `CHAIN`          | Chain to program       | `CHAIN "prog2.bas"`         |
| `CALL`           | Call subroutine        | `CALL MySub(x, y)`          |
| `CALL ABSOLUTE`  | Call machine code      | `CALL ABSOLUTE(addr)`       |

### Error Handling

| Command                | Description         | Example                      |
| ---------------------- | ------------------- | ---------------------------- |
| `ON ERROR GOTO`        | Set error handler   | `ON ERROR GOTO ErrorHandler` |
| `ON ERROR RESUME NEXT` | Ignore errors       | `ON ERROR RESUME NEXT`       |
| `RESUME`               | Resume after error  | `RESUME`                     |
| `RESUME NEXT`          | Resume next line    | `RESUME NEXT`                |
| `RESUME <label>`       | Resume at label     | `RESUME Continue`            |
| `ERR`                  | Error number        | `IF ERR = 53 THEN...`        |
| `ERL`                  | Error line number   | `PRINT "Error at line"; ERL` |
| `ERDEV`                | Device error code   | `ERDEV`                      |
| `ERDEV$`               | Device error string | `ERDEV$`                     |

### Advanced Features

| Command                   | Description            | Example                           |
| ------------------------- | ---------------------- | --------------------------------- |
| `SUB...END SUB`           | Define subroutine      | `SUB MySub(x)`                    |
| `FUNCTION...END FUNCTION` | Define function        | `FUNCTION Add(a, b)`              |
| `DECLARE SUB`             | Declare subroutine     | `DECLARE SUB MySub(x AS INTEGER)` |
| `DECLARE FUNCTION`        | Declare function       | `DECLARE FUNCTION Add#(a, b)`     |
| `DEF FN`                  | Define inline function | `DEF FNSquare(x) = x * x`         |
| `STATIC`                  | Static sub/function    | `SUB MySub STATIC`                |
| `SHARED`                  | Share variables        | `SHARED x, y`                     |
| `COMMON SHARED`           | Share across modules   | `COMMON SHARED x AS INTEGER`      |
| `DATA`                    | Define data            | `DATA 10, 20, 30`                 |
| `READ`                    | Read data              | `READ x, y, z`                    |
| `RESTORE`                 | Reset data pointer     | `RESTORE MyData`                  |
| `KEY`                     | Define function key    | `KEY 1, "LIST" + CHR$(13)`        |
| `KEY ON/OFF`              | Enable/disable keys    | `KEY ON`                          |
| `KEY LIST`                | List key definitions   | `KEY LIST`                        |
| `ON TIMER`                | Timer event            | `ON TIMER(1) GOSUB TimerEvent`    |
| `TIMER ON/OFF`            | Enable/disable timer   | `TIMER ON`                        |
| `ON COM`                  | Serial port event      | `ON COM(1) GOSUB ComEvent`        |
| `ON PEN`                  | Light pen event        | `ON PEN GOSUB PenEvent`           |
| `ON STRIG`                | Joystick event         | `ON STRIG(1) GOSUB JoyEvent`      |
| `ON PLAY`                 | Music event            | `ON PLAY(1) GOSUB MusicEvent`     |
| `TRON`                    | Trace on               | `TRON`                            |
| `TROFF`                   | Trace off              | `TROFF`                           |
| `LPRINT`                  | Print to printer       | `LPRINT "Text"`                   |
| `LPOS`                    | Printer position       | `LPOS(1)`                         |
| `OUT`                     | Output to port         | `OUT &H3F8, 65`                   |
| `INP`                     | Input from port        | `INP(&H3F8)`                      |
| `WAIT`                    | Wait for port          | `WAIT &H3DA, 8`                   |
| `STICK`                   | Joystick position      | `STICK(0)`                        |
| `VIEW PRINT`              | Set text viewport      | `VIEW PRINT 1 TO 20`              |

### Logical Operators

| Operator | Description  | Example                   |
| -------- | ------------ | ------------------------- |
| `AND`    | Logical AND  | `IF x > 0 AND y > 0 THEN` |
| `OR`     | Logical OR   | `IF x = 1 OR y = 1 THEN`  |
| `NOT`    | Logical NOT  | `IF NOT flag THEN`        |
| `XOR`    | Exclusive OR | `result = a XOR b`        |
| `EQV`    | Equivalence  | `result = a EQV b`        |
| `IMP`    | Implication  | `result = a IMP b`        |

### Comparison Operators

| Operator | Description      | Example           |
| -------- | ---------------- | ----------------- |
| `=`      | Equal            | `IF x = 10 THEN`  |
| `<>`     | Not equal        | `IF x <> 10 THEN` |
| `<`      | Less than        | `IF x < 10 THEN`  |
| `>`      | Greater than     | `IF x > 10 THEN`  |
| `<=`     | Less or equal    | `IF x <= 10 THEN` |
| `>=`     | Greater or equal | `IF x >= 10 THEN` |

---

## Development

### Project Structure

The project uses Cargo workspaces for modular development. In-source unit tests stay next to implementation files, and file-based integration tests plus BASIC fixtures are centralized under `test/`.

### Test Layout

- `test/integration/cli_tool/` contains CLI and end-to-end integration tests
- `test/integration/vm_engine/` contains VM integration and semantic regression tests
- `test/integration/tokenizer/` contains tokenizer integration coverage
- `test/fixtures/basic/` contains shared BASIC smoke, graphics, and compatibility fixtures

### Run Test Suite

**Run all tests**

```bash
cargo test
```

**Run tests for specific crate**

```bash
cargo test -p tokenizer
cargo test -p syntax_tree
cargo test -p vm_engine
cargo test -p cli_tool --test shell_cli
cargo test -p cli_tool --test compile_smoke_test
```

**Run with output**

```bash
cargo test -- --nocapture
```

### Build Variants

**Debug build (fast compilation, slower execution)**

```bash
cargo build
./target/debug/qb test/fixtures/basic/test_all.bas
```

**Release build (optimized)**

```bash
cargo build --release
./target/release/qb test/fixtures/basic/test_all.bas
```

**Benchmark build**

```bash
cargo build --profile bench
```

### Run Shared Fixtures

**Comprehensive BASIC fixture**

```bash
cargo run --release -- test/fixtures/basic/test_all.bas
```

**Targeted fixture or sample program**

```bash
cargo run --release -- -x test/fixtures/basic/test_graphics_getput.bas
cargo run --release -- -c path/to/program.bas
```

### Development Workflow

1. **Make changes** to source code
2. **Run tests** to verify functionality
   ```bash
   cargo test
   ```
3. **Test with shared fixtures**
   ```bash
   cargo run -- -x test/fixtures/basic/test_all.bas
   ```
4. **Check for warnings**
   ```bash
   cargo clippy
   ```
5. **Format code**
   ```bash
   cargo fmt
   ```

### Performance Profiling

**Build with profiling**

```bash
cargo build --release
```

**Profile execution**

```bash
# Windows
cargo run --release -- test/fixtures/basic/test_all.bas

# Linux (with perf)
perf record cargo run --release -- test/fixtures/basic/test_all.bas
perf report
```

### Debugging

**Enable debug output**

```bash
RUST_LOG=debug cargo run -- -x test/fixtures/basic/test_all.bas
```

**Run with debugger**

```bash
# GDB (Linux)
gdb --args target/debug/qb test/fixtures/basic/test_all.bas

# LLDB (macOS)
lldb target/debug/qb -- test/fixtures/basic/test_all.bas
```

### Adding New Features

1. **Add keyword** to `tokenizer/src/tokens.rs`
2. **Update parser** in `syntax_tree/src/parser.rs`
3. **Add AST node** in `syntax_tree/src/ast_nodes.rs`
4. **Implement in VM** in `vm_engine/src/compiler.rs` and `vm_engine/src/runtime.rs`
5. **Add codegen** in `native_codegen/src/codegen.rs`
6. **Write file-based integration tests** under `test/integration/<crate>/` and shared BASIC fixtures under `test/fixtures/basic/`

### Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Use meaningful variable names
- Add comments for complex logic
- Write unit tests for new features
- Update documentation

---

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

**Ways to contribute**

- Report bugs and issues
- Suggest new features
- Improve documentation
- Submit pull requests
- Write test cases
- Optimize performance

**Development setup**

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests (`cargo test`)
5. Commit your changes (`git commit -m 'Add amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

---

## License

This project is licensed under the **MIT License**. See [LICENSE](LICENSE) for details.

**MIT License Summary**

- ✅ Commercial use
- ✅ Modification
- ✅ Distribution
- ✅ Private use
- ❌ Liability
- ❌ Warranty

---

## Acknowledgments

- **QBasic/QuickBASIC** - The original BASIC implementation by Microsoft
- **QB64** - Modern QBasic compiler that inspired this project
- **Rust Community** - For excellent tools and libraries
- **Contributors** - Everyone who has contributed to this project

---

## Links

- **Repository** [https//github.com/thirawat27/QBNex](https//github.com/thirawat27/QBNex)
- **Issues** [https//github.com/thirawat27/QBNex/issues](https//github.com/thirawat27/QBNex/issues)
- **Changelog** [CHANGELOG.md](CHANGELOG.md)
- **Security** [SECURITY.md](SECURITY.md)

---

<div align="center">
  <strong>Built with ❤️ using Rust</strong>
  <br>
  <p>Created by <a href="https//github.com/thirawat27">thirawat27</a></p>
</div>
