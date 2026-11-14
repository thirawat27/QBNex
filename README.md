<div align="center">
  <img src="assets/QBNex.png" alt="QBNex Logo" width="256" height="256">
  
  # QBNex
  
  **Modern QBasic/QuickBASIC Compiler**
  
  > **QBNex is a modern QBasic compiler built to keep the QuickBASIC/QBasic spirit alive.** It stays true to classic BASIC compatibility, so existing ideas and code patterns still feel familiar, while adding a Rust runtime, a standard library, editor tools, and standalone executables for Windows, Linux, and macOS.
  
</div>

---

## Documentation / เอกสาร

The user manual is in [`docs/manual`](docs/manual/README.md), in English and Thai.
คู่มือผู้ใช้อยู่ที่ [`docs/manual`](docs/manual/README.md) มีทั้งภาษาอังกฤษและภาษาไทย

| English | ภาษาไทย |
| --- | --- |
| [Installation](docs/manual/en/installation.md) | [การติดตั้ง](docs/manual/th/installation.md) |
| [Commands: compile, fmt, lsp, dap](docs/manual/en/commands.md) | [คำสั่ง: คอมไพล์, fmt, lsp, dap](docs/manual/th/commands.md) |
| [Language extensions (available and planned)](docs/manual/en/language.md) | [ส่วนขยายของภาษา (ใช้ได้แล้วและวางแผนไว้)](docs/manual/th/language.md) |
| [Standard library: json, url, ext.*](docs/manual/en/stdlib.md) | [ไลบรารีมาตรฐาน: json, url, ext.*](docs/manual/th/stdlib.md) |
| [Graphics and 3D](docs/manual/en/graphics-3d.md) | [กราฟิกและ 3 มิติ](docs/manual/th/graphics-3d.md) |
| [Web (WebAssembly) build](docs/manual/en/web.md) | [รันบนเว็บ (WebAssembly)](docs/manual/th/web.md) |
| [VS Code, formatter, language server, debugger](docs/manual/en/vscode.md) | [VS Code, formatter, language server, debugger](docs/manual/th/vscode.md) |
| [Differences from QB64 and QBasic](docs/manual/en/differences.md) | [ความแตกต่างจาก QB64 และ QBasic](docs/manual/th/differences.md) |
| [Limitations and differences from QB64](docs/manual/en/limitations.md) | [ข้อจำกัดและความต่างจาก QB64](docs/manual/th/limitations.md) |

Where this README and the manual disagree, the manual is the current one.

Developer reference (QIR format, deviation IDs `L-xx`/`D-xx`, `DECLARE LIBRARY` and `_FLOAT`,
3D API design): [`docs/dev`](docs/dev/README.md). Building, testing and contributing:
[CONTRIBUTING.md](CONTRIBUTING.md). Release history: [CHANGELOG.md](CHANGELOG.md).
เอกสารอ้างอิงสำหรับนักพัฒนาอยู่ที่ [`docs/dev`](docs/dev/README.md) และวิธี build/ทดสอบ/ร่วมพัฒนาอยู่ใน
[CONTRIBUTING.md](CONTRIBUTING.md)

---

## Table of Contents

- [Documentation / เอกสาร](#documentation--เอกสาร)
- [About](#about)
- [Features](#features)
- [System Requirements](#system-requirements)
- [Installation](#installation)
  - [Release Packages and Installer](#release-packages-and-installer)
  - [Build from Source](#build-from-source)
  - [Docker](#docker)
- [Quick Start](#quick-start)
- [Usage Guide](#usage-guide)
  - [Basic Compilation](#basic-compilation)
  - [Compiler Flags Reference](#compiler-flags-reference)
  - [Compiler Settings](#compiler-settings)
  - [Environment Variables](#environment-variables)
- [Language Extensions](#language-extensions)
- [Standard Library](#standard-library)
  - [Importing Modules](#importing-modules)
  - [Working with Modules](#working-with-modules)
  - [Collection Libraries](#collection-libraries)
  - [String Libraries](#string-libraries)
  - [I/O Libraries](#io-libraries)
  - [System Libraries](#system-libraries)
  - [Math \& Error Handling](#math--error-handling)
  - [OOP Support](#oop-support)
- [Developer Tools](#developer-tools)
- [Docker Guide](#docker-guide)
- [Compilation Pipeline](#compilation-pipeline)
- [Diagnostics](#diagnostics)
- [Testing \& Verification](#testing--verification)
- [Troubleshooting](#troubleshooting)
- [Code Examples](#code-examples)
- [Supported QBasic Commands](#supported-qbasic-commands)
- [Development](#development)
  - [Project Structure](#project-structure)
  - [Adding New Features](#adding-new-features)
  - [Continuous Integration and Releases](#continuous-integration-and-releases)
- [Contributing](#contributing)
- [License](#license)
- [Acknowledgments](#acknowledgments)
- [Links](#links)

---

## About

**QBNex** is a modern QBasic/QuickBASIC compiler. It translates BASIC source code into QIR, a
text intermediate language, and packages it with a Rust runner into a standalone executable for
Windows, Linux, and macOS. It started as a refactor of QB64 into a CLI-driven compiler without the
IDE. Since 2.0 there is no C or C++ anywhere in the pipeline: the runner and runtime are Rust
crates (`crates/`), the QIR runs on a register VM, and hot loops are compiled to native code with
Cranelift.

**Version**: 2.0.0 (`qb --version`; see [CHANGELOG.md](CHANGELOG.md))

The compiler is self-hosting: it is written in QBNex BASIC (about 35,000 lines in `compiler/`) and
compiles itself. Graphics, windowing, sound, fonts and images come from the Rust runtime (winit,
softbuffer, cpal, skrifa, and wgpu for the optional GPU path); no OpenGL, FreeGLUT, FreeType or
miniaudio is linked.

Repository: https://github.com/thirawat27/QBNex

---

## Features

- **Language**
  - QBasic / QuickBASIC 4.5 programs and the QB64 dialect (`_` keywords, `$` metacommands, `_UNSIGNED`, `_INTEGER64`, `_FLOAT`, `_MEM`, `DECLARE LIBRARY`)
  - `DEF FN` (single-line and multi-line), `CHAIN` with `COMMON`, event traps (`ON TIMER`, `ON KEY`, `ON STRIG`, `ON PLAY`, `ON ERROR`), `LPRINT`
  - QBNex extensions: `_IIF`, `_ANDALSO`/`_ORELSE`, `$"...{expr}..."` interpolation, `FOR EACH`, `ENUM`, `TRY`/`CATCH`/`FINALLY`, `OPTIONAL` parameters, `_CONTINUE`, `$ASSERTS`/`_ASSERT`, CLASS (with `'$IMPORT:'qbnex'`)
  - Planned: `x += 1`, array literals, `_THROW`, string helpers such as `_SPLIT`; see [docs/manual/en/language.md](docs/manual/en/language.md#planned)
  - Source files in CP437 (as QBasic saved them) or UTF-8
  - Extended data types: `_BIT`, `_BYTE`, `_INTEGER64`, `_FLOAT` (exact software 80-bit), `_OFFSET`, and the `_UNSIGNED` forms

- **Execution**
  - Self-hosting compiler written in QBNex BASIC
  - BASIC is translated to QIR and run by a Rust VM with a Cranelift JIT for hot code
  - Standalone executables (runner + program); no C/C++ toolchain is needed to build programs
  - Native executables linked with `rust-lld` (Preview, needs the native kit)
  - Windows x64, Linux and macOS

- **Graphics, window and sound**
  - All QBasic `SCREEN` modes (0-13) and QB64 32-bit images; `PSET`, `LINE`, `CIRCLE`, `PAINT`, `DRAW`, `GET`/`PUT`, `_PUTIMAGE`, `_LOADIMAGE` (PNG, JPEG, BMP, PCX, ...), TrueType fonts
  - `_MAPTRIANGLE` 3D with a depth buffer (software renderer); optional wgpu GPU path
  - High-level 3D (Preview): `_LOADMODEL`, `_MESH*`, `_CAMERA`, `_LIGHT`, `_DRAW3D`, WGSL shaders with `_SHADER`
  - ALT+ENTER full screen (`_ALLOWFULLSCREEN`)
  - `SOUND`, `PLAY`, `BEEP` with QBasic's `MF`/`MB` music modes, `_SND*` sound files, 4-voice synthesis (`_VOICE`, `_ADSR`, `_WAVE`)
  - Audio through cpal: WASAPI (Windows), CoreAudio (macOS), ALSA (Linux); a silent device when there is no speaker

- **Files, network and system**
  - Sequential, random and binary files; `FIELD`, `LSET`/`RSET`; `KILL`, `NAME`, `FILES`, `MKDIR`/`CHDIR`/`RMDIR`; `BLOAD`/`BSAVE`
  - TCP/IP with `_OPENHOST`, `_OPENCLIENT`, `_OPENCONNECTION`
  - `PEEK`/`POKE`/`DEF SEG` over an emulated 1 MB memory, `CALL ABSOLUTE` and `CALL INTERRUPT` (16-bit x86 emulator, INT 33h mouse)
  - `SHELL`, `CHAIN`, `RUN`, `COMMAND$`, `ENVIRON$`

- **Standard library** (`'$IMPORT:'module'`): `json`, `url`, collections, strings, I/O, system, math, result, OOP, and the Rust-backed `ext.*` modules (regex, hash, zip, http, utf8, collections, datetime)

- **Tools**
  - Formatter (`qbnex fmt`), language server (`qbnex lsp`), debugger (`qbnex dap`), VS Code extension (`packaging/vscode`)
  - Browser build of the QIR runner (WebAssembly, Preview)
  - One-file installer `qbnex-setup` for Windows, Linux and macOS

---

## System Requirements

QBNex needs no C or C++ compiler. The compiler is written in BASIC, translated to
QIR (`bootstrap/qbnex.qir`) and run by the Rust runner `qb-exec`; every compiled program
is a copy of that runner with its own QIR appended.

- **Using a release:** nothing else. The release bundle and the installer contain the compiler and the prebuilt runner.
- **Building from source:** the Rust toolchain (`cargo`, from https://rustup.rs). `rust-toolchain.toml` pins Rust 1.100.0.
- **Windows:** Windows 10 or newer, 64-bit. Excluding the QBNex folder from antivirus scanning helps (every compiled program is a new executable).
- **macOS:** CoreAudio and Cocoa come with macOS. Tested in CI (arm64) only.
- **Linux:** ALSA, udev, xkbcommon and X11 development packages and `pkg-config` to build; `gcc` is only the system linker Rust uses. Tested in CI only.

Window, graphics, audio, fonts, images and compression are Rust crates of the runtime
(see `licenses/THIRD_PARTY.TXT`).

---

## Installation

### Release Packages and Installer

Tagged releases publish `qbnex_<tag>_win-x64.zip`, `qbnex_<tag>_lnx.tar.gz` and
`qbnex_<tag>_osx.tar.gz`. Extract to a folder you can write to.

Or download the one-file installer for your system (`qbnex_<tag>_win-x64-setup.exe`,
`qbnex_<tag>_lnx-setup`, `qbnex_<tag>_osx-setup`) and run it. It installs for the current
user without administrator rights: on Windows into `%LOCALAPPDATA%\Programs\QBNex`, added
to your PATH, the Start menu and "Apps & features"; on Linux/macOS into
`~/.local/share/qbnex` with `qb` and `qbnex` linked in `~/.local/bin`.

| Option | Meaning |
| --- | --- |
| `--yes` (`/S`, `/SILENT`, `/VERYSILENT`) | Install without asking |
| `--dir <folder>` (`/DIR=<folder>`) | Install folder |
| `--no-path`, `--no-menu` | Leave PATH alone, add no menu entry |
| `--desktop`, `--associate` | Desktop shortcut, "Compile with QBNex" for `.bas` files |
| `--console` | Ask in the terminal instead of a window |
| `--uninstall` | Run from the install folder: remove what was installed |

The installer and `qb` are not code-signed, so SmartScreen and Gatekeeper may warn on the
first run. Details: [installation manual](docs/manual/en/installation.md).

### Build from Source

```bash
git clone https://github.com/thirawat27/QBNex.git
cd QBNex
scripts\setup_win.cmd          # Windows
sh scripts/setup_lnx.sh        # Linux
sh scripts/setup_osx.command   # macOS
```

Linux packages:

```bash
sudo apt-get install gcc libasound2-dev libudev-dev libxkbcommon-dev libx11-dev pkg-config  # Debian/Ubuntu
sudo dnf install gcc alsa-lib-devel systemd-devel libxkbcommon-devel libX11-devel pkgconf-pkg-config  # Fedora/RHEL
sudo pacman -S gcc alsa-lib systemd-libs libxkbcommon libx11 pkgconf  # Arch Linux
```

The setup script:
1. Builds the runner `qb-exec` with cargo into `crates/engine/exec/bin/<os>/`
2. Packs `bootstrap/qbnex.qir` into it, giving a stage-0 compiler (`qb-stage0`)
3. Lets stage 0 compile `compiler/qbnex.bas` into `qb` / `qb.exe` in the project root
   (run it as `qb` or `qbnex`) and runs a smoke test

Setup variables: `QBNEX_QB_EXEC` (use a prebuilt `qb-exec`), `QBNEX_RUST_TARGET`,
`QBNEX_BOOTSTRAP`, `QBNEX_QIR_PACK=ir|native`, `QBNEX_STAGE1_ARGS`, `QBNEX_SKIP_SELFHOST=1`,
`QBNEX_KEEP_STAGE0=1`, `QBNEX_CI=1` (Windows: no pause at the end).

The developer tools are separate Rust programs:

```bash
cargo build --release -p qb-fmt -p qb-lsp -p qb-dap
```

`scripts/selfhost_fixpoint.cmd` (or `.sh`) checks that the compiler rebuilds itself to
the same QIR and that `bootstrap/qbnex.qir` is up to date. `scripts/reset.cmd` removes
the compiler's build folders, its settings and `qb.exe`.

### Docker

```bash
docker build -f docker/Dockerfile -t qbnex .
docker run --rm -v "$(pwd)":/project qbnex qb yourfile.bas
```

See the [Docker Guide](#docker-guide) below.

---

## Quick Start

**1. Create a BASIC program** (`hello.bas`):
```basic
PRINT "Hello, QBNex!"
PRINT "Welcome to modern BASIC programming!"

FOR i = 1 TO 5
    PRINT "Count: "; i
NEXT i

PRINT "Done!"
```

**2. Compile it:**
```bash
qb hello.bas
```

The compiler can be run as either `qb` or `qbnex`. Without `-o` the executable goes to the
QBNex folder (the one that holds `qb` and `crates/`), as in QB64; use `-o` or
`qb -s:exewithsource=true` to put it somewhere else.

**3. Run the executable:**
```bash
# Windows
hello.exe

# Linux/macOS
./hello
```

---

## Usage Guide

### Basic Compilation

```bash
# Compile to an executable (hello.exe or ./hello)
qb hello.bas

# Choose the output file
qb hello.bas -o myprogram.exe

# Show warnings too
qb hello.bas -w

# Show compiler version and help
qb --version
qb --help
```

**Command pattern:**
```bash
qb <source.bas> [switches]
```

Exit code 0 means the program compiled.

### Compiler Flags Reference

| Flag | Aliases | Description | Example |
|------|---------|-------------|---------|
| `-h` | `--help`, `/?` | Show help | `qb -h` |
| `-v` | `--version` | Show compiler version | `qb -v` |
| `-i` | `--info`, `--about` | Show project information | `qb -i` |
| `-g` | `--examples` | Show common CLI examples | `qb -g` |
| `-c` | - | Compile the source file (default) | `qb file.bas -c` |
| `-o <file>` | - | Output file; a name ending in `.qir` writes the QIR text (for qb-web) | `qb file.bas -o myapp.exe` |
| `--emit-ir` | - | Write the QIR text instead of an executable | `qb file.bas --emit-ir -o file.qir` |
| `-x` | - | Send the compiler's output to the console (it does not run the program) | `qb file.bas -x` |
| `-w` | - | Show warnings | `qb file.bas -w` |
| `-Werror` | `--warnings-as-errors` | Treat warnings as blocking diagnostics | `qb file.bas --warnings-as-errors` |
| `-q` | - | Quiet mode (warnings and errors are still shown) | `qb file.bas -q` |
| `-m` | - | No colours in the compiler output (also when `NO_COLOR` is set) | `qb file.bas -m` |
| `-k` | `--compact-errors` | Compact diagnostics (detailed ones are the default) | `qb file.bas -k` |
| `-d` | `--verbose-errors` | Old alias; detailed diagnostics are the default | `qb file.bas -d` |
| `-e` | - | `OPTION _EXPLICIT` for this compilation | `qb file.bas -e` |
| `-s[:setting[=true/false]]` | - | Show or change compiler settings | `qb -s:exewithsource=true` |
| `-p`, `-z` | `--ir` | Accepted for old scripts; they change nothing | `qb file.bas -z` |

Detailed diagnostics (`cause`, `example` and related notes) are shown by default:

```bash
qb myprogram.bas                    # detailed diagnostics
qb myprogram.bas -w                 # also warnings
qb myprogram.bas --compact-errors   # headline and source snippet only
qb myprogram.bas -w -o myapp.exe    # warnings and a custom output name
qb myprogram.bas -e -q              # OPTION _EXPLICIT, quiet
```

### Compiler Settings

`qb -s` reads and writes `target/qb-config.ini` in the QBNex folder:

| Setting | `-s` switch | Default | Description |
|---------|-------------|---------|-------------|
| `SaveExeWithSource` | `-s:exewithsource=true/false` | `false` | Write the executable next to the source file instead of the QBNex folder |
| `DebugInfo` | `-s:debuginfo=true/false` | `false` | Accepted for old scripts; no effect |
| `IgnoreWarnings` | (INI file only) | `false` | Hide warnings even with `-w` |

```bash
qb -s                          # show the settings
qb -s:exewithsource            # show one setting
qb -s:exewithsource=true       # change it
```

```ini
[COMPILER SETTINGS]
SaveExeWithSource=False
IgnoreWarnings=False
DebugInfo=False 'Managed by qbnex -s
```

### Environment Variables

| Variable | Effect |
| --- | --- |
| `QBNEX_NO_ERROR_DIALOG=1` | Runtime errors go to stderr with exit code 1 instead of the "Unhandled Error" dialog (set it in scripts and CI) |
| `QB_JIT=off` / `eager` / `<n>` | No JIT, compile at once, or after n loop iterations |
| `QB_JIT_THREADS=<n>`, `QB_JIT_STATS=1` | JIT compiler threads; JIT statistics on stderr |
| `QB_PROFILE=<file>` | Write the hottest functions and lines to a file at the end |
| `QB_GPU=off` / `verbose` | Runtimes with the GPU feature: never use the GPU / log the choice |
| `QB_GL=off` | No OpenGL context: `SUB _GL` is not run |
| `QB_AUDIO_DEVICE=null` / `none` | Silent device that keeps time / no audio |
| `QBNEX_LPRINT=<file>` | `LPRINT` appends to a text file |
| `QBNEX_HEADLESS=1` | Graphics programs run without a window (tests) |
| `NO_COLOR` | Compiler output without colours, like `-m` |

---

## Language Extensions

QBNex adds these to QBasic. Existing programs are unaffected: the new words start with `_` or are only recognised where QBasic would have reported a syntax error. Examples with output: [docs/manual/en/language.md](docs/manual/en/language.md).

| Feature | Example | Notes |
| --- | --- | --- |
| `_IIF` | `PRINT _IIF(n > 1, "items", "item")` | Only the chosen value is evaluated. Both values are strings or both are numbers. |
| `_ANDALSO`, `_ORELSE` | `IF i <= UBOUND(a) _ANDALSO a(i) > 0 THEN` | Short-circuit logic, result -1 or 0. |
| String interpolation | `PRINT $"Hello {name$ + "!"}, you are {age%}"` | Numbers are formatted like `LTRIM$(STR$(x))` (`_TOSTR$`). Expressions may contain string literals. `{{` and `}}` are literal braces. |
| `FOR EACH` | `FOR EACH v IN grid()` ... `NEXT` | Every element, first index outermost (any number of dimensions); `v` is a copy of the element. |
| `ENUM` | `ENUM Color` / `Red, Green = 5, Blue` / `END ENUM` | Members become integer `CONST`s, counting up from 0 or from the last `= value`. |
| `TRY` / `CATCH` / `FINALLY` | see below | Built on `ON ERROR`; `CATCH e` stores the error number in `e`. Without `CATCH`, the error is raised again after `FINALLY`. |
| `OPTIONAL` parameters | `SUB Greet (who$, OPTIONAL greeting$ = "Hello")` | Trailing arguments may be left out; the default is any expression valid at the call site. |
| `_CONTINUE` | `IF i MOD 2 = 0 THEN _CONTINUE` | Next iteration of the innermost `FOR`, `DO` or `WHILE`. |
| `$ASSERTS` + `_ASSERT` | `_ASSERT total >= 0, "negative total"` | Checked only when `$ASSERTS` is present. |
| CLASS | `CLASS Dog` ... `END CLASS` | With `'$IMPORT:'qbnex'`; rewritten into TYPEs and procedures (see [OOP Support](#oop-support)). |

```basic
TRY
    OPEN "data.txt" FOR INPUT AS #1
    LINE INPUT #1, first$
CATCH e
    PRINT "could not read data.txt, error"; e
FINALLY
    CLOSE
END TRY
```

`TRY`, `CATCH`, `FINALLY`, `END TRY`, `ENUM` and `END ENUM` may share a line with other statements (`TRY: x = 1 / d: CATCH e: PRINT e: END TRY`). TRY also works in `$INCLUDE` and `$IMPORT` files.

Planned (not in the compiler yet; the editor tools may already know the words): `x += 1` and the other
augmented assignments, `# comment` lines, `IMPORT module` without the `'$IMPORT:` form,
`FUNCTION name (...) AS type`, array literals, `_THROW`, `_SPLIT`/`_JOIN$`/`_REPLACE$` and the UTF-8
string helpers, built-in `_LIST`/`_DICT`, CLASS virtual dispatch. Earlier READMEs described some of
these as available; they are not.

---

## Standard Library

The modules are BASIC files in `compiler/stdlib/`; the `ext.*` modules call Rust code linked into
every runner (`crates/runtime/ext`). Reference for `json`, `url` and `ext.*`:
[docs/manual/en/stdlib.md](docs/manual/en/stdlib.md).

### Importing Modules

The only import form the compiler accepts is the `'$IMPORT:'module'` metacommand:

<!-- docs-test: define qbnex -->
```basic
' Traditional import syntax
'$IMPORT:'qbnex'
'$IMPORT:'json'
'$IMPORT:'url'
```

```basic
' Import individual modules
'$IMPORT:'sys.env'
'$IMPORT:'io.path'
'$IMPORT:'strings.text'
'$IMPORT:'collections.list'
'$IMPORT:'math.numeric'
```

The `IMPORT module` form below is **planned** and does not compile yet:

<!-- docs-test: compile-error -->
```basic
' Modern import syntax - cleaner and easier to read
IMPORT qbnex
IMPORT json
IMPORT url
```

An import line may appear anywhere in the main file: each module's declarations are placed
before the program and its procedures after it.

```basic
' Import the full qbnex stdlib core
'$IMPORT:'qbnex'

' Now you can use QBNex_ObjectHeader, QBNex_List, etc.
DIM myList AS QBNex_List
List_Init myList
List_Add myList, "Hello"
```

```basic
SUB Main ()
    PRINT Env_Platform$
    PRINT Path_Join$("root", "demo.txt")
END SUB

'$IMPORT:'sys.env'
'$IMPORT:'io.path'
```

```basic
'$IMPORT:'qbnex'

CLASS Dog
    Name AS STRING * 32
    
    CONSTRUCTOR (petName AS STRING)
        ME.Name = petName
    END CONSTRUCTOR
END CLASS
```

| Module | Import | Contents |
|--------|--------|----------|
| Core | `'$IMPORT:'qbnex'` | Every module below except `json`, `url` and `ext.*` in one import; those combine with it (`qbnex_stdlib.bas`), plus CLASS support |
| JSON | `'$IMPORT:'json'` | RFC 8259 JSON: `JSON_Parse%`, `JsonObjectGet%`, `JsonCreateObject%`, `JSON_Stringify$`, ... |
| URL | `'$IMPORT:'url'` | `UrlParse`, `UrlFormat$`, `UrlEncodeComponent$`, `UrlDecode$`, `UrlResolve`, ... |
| Collections | `collections.list`, `.stack`, `.queue`, `.set`, `.dictionary` | `QBNex_List` and friends; import `qbnex` for all but `collections.list` (on their own they do not compile yet) |
| Strings | `strings.strbuilder`, `strings.text` | String builder, padding and tests |
| I/O | `io.path`, `io.csv`, `io.json` | Paths, CSV rows, JSON builders |
| System | `sys.env`, `sys.args`, `sys.datetime` | Platform, arguments, dates |
| Math, errors | `math.numeric`, `error.result` | `Math_Clamp#`, `QBNex_Result` |
| OOP | `oop.class`, `oop.interface` | Class registry used by CLASS |
| Rust-backed | `ext.regex`, `ext.hash`, `ext.zip`, `ext.http`, `ext.utf8`, `ext.collections`, `ext.datetime` | See [stdlib manual](docs/manual/en/stdlib.md) |

### Working with Modules

**Example: Using Collections**
```basic
'$IMPORT:'qbnex'

SUB Demo_Collections ()
    DIM modules AS QBNex_List
    DIM loadOrder AS QBNex_Queue
    DIM features AS QBNex_HashSet
    DIM history AS QBNex_Stack
    DIM report AS QBNex_StringBuilder

    List_Init modules
    Queue_Init loadOrder
    HashSet_Init features
    Stack_Init history
    SB_Init report

    ' Add items
    List_Add modules, "collections.list"
    List_Add modules, "strings.strbuilder"
    List_Add modules, "sys.env"

    Queue_Enqueue loadOrder, "core"
    Queue_Enqueue loadOrder, "collections"

    HashSet_Add features, "OOP"
    HashSet_Add features, "Collections"

    Stack_Push history, "init"
    Stack_Push history, "ready"

    ' Build report
    SB_AppendLine report, "Loaded modules:"
    SB_AppendLine report, "  " + List_Join$(modules, ", ")
    SB_AppendLine report, "Queue head: " + Queue_Peek$(loadOrder)
    SB_AppendLine report, "Set members: " + HashSet_ToString$(features, " | ")
    SB_AppendLine report, "Latest stack item: " + Stack_Peek$(history)

    PRINT SB_ToString$(report)

    ' Clean up
    SB_Free report
    Stack_Free history
    HashSet_Free features
    Queue_Free loadOrder
    List_Free modules
END SUB
```

**Example: Using System Modules**
```basic
SUB Demo_System ()
    DIM nowValue AS QBNex_Date

    Date_SetNow nowValue
    PRINT "Platform: "; Env_Platform$
    PRINT "64-bit: "; Env_Is64Bit&
    PRINT "Home: "; Env_GetHome$
    PRINT "Joined path: "; Path_Join$(Env_GetHome$, "qbnex/demo/output.txt")
    PRINT "File name: "; Path_FileName$("src/stdlib/demo.bas")
    PRINT "Arg count: "; Args_Count&
    PRINT "Date ISO: "; Date_ToISOString$(nowValue)
END SUB

'$IMPORT:'strings.text'
'$IMPORT:'sys.env'
'$IMPORT:'sys.args'
'$IMPORT:'sys.datetime'
'$IMPORT:'io.path'
```

**Example: Using I/O Modules**
```basic
SUB Demo_Data ()
    DIM metadata AS QBNex_Dictionary
    DIM outcome AS QBNex_Result

    Dict_Init metadata
    Dict_Set metadata, "name", "QBNex"
    Dict_Set metadata, "layer", "stdlib"

    PRINT "Dict count: "; Dict_Count&(metadata)
    PRINT "Dict name: "; Dict_Get$(metadata, "name", "")
    PRINT "JSON sample: "; Json_Object3$("name", Json_String$(Dict_Get$(metadata, "name", "")), "layer", Json_String$(Dict_Get$(metadata, "layer", "")), "status", Json_String$("ok"))

    Result_Ok outcome, "stable"
    PRINT "Result ok: "; Result_IsOk&(outcome)
    PRINT "Result value: "; Result_Value$(outcome, "")

    Dict_Free metadata
END SUB

'$IMPORT:'qbnex'
```


### Collection Libraries

The examples below start with `'$IMPORT:'qbnex'`. All collections hold strings.

#### List (`collections.list`)
Dynamic array with automatic resizing.

- `List_Init list` · `List_Free list` · `List_Clear list`
- `List_Add list, item$` · `List_Set list, index&, item$` · `List_RemoveAt list, index&`
- `List_Get$(list, index&)` (0-based) · `List_Count&(list)`
- `List_IndexOf&(list, item$)` · `List_Contains&(list, item$)`
- `List_Join$(list, separator$)`

**Example:**
<!-- docs-test: prepend qbnex -->
```basic
DIM myList AS QBNex_List
List_Init myList
List_Add myList, "Apple"
List_Add myList, "Banana"
List_Add myList, "Cherry"

PRINT "Items: "; List_Count&(myList)
PRINT "All: "; List_Join$(myList, ", ")
' Output: Items: 3
' Output: All: Apple, Banana, Cherry

List_Free myList
```

#### Stack (`collections.stack`)
LIFO (last in, first out).

- `Stack_Init stack` · `Stack_Free stack` · `Stack_Clear stack`
- `Stack_Push stack, item$` · `Stack_Pop$(stack)` · `Stack_Peek$(stack)` · `Stack_Count&(stack)`

**Example:**
<!-- docs-test: prepend qbnex -->
```basic
DIM history AS QBNex_Stack
Stack_Init history

Stack_Push history, "init"
Stack_Push history, "registry"
Stack_Push history, "ready"

PRINT "Latest: "; Stack_Peek$(history)
PRINT "Count: "; Stack_Count&(history)
' Output: Latest: ready
' Output: Count: 3

Stack_Free history
```

#### Queue (`collections.queue`)
FIFO (first in, first out).

- `Queue_Init queue` · `Queue_Free queue` · `Queue_Clear queue`
- `Queue_Enqueue queue, item$` · `Queue_Dequeue$(queue)` · `Queue_Peek$(queue)` · `Queue_Count&(queue)`

**Example:**
<!-- docs-test: prepend qbnex -->
```basic
DIM loadOrder AS QBNex_Queue
Queue_Init loadOrder

Queue_Enqueue loadOrder, "core"
Queue_Enqueue loadOrder, "collections"
Queue_Enqueue loadOrder, "text"

PRINT "Next: "; Queue_Peek$(loadOrder)
' Output: Next: core

Queue_Free loadOrder
```

#### Set (`collections.set`)
Hash-based set of unique strings.

- `HashSet_Init set` · `HashSet_Free set` · `HashSet_Clear set`
- `HashSet_Add set, item$` (a duplicate is ignored) · `HashSet_Remove set, item$`
- `HashSet_Contains&(set, item$)` · `HashSet_Count&(set)` · `HashSet_ToString$(set, separator$)`

**Example:**
<!-- docs-test: prepend qbnex -->
```basic
DIM features AS QBNex_HashSet
HashSet_Init features

HashSet_Add features, "OOP"
HashSet_Add features, "Collections"
HashSet_Add features, "OOP"  ' Duplicate, ignored

PRINT "Members: "; HashSet_ToString$(features, " | ")
PRINT "Count: "; HashSet_Count&(features)
' Output: Members: OOP | Collections
' Output: Count: 2

HashSet_Free features
```

#### Dictionary (`collections.dictionary`)
Key-value store with string keys.

- `Dict_Init dict` · `Dict_Free dict` · `Dict_Clear dict`
- `Dict_Set dict, key$, value$` · `Dict_Remove dict, key$`
- `Dict_Get$(dict, key$, default$)` · `Dict_Has&(dict, key$)` · `Dict_Count&(dict)`
- `Dict_KeyAt$(dict, index&)` · `Dict_ValueAt$(dict, index&)`

**Example:**
<!-- docs-test: prepend qbnex -->
```basic
DIM metadata AS QBNex_Dictionary
Dict_Init metadata

Dict_Set metadata, "name", "QBNex"
Dict_Set metadata, "version", "current"
Dict_Set metadata, "kind", "compiler"

PRINT "Name: "; Dict_Get$(metadata, "name", "")
PRINT "Count: "; Dict_Count&(metadata)
' Output: Name: QBNex
' Output: Count: 3

Dict_Free metadata
```

### String Libraries

#### StringBuilder (`strings.strbuilder`)

- `SB_Init sb` · `SB_Free sb` · `SB_Clear sb`
- `SB_Append sb, text$` · `SB_AppendLine sb, text$`
- `SB_ToString$(sb)` · `SB_Length&(sb)`

**Example:**
<!-- docs-test: prepend qbnex -->
```basic
DIM report AS QBNex_StringBuilder
SB_Init report

SB_AppendLine report, "=== Report ==="
SB_AppendLine report, "Total items: 10"
SB_AppendLine report, "Status: OK"
SB_Append report, "End of report."

PRINT SB_ToString$(report)
' Output:
' === Report ===
' Total items: 10
' Status: OK
' End of report.

SB_Free report
```

#### Text Utilities (`strings.text`)

- `Text_PadLeft$(text$, width&, pad$)` · `Text_PadRight$(text$, width&, pad$)`
- `Text_Repeat$(text$, count&)`
- `Text_StartsWith&(text$, prefix$)` · `Text_EndsWith&(text$, suffix$)` · `Text_Contains&(text$, search$)`

**Example:**
<!-- docs-test: prepend qbnex -->
```basic
PRINT Text_PadRight$("QBNex", 10, ".")
' Output: QBNex.....

PRINT Text_PadLeft$("123", 8, "0")
' Output: 00000123
```

### I/O Libraries

#### Path Utilities (`io.path`)

- `Path_Join$(base$, leaf$)` · `Path_Normalize$(path$)` · `Path_Separator$`
- `Path_FileName$(path$)` · `Path_DirName$(path$)` · `Path_Extension$(path$)`

**Example:**
<!-- docs-test: prepend qbnex -->
```basic
PRINT Path_Join$("src", "stdlib/demo.bas")
' Output: src/stdlib/demo.bas

PRINT Path_FileName$("src/stdlib/demo.bas")
' Output: demo.bas

PRINT Path_Extension$("src/stdlib/demo.bas")
' Output: .bas
```

#### CSV Generation (`io.csv`)

- `CSV_Row2$(a$, b$)` · `CSV_Row3$(a$, b$, c$)` · `CSV_Row4$(a$, b$, c$, d$)`
- `CSV_Escape$(value$)`: quotes a value that holds a comma, quote or line break

**Example:**
<!-- docs-test: prepend qbnex -->
```basic
PRINT CSV_Row3$("name", "score", "status")
' Output: name,score,status

PRINT CSV_Row3$("Alice", "100", "pass")
' Output: Alice,100,pass
```

#### JSON Builders (`io.json`)

Small builders that return JSON text. For parsing, use the `json` module
([stdlib manual](docs/manual/en/stdlib.md#json)).

- `Json_String$(text$)` · `Json_Escape$(text$)` · `Json_Pair$(key$, valueJson$)`
- `Json_Object2$(...)` · `Json_Object3$(k1$, v1Json$, k2$, v2Json$, k3$, v3Json$)`
- `Json_Array2$(a$, b$)` · `Json_Array3$(a$, b$, c$)`

**Example:**
<!-- docs-test: prepend qbnex -->
```basic
PRINT Json_Object3$("name", Json_String$("QBNex"), "version", Json_String$("current"), "status", Json_String$("ok"))
' Output: {"name":"QBNex","version":"current","status":"ok"}
```

### System Libraries

#### Environment (`sys.env`)

- `Env_Platform$`: `WINDOWS`, `LINUX`, `MACOS` or `UNKNOWN`
- `Env_Is64Bit&` (-1 or 0) · `Env_GetHome$`
- `Env_Get$(name$, default$)` · `Env_Has&(name$)`

**Example:**
<!-- docs-test: prepend qbnex -->
```basic
PRINT "Platform: "; Env_Platform$
PRINT "64-bit: "; Env_Is64Bit&
PRINT "Home: "; Env_GetHome$
' Output (Linux):
' Platform: LINUX
' 64-bit: -1
' Home: /home/user
```

#### Arguments (`sys.args`)

- `Args_Count&` · `Args_Get$(index&, default$)` · `Args_Program$` · `Args_All$`

**Example:**
<!-- docs-test: prepend qbnex -->
```basic
PRINT "Argument count: "; Args_Count&
FOR i = 0 TO Args_Count& - 1
    PRINT "Arg "; i; ": "; Args_Get$(i, "")
NEXT i
```

#### DateTime (`sys.datetime`)

Months count from 0 and `Date_GetDay&` is the weekday (0 = Sunday), as in JavaScript.

- `Date_SetNow date` · `Date_FromParts date, year&, monthIndex&, day&, hour&, minute&, second&, ms&` · `Date_FromUnixMs date, ms#`
- `Date_ToISOString$(date)` · `Date_ToJSON$(date)` · `Date_ValueOf#(date)` · `Date_NowMs#`
- `Date_GetFullYear&`, `Date_GetMonth&`, `Date_GetDate&`, `Date_GetDay&`, `Date_GetHours&`, `Date_GetMinutes&`, `Date_GetSeconds&`, `Date_GetMilliseconds&`
- `Date_IsLeapYear&(year&)` · `Date_DaysInMonth&(year&, month&)`

**Example:**
<!-- docs-test: prepend qbnex -->
```basic
DIM now AS QBNex_Date
Date_SetNow now

PRINT "ISO: "; Date_ToISOString$(now)
PRINT "Year: "; Date_GetFullYear&(now)
PRINT "Timestamp: "; Date_NowMs#
' Output:
' ISO: 2026-04-13T19:30:00.000
' Year:  2026
' Timestamp:  1776108600000
```

For time zones, parsing and formatting use `ext.datetime` ([stdlib manual](docs/manual/en/stdlib.md#extdatetime)).

### Math & Error Handling

#### Numeric Utilities (`math.numeric`)

- `Math_Min#(a#, b#)` · `Math_Max#(a#, b#)` · `Math_Clamp#(x#, min#, max#)` · `Math_Lerp#(a#, b#, t#)`
- `Math_Deg2Rad#(degrees#)` · `Math_Rad2Deg#(radians#)`

**Example:**
<!-- docs-test: prepend qbnex -->
```basic
PRINT Math_Clamp#(15#, 0#, 10#)
' Output: 10

PRINT Math_Clamp#(5#, 0#, 10#)
' Output: 5
```

#### Result Type (`error.result`)
Structured error handling with typed failures, propagation context, and readable error chains.

**Functions:**
- `Result_Ok(result, value$)` - Set successful result
- `Result_Fail(result, message$)` - Set a generic error result
- `Result_FailCode(result, code&, message$)` - Set an error with an explicit code
- `Result_FailWithContext(result, code&, message$, context$, source$)` - Set an error with code, context, and source information
- `Result_AddContext(result, context$)` - Prepend outer context while propagating an error
- `Result_SetSource(result, source$)` - Record the subsystem, file, or module that emitted the error
- `Result_SetCause(result, cause$)` - Attach the underlying cause text
- `Result_Propagate(result, sourceResult, context$, source$)` - Copy an error forward and add outer context
- `Result_IsOk&(result)` - Check if result is OK
- `Result_IsError&(result)` - Check if result is an error
- `Result_Code&(result)` - Get the error code
- `Result_Value$(result, default$)` - Get result value
- `Result_Message$(result)` - Get the error message
- `Result_Context$(result)` - Get the accumulated context chain
- `Result_Source$(result)` - Get the source/subsystem text
- `Result_Cause$(result)` - Get the attached cause text
- `Result_ErrorChain$(result)` - Render a readable combined error chain
- `Result_Describe$(result)` - Describe the result in a readable single string
- `Result_Expect$(result, expectation$)` - Abort with a panic-style message if the result is an error

**Example:**
<!-- docs-test: prepend qbnex -->
```basic
DIM outcome AS QBNex_Result
DIM startup AS QBNex_Result

Result_Ok outcome, "stable"
IF Result_IsOk&(outcome) THEN
    PRINT "Success: "; Result_Value$(outcome, "")
END IF
' Output: Success: stable

Result_FailWithContext outcome, 404, "Configuration file not found", "while reading settings.json", "config.loader"
Result_SetCause outcome, "startup profile is missing from the project root"
Result_Propagate startup, outcome, "while starting application", "startup"

IF Result_IsError&(startup) THEN
    PRINT "Error: "; Result_ErrorChain$(startup)
END IF
' Output: Error: while starting application -> while reading settings.json: [E404] Configuration file not found [source=startup] | cause: startup profile is missing from the project root
```

### OOP Support

QBNex supports object-oriented programming with classes, inheritance, and interfaces.

**Class Declaration:**
```basic
'$IMPORT:'qbnex'

CLASS Animal
    Name AS STRING * 32
    Age AS INTEGER

    CONSTRUCTOR (petName AS STRING, petAge AS INTEGER)
        ME.Name = petName
        ME.Age = petAge
    END CONSTRUCTOR

    FUNCTION Describe$ ()
        Describe$ = RTRIM$(ME.Name) + " (age " + STR$(ME.Age) + ")"
    END FUNCTION
END CLASS
```

**Inheritance:**
<!-- docs-test: skip continues the CLASS Animal example above -->
```basic
CLASS Dog EXTENDS Animal
    Breed AS STRING * 32

    CONSTRUCTOR (petName AS STRING, petAge AS INTEGER, petBreed AS STRING)
        ME.Breed = petBreed
    END CONSTRUCTOR

    FUNCTION Bark$ ()
        Bark$ = "Woof!"
    END FUNCTION
END CLASS
```

**Using Classes:**
<!-- docs-test: skip continues the CLASS example above -->
```basic
DIM pet AS Dog
New_Dog pet, "Buddy", 3, "Collie"

PRINT "Name: "; pet.Name
PRINT "Describe: "; pet.Describe$
PRINT "Sound: "; pet.Bark$
```

**Interfaces:**
<!-- docs-test: skip fragment of a CLASS header -->
```basic
IMPLEMENTS IPet
' Class implements IPet interface
' Can be checked with QBNEX_Implements&()
```

**Runtime OOP API:**
- `QBNEX_RegisterClass$(className$, parentClassID)` - Register class
- `QBNEX_FindClass$(className$)` - Find class by name
- `QBNEX_RegisterMethod(classID, methodName$, slot)` - Register method
- `QBNEX_RegisterInterface(classID, interfaceName$)` - Register interface
- `QBNEX_ObjectInit(header, classID)` - Initialize object
- `QBNEX_ObjectClassName$(header)` - Get object class name
- `QBNEX_ObjectIs&(object, className$)` - Check inheritance
- `QBNEX_Implements&(classID, interfaceName$)` - Check interface
- `QBNEX_FindMethodSlot&(classID, methodName$)` - Find method slot

**Example:**
<!-- docs-test: skip continues the CLASS example above -->
```basic
DIM pet AS Dog
New_Dog pet, "Buddy", 3, "Collie"

PRINT "Class: "; QBNEX_ObjectClassName$(pet.Header)
PRINT "Is Animal: "; QBNEX_ObjectIs&(pet.Header, "Animal")
PRINT "Is Dog: "; QBNEX_ObjectIs&(pet.Header, "Dog")
PRINT "Has IPet: "; QBNEX_Implements&(pet.Header.ClassID, "IPet")
```


---

## Developer Tools

| Command | Program | What it does |
| --- | --- | --- |
| `qbnex fmt [options] [files]` | `qb-fmt` | Formatter: keyword case, block indentation, spacing. `-w` rewrites files, `--check` exits 1 when a file would change. Comments, strings, `DATA` and metacommands are never changed. |
| `qbnex lsp` | `qb-lsp` | Language server on stdin/stdout: diagnostics, outline, go to definition and references across `$INCLUDE` files, hover, completion, formatting. |
| `qbnex dap` | `qb-dap` | Debug adapter on stdin/stdout: compiles with `qb` and runs on the VM with breakpoints, stepping and variables. |
| VS Code extension | `packaging/vscode` | Syntax highlighting, the language server, the formatter and the debugger (F5). |
| `qb file.bas -o file.qir` | `crates/web` | QIR for the WebAssembly build of the runner, which runs programs in a browser (Preview). |

`qbnex fmt|lsp|dap` (and `qb.cmd fmt|lsp|dap` on Windows) finds the tools next to the script,
then in `target/release` or `target/debug` of the checkout, then on `PATH`. Build them with
`cargo build --release -p qb-fmt -p qb-lsp -p qb-dap`; release bundles include them.
Details: [commands](docs/manual/en/commands.md), [VS Code and tools](docs/manual/en/vscode.md),
[web build](docs/manual/en/web.md).

---

## Docker Guide

`docker/Dockerfile` builds QBNex on Ubuntu 22.04 (Rust plus the Linux build packages) and copies
the compiler, the runner and the standard library into a smaller runtime image. In that image
`qb` and `qbnex` are on PATH, the work folder is `/project`, and the container runs as the
non-root user `qbnex` (uid 1000), so files it writes into a mounted folder belong to uid 1000.

```bash
# Build the image (the build context is the repository root)
docker build -f docker/Dockerfile -t qbnex .

# Compile a program in the current folder
docker run --rm -v "$(pwd)":/project qbnex qb yourfile.bas -o yourfile   # Linux/macOS
docker run --rm -v "${PWD}:/project" qbnex qb yourfile.bas -o yourfile   # PowerShell
docker run --rm -v "%cd%:/project" qbnex qb yourfile.bas -o yourfile     # cmd.exe

# Run the result inside the container
docker run --rm -v "$(pwd)":/project qbnex ./yourfile
```

Give `-o` a path under `/project`: without it the compiler writes to the QBNex folder inside the
container (`/app`), which the `qbnex` user cannot write to. Without arguments the image prints `qb --help`.

**Docker Compose** (`docker/docker-compose.yml`) defines two services that mount the repository root
on `/project`, pass `DISPLAY` and use the host network:

```bash
docker compose -f docker/docker-compose.yml build
docker compose -f docker/docker-compose.yml run --rm qbnex qb yourfile.bas -o yourfile
docker compose -f docker/docker-compose.yml run --rm qbnex-dev        # bash in the dev image
```

The `qbnex-dev` service uses `docker/Dockerfile.dev`: one stage that keeps the cargo build cache,
starts `bash`, and runs as root.

**Graphics programs** need an X server on the host:

```bash
xhost +local:docker                                  # Linux
docker run --rm -e DISPLAY=$DISPLAY -v /tmp/.X11-unix:/tmp/.X11-unix \
    -v "$(pwd)":/project qbnex ./graphics
```

On macOS use XQuartz ("Allow connections from network clients", then `DISPLAY=host.docker.internal:0`);
on Windows an X server such as VcXsrv with access control disabled. Programs that only need the
screen contents can run without a window with `-e QBNEX_HEADLESS=1`.

**Network programs** (`_OPENHOST`, `_OPENCLIENT`): use `--network host` (Linux) or publish the
port with `-p 8080:8080`.

**Sound** is not passed into the container; programs run on the silent audio device there.

---

## Compilation Pipeline

```
┌─────────────────┐
│  BASIC Source   │  (yourfile.bas, UTF-8 or CP437)
└────────┬────────┘
         ▼
┌─────────────────┐
│   Pre-pass      │  $INCLUDE, $IF/$LET, '$IMPORT modules, DEF FN, CLASS lowering
└────────┬────────┘
         ▼
┌─────────────────┐
│  QIR emission   │  Translate BASIC to QIR (text IR, docs/dev/qir.md),
│                 │  written to target/qb-temp/program.qir
└────────┬────────┘
         ▼
┌─────────────────┐
│   Packaging     │  Copy the runner qb-exec and append the QIR
│                 │  (or link a native executable when the native kit exists)
└────────┬────────┘
         ▼
┌─────────────────┐
│   Executable    │  yourfile.exe (Windows), ./yourfile (Linux/macOS)
│                 │  qb-exec runs the QIR on its VM and JIT
└─────────────────┘
```

**Pre-pass.** The compiler reads the source, inlines `$INCLUDE` files, evaluates `$IF`/`$LET`,
resolves `'$IMPORT` lines (module declarations go before the program, procedures after it, so
diagnostics still name the module file and line), rewrites `DEF FN` and CLASS into ordinary
procedures, and records variables, procedures and types.

<!-- docs-test: skip needs mylib.bas -->
```basic
'$IMPORT:'qbnex'
'$INCLUDE:'mylib.bas'

$LET DEBUG_MODE = 1
$IF DEBUG_MODE = 1 THEN
    PRINT "Debug mode enabled"
$END IF
```

**QIR emission.** Each statement is checked and lowered to QIR: typed registers, loads and stores
of variables, calls into the runtime (`rt PRINTS`, `rt SCREEN`, ...), labels and branches. The QIR
of the last build stays in `target/qb-temp/program.qir` in the QBNex folder (each running
compiler uses its own `target/qb-temp<N>` folder). A statement the QIR back end does not cover
stops the build with `QIR: unsupported: line N: ...`.

**Packaging and execution.** The output is a copy of the prebuilt runner
`crates/engine/exec/bin/<os>/qb-exec` followed by the QIR text, its length (u64, little endian)
and the tag `QBNEXIR0`. At start-up the runner reads that payload from its own file, validates it,
lowers it to bytecode and runs it on a register VM; hot functions are compiled by the Cranelift
JIT (`QB_JIT`). The runtime (window, graphics, sound, files, network) is the Rust crate
`qb-runtime` with `qb-audio`, `qb-gpu` and `qb-ext`.

**Sizes and speed** (Windows x64, this machine): hello world compiles in about 0.5 s; a
10,000-line program in about 15 s; the compiler itself in about 41 s. Every program is the
runner (about 12.7 MB, with graphics, sound, network and the JIT) plus its QIR text, so a hello
world and a graphics program are about the same size.

---

## Diagnostics

**Build screen** (in colour the logo shades from periwinkle through sky blue and mint to light
green; the progress bar is redrawn in place):
```
   ██████╗ ██████╗ ███╗   ██╗███████╗██╗  ██╗
  ██╔═══██╗██╔══██╗████╗  ██║██╔════╝╚██╗██╔╝
  ██║   ██║██████╔╝██╔██╗ ██║█████╗   ╚███╔╝
  ██║▄▄ ██║██╔══██╗██║╚██╗██║██╔══╝   ██╔██╗
  ╚██████╔╝██████╔╝██║ ╚████║███████╗██╔╝ ██╗
   ╚══▀▀═╝ ╚═════╝ ╚═╝  ╚═══╝╚══════╝╚═╝  ╚═╝

  ⚡ QBNex Compiler v2.0.0  ·  BASIC → QIR → executable
  📄 Source  C:\work\hello.bas

  🔨 Building  ████████████████████████████████ 100%  12 lines

  ✅ Build complete: C:\work\hello.exe  (0.3 s)
```

With `-m` (or `NO_COLOR` set) the banner is drawn with letters and the rest prints in plain ASCII:
```
Preparing build files... [########################################] 100%

Build complete: hello.exe
```

**A compile error** (detailed form, the default):
```
[x] QBNex :: Error [E1106]  IF statement is missing THEN or GOTO
  [@] app.bas(42,13)
  [#] source
    42 | IF score > 10 PRINT "win"
       |             ^ add THEN or GOTO here
  [>] next     Add THEN after the IF condition.
  [::] flow    Parsing :: parse source file -> file: app.bas
  [!] cause    The compiler parsed an IF condition but never found THEN or GOTO to finish the statement.
  [+] example  IF score > 10 THEN PRINT "win"

[x] QBNex :: Build Halted  1 blocking diagnostic(s)
```

`-k` / `--compact-errors` drops the `[!] cause` and `[+] example` notes.

| Marker | Meaning |
|--------|---------|
| `[x]` | Error diagnostic |
| `[!!]` | Fatal diagnostic |
| `[~]` | Warning diagnostic |
| `[i]` | Informational diagnostic |
| `[@]` | File location (`file(line,column)`) |
| `[#]` | Source snippet |
| `[>]` | Next action / fix hint |
| `[::]` | Phase and flow summary |
| `[*]` | Active compiler configuration note |
| `[=]` | Repeated diagnostic or hidden duplicate summary |
| `[^]` | Detailed location context |
| `[.]` | Detailed note |
| `[>>]` | Detailed in-progress context |
| `[!]` | Cause note |
| `[+]` | Example or corrected form |

**Runtime errors.** An unhandled runtime error in an interactive console shows the classic
"Unhandled Error #n ... Continue?" dialog. With redirected output or `QBNEX_NO_ERROR_DIALOG=1` it
prints one line to stderr and the program ends with exit code 1. An error in an `$INCLUDE` file
names that file and line.

---

## Testing & Verification

| Suite | Command | What it checks |
| --- | --- | --- |
| Rust unit and integration tests | `cargo test --workspace` (or `-p qb-runtime`, `-p qb-exec`, ...) | Runtime, VM, JIT, FFI, tools |
| Conformance | `cargo run --release -p qbnex-conformance` | Compiles every program in `tests/conformance/cases/` (156) with the built `qb` and compares the output with the golden files |
| Smoke scripts | `tests\smoke\<name>_smoke.cmd` / `sh tests/smoke/<name>_smoke.sh` | CLI and compiler behaviour end to end |
| Documentation | `pwsh tests/docs/check_docs.ps1` | Compiles every BASIC block of this README and `docs/manual`, runs those with a shown output ([tests/docs/README.md](tests/docs/README.md)) |
| Self-host fixpoint | `scripts\selfhost_fixpoint.cmd` / `sh scripts/selfhost_fixpoint.sh` | The compiler rebuilds itself to the same QIR, equal to `bootstrap/qbnex.qir` |

Smoke scripts (run from the repository root after setup; each prints `[PASS]` or a `*_SMOKE_OK`
marker and exits non-zero on failure):

| Script | Covers |
| --- | --- |
| `cli_smoke` | `--help`, `--version`, unknown switches, output paths, `-q`, `-s`, `-x`, `-z`, warnings-as-errors |
| `diagnostics_smoke` | Detailed and compact diagnostics, include-chain reporting |
| `warnings_smoke` | Warnings and `--warnings-as-errors` |
| `labels_smoke` | Label resolution, stale-output protection |
| `encoding_smoke` | UTF-16 and invalid UTF-8 rejection, BOM, CP437, empty source, spaced paths |
| `stdlib_smoke`, `stdlib_modules_smoke` | Standard library imports and includes, every module |
| `compiler_robustness_smoke` | Malformed input ends in a diagnostic, not an internal error |
| `audio_smoke` | `_VOICE`, `_ADSR`, `_WAVE`, `SOUND`, `PLAY` |
| `chain_smoke` | `CHAIN` with COMMON variables and arrays, also from a SUB |
| `benchmark_smoke` | Compile times of small, medium and large programs (manual, not in CI) |

Fixture programs live in `tests/fixtures/`. CI runs every smoke script except the benchmark.

```cmd
:: Windows: build, then the smoke scripts CI runs
set QBNEX_CI=1
scripts\setup_win.cmd
for %t in (cli diagnostics warnings labels encoding stdlib stdlib_modules compiler_robustness audio chain) do tests\smoke\%t_smoke.cmd
```

```bash
# Linux/macOS
sh scripts/setup_lnx.sh
for t in cli diagnostics warnings labels encoding stdlib stdlib_modules compiler_robustness audio chain; do
    sh "tests/smoke/${t}_smoke.sh" || break
done
```

**Writing a test program.** End with `SYSTEM <code>` so scripts see the result:

```basic
' test_basic.bas
DIM failures AS LONG

IF 2 + 2 <> 4 THEN failures = failures + 1
IF LEN("Hello") <> 5 THEN failures = failures + 1
IF LCASE$("HELLO") <> "hello" THEN failures = failures + 1

IF failures = 0 THEN
    PRINT "ALL_TESTS_PASSED"
    SYSTEM 0
ELSE
    PRINT "TEST_FAILURES: "; failures
    SYSTEM 1
END IF
```

```bash
qb test_basic.bas -o test_basic.exe
QBNEX_NO_ERROR_DIALOG=1 ./test_basic.exe
```

A change in behaviour gets a conformance case in `tests/conformance/cases/` (see CONTRIBUTING.md).

---

## Troubleshooting

**"cargo was not found" during setup.** Install Rust from https://rustup.rs and run the setup
script again, or point `QBNEX_QB_EXEC` at a prebuilt `qb-exec`.

**"Cannot locate source file".** Check the path. The compiler resolves a relative source path
against the current folder; when in doubt, give an absolute path.

**The executable is not where I expected.** Without `-o` it goes to the QBNex folder (next to
`qb`). Use `-o path` or `qb -s:exewithsource=true`.

**`QIR: unsupported: line N: ...`.** The statement is one of the few the QIR back end does not
compile yet; see [limitations](docs/manual/en/limitations.md#statements-that-are-not-supported-or-behave-differently).

**A console program waits at "Press any key to continue".** In an interactive console a program
that ends with `END` waits for a key, as in QBasic; with stdin or stdout redirected (or
`QBNEX_NO_ERROR_DIALOG=1`) it does not. End it with `SYSTEM` to never wait.

**A runtime error opens a dialog in CI.** Set `QBNEX_NO_ERROR_DIALOG=1`.

**Antivirus flags a compiled program.** Every compiled program is a new, unsigned executable
with data appended. Exclude the QBNex folder or your output folder from scanning.

**`permission denied: ./myprogram` (Linux/macOS).** `chmod +x myprogram`.

**No window or no sound.** On Linux check that the ALSA and X11/Wayland libraries are installed;
in Docker see the [Docker Guide](#docker-guide). `_GPU3D` returns 0 without a GPU runtime;
`SUB _GL` needs an OpenGL driver (`QB_GL=off` disables it).

**Import problems.** The form is `'$IMPORT:'module.name'` (apostrophe, `$IMPORT:`, the name in
single quotes). Modules are in `compiler/stdlib/`; the collections other than
`collections.list` need `'$IMPORT:'qbnex'`.

<!-- docs-test: skip shows wrong syntax on purpose -->
```basic
' Correct syntax (note the quotes and colon)
'$IMPORT:'qbnex'
'$IMPORT:'collections.list'
'$IMPORT:'sys.env'

' Wrong syntax (common mistakes)
'$IMPORT: qbnex'          ' Missing quotes around module
'$IMPORT:qbnex'           ' Missing inner quotes
IMPORT:'qbnex'            ' Missing $ prefix
```

**Sound test.**
```basic
BEEP
SOUND 440, 18  ' 440 Hz for 1 second

_VOICE 1
_ADSR .01, .05, .7, .08
_WAVE "SINE"
SOUND 440, 6

_VOICE 2
_WAVE "CUSTOM:0,1,0,-1"
SOUND 660, 6

_VOICE 0  ' return to automatic 4-voice round-robin
_WAVE "PINK", 3
PLAY "MB T180 O3 L8 CEG"
```

**Reporting bugs.** Include the platform, `qb --version`, a minimal program, the expected and the
actual behaviour, and the complete compiler output (`qb file.bas -w -m`). The QIR of the last
build is in `target/qb-temp/program.qir`. Security problems: see [SECURITY.md](SECURITY.md).

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

### Data Types

QBNex supports a comprehensive range of data types:

```basic
' types_demo.bas
DIM bitVar AS _BIT
DIM byteVar AS _BYTE
DIM intVar AS INTEGER
DIM longVar AS LONG
DIM int64Var AS _INTEGER64
DIM singleVar AS SINGLE
DIM doubleVar AS DOUBLE
DIM floatVar AS _FLOAT
DIM stringVar AS STRING * 20
DIM offsetVar AS _OFFSET

' Unsigned types
DIM uByte AS _UNSIGNED _BYTE
DIM uInt AS _UNSIGNED INTEGER
DIM uLong AS _UNSIGNED LONG
DIM uInt64 AS _UNSIGNED _INTEGER64

' Type conversion
DIM num AS INTEGER
num = CINT(VAL("123"))
PRINT "Converted: "; num
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
REDIM _PRESERVE dynamic(10) AS INTEGER
PRINT "Dynamic array element "; dynamic(1)
```

### File I/O

```basic
' fileio.bas
DIM text$
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
    LINE INPUT #1, text$
    count = count + 1
    PRINT "Read line "; count; " "; text$
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

The tables below list the classic QBasic statements and functions by category; QB64 `_` keywords are covered too (see the [manual](docs/manual/README.md)).

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
| `REDIM _PRESERVE` | Redimension keeping data | `REDIM _PRESERVE arr(30)`    |
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
| `_TRIM$`  | Trim both sides   | `_TRIM$("  Hi  ")` → "Hi"     |
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
| `REDIM _PRESERVE` | Resize keeping data | `REDIM _PRESERVE arr(30)` |
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

The compiler is BASIC and compiles itself:

1. **Bootstrap**: `bootstrap/qbnex.qir` (the compiler, already translated to QIR) is packed into the runner `qb-exec`, giving the stage-0 compiler `qb-stage0`
2. **Self-hosting**: stage 0 compiles `compiler/qbnex.bas` and its modules into `qb` / `qb.exe`; `scripts/selfhost_fixpoint.*` checks that the next generations produce the same QIR and that it equals `bootstrap/qbnex.qir` (the Windows build of the compiler, so only Windows compares against it; elsewhere set `QBNEX_FIXPOINT_BOOTSTRAP=0`)
3. **Code generation**: the compiler translates BASIC into QIR
4. **Packaging**: the output is the runner with the program's QIR appended (no C/C++ compiler or linker)
5. **Runtime**: `qb-exec` runs the QIR with the Rust runtime crates

`compiler/qbnex.bas` is the main orchestrator; most of the compiler lives in
`compiler/utilities/` (QIR emission in `ir_*.bas`, CLI in `cli.bas`, shared state in
`state.bas`). Rebuild steps, required checks and code conventions: [CONTRIBUTING.md](CONTRIBUTING.md).

### Project Structure

```
QBNex/
├── .github/
│   ├── scripts/                # Release bundle and release-notes helpers
│   ├── workflows/              # ci.yml, release.yml, docs.yml
│   └── dependabot.yml
├── assets/                     # Logo and platform icons
├── bootstrap/
│   └── qbnex.qir               # The compiler in QIR (input of the setup scripts)
├── compiler/                   # QBNex compiler source (BASIC)
│   ├── qbnex.bas               # Main orchestrator entry point
│   ├── global/                 # Version, constants, settings
│   ├── includes/               # Shared include files
│   ├── stdlib/                 # Standard library modules (collections, error, ext, io,
│   │                           #   math, oop, strings, sys, json.bas, url.bas, qbnex_stdlib.bas)
│   ├── subs_functions/         # Built-in statement and function table
│   ├── support/                # $COLOR tables, SUB _GL keyword table, vwatch
│   └── utilities/              # Compiler modules (QIR emission, CLI, imports, DEF FN, ...)
├── crates/                     # Rust crates (Cargo workspace)
│   ├── compiler/ir/            # qb-ir: QIR types, text parser/printer, validator, builtin table
│   ├── compiler/syntax/        # qb-syntax: lexer and keyword tables for the tools
│   ├── engine/exec/            # qb-exec: runner for compiled QIR programs (VM, loader; bin/<os>/qb-exec)
│   ├── engine/jit/             # qb-jit: Cranelift JIT tier
│   ├── engine/link/            # qb-link: native code loading and executable writer
│   ├── engine/native/          # qb-native: runtime library linked into native executables
│   ├── engine/ffi/             # qb-ffi: DECLARE LIBRARY calls into shared libraries
│   ├── runtime/core/           # qb-runtime: window, graphics, console, files, network, memory, 3D scene
│   ├── runtime/audio/          # qb-audio: audio engine (SOUND, PLAY, _SND*)
│   ├── runtime/float80/        # qb-float80: software 80-bit _FLOAT
│   ├── runtime/gpu/            # qb-gpu: optional wgpu presentation and 3D renderer
│   ├── runtime/ext/            # qb-ext: Rust side of the ext.* standard library modules
│   ├── tools/fmt/              # qb-fmt: formatter (qbnex fmt)
│   ├── tools/lsp/              # qb-lsp: language server (qbnex lsp)
│   ├── tools/dap/              # qb-dap: debug adapter (qbnex dap)
│   ├── tools/setup/            # qb-setup: the one-file installer qbnex-setup
│   └── web/                    # qb-web: WebAssembly build of the runner
├── docker/                     # Dockerfile, Dockerfile.dev, docker-compose.yml
├── docs/
│   ├── manual/                 # User manual (en, th)
│   └── dev/                    # Developer reference (QIR, deviations, FFI, 3D API)
├── licenses/                   # License texts of QB64 and the third-party components
├── packaging/
│   └── vscode/                 # VS Code extension
├── scripts/                    # setup_win.cmd, setup_lnx.sh, setup_osx.command, selfhost_fixpoint.*,
│                               # native-kit.*, qir_pack.ps1, reset.cmd, qbnex-launcher.sh,
│                               # third_party_licenses.py, bench/
├── tests/
│   ├── conformance/            # qbnex-conformance runner + cases/ with golden outputs
│   ├── corpus-local/           # Your own BASIC programs (ignored by git)
│   ├── data/                   # Fonts, images and sounds used by tests
│   ├── docs/                   # check_docs.ps1: compiles the BASIC examples of the docs
│   ├── fixtures/               # Programs used by the smoke scripts
│   └── smoke/                  # Smoke and benchmark scripts (*.cmd / *.sh)
├── Cargo.toml                  # Cargo workspace (version 2.0.0)
├── rust-toolchain.toml         # Pinned Rust version (also used by CI)
├── qb.cmd, qbnex.cmd           # Windows launchers (compiler + fmt/lsp/dap subcommands)
├── README.md, CHANGELOG.md, CONTRIBUTING.md, SECURITY.md
├── LICENSE, LICENSE-EXCEPTION.txt, NOTICE.txt, TRADEMARKS.md, CLA.md
└── .dockerignore, .gitattributes, .gitignore

# Generated by setup and builds (not source, ignored by git)
# - target/ (cargo output, target/qb-temp*, target/qb-config.ini)
# - qb, qb.exe, qbnex, qb-stage0(.exe)
# - crates/engine/exec/bin/ (the prebuilt runner and the native kit)
# - dist/ (release bundles)
```

### Adding New Features

- **Built-in statements and functions**: `compiler/subs_functions/subs_functions.bas`, the QIR builtin table in `crates/compiler/ir/src/builtins.rs`, the implementation in `crates/engine/exec/src/rt/` and the runtime crates; the keyword list for the tools in `crates/compiler/syntax`
- **Compiler core**: `compiler/qbnex.bas` and `compiler/utilities/` (QIR format: `docs/dev/qir.md`)
- **VM / JIT**: `crates/engine/exec/src/vm.rs`, native code in `crates/engine/jit`
- **Runtime**: `crates/runtime/core`, `crates/runtime/audio`, `crates/runtime/gpu`, `crates/runtime/ext`
- **Editor tools**: `crates/tools/fmt`, `crates/tools/lsp`, `crates/tools/dap`, and `packaging/vscode`
- **Behaviour changes**: a conformance case in `tests/conformance/cases`; an intentional difference from QB64 gets a `D-xx` entry in `docs/dev/deviations.md`; user-visible changes go into `docs/manual` (en and th) and [CHANGELOG.md](CHANGELOG.md)

### Continuous Integration and Releases

- **`ci.yml`** (pushes and pull requests to `main` and `rust-port`, and weekly for the audit): dependency audit, build, clippy, Rust tests, smoke scripts, conformance, self-host fixpoint and native executables on Linux, macOS and Windows x64. A commit message containing `ci-skip` skips the push jobs. Actions are pinned to commit SHAs and run with read-only permissions.
- **`release.yml`** (tags `v*` and manual dispatch): builds and packages `qbnex_<tag>_win-x64.zip`, `qbnex_<tag>_lnx.tar.gz`, `qbnex_<tag>_osx.tar.gz` and the matching `-setup` installers, tests the installers, signs `qb-fmt`, `qb-lsp` and `qb-dap` when signing secrets are set, and publishes a GitHub Release.
- **`docs.yml`** (pushes and pull requests that change `README.md`, `docs/` or `tests/docs/`): runs `tests/docs/check_docs.ps1`, so every BASIC block of the README and the manual still compiles and prints what it shows. `ci.yml` skips pushes that only change prose.

There are no 32-bit Windows builds.

```bash
git tag v2.0.0
git push origin v2.0.0
```

---

## Contributing

Contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) for the build, the required
checks and the pull request process; contributions are accepted under the
[Contributor License Agreement](CLA.md).

- Report bugs and suggest features through GitHub Issues
- Improve documentation and examples (English and Thai)
- Add conformance cases and tests
- Fix bugs and improve performance of the compiler or runtime

For security vulnerabilities, read [SECURITY.md](SECURITY.md); **do not** open a public issue.

---

## License

Copyright (C) 2026 Thirawat Sinlapasomsak. QBNex is licensed under the **GNU General Public License, version 3 or later** ([LICENSE](LICENSE)), with the **QBNex Compiled Program Exception** ([LICENSE-EXCEPTION.txt](LICENSE-EXCEPTION.txt)).

- **Your programs are yours.** A program you compile with QBNex carries a copy of the QBNex runtime; the exception lets you license and sell that program under any terms, without publishing its source.
- **QBNex stays free.** Anyone may use, study, change and share QBNex. Whoever distributes a changed compiler or runtime must release its source under the same license.
- **Commercial license.** To ship a changed compiler or runtime without releasing its source, ask the copyright holder for a separate license.
- **Name and logo.** "QBNex" and the logo are not covered by the license; forks use a name of their own. See [TRADEMARKS.md](TRADEMARKS.md).
- **Contributions** are accepted under the [Contributor License Agreement](CLA.md).

QBNex is based on QB64; the parts that come from it keep their MIT license ([licenses/QB64.txt](licenses/QB64.txt)). Versions released before this change remain available under the MIT license. The licenses of the other components are in [licenses/COPYING.TXT](licenses/COPYING.TXT) and [licenses/THIRD_PARTY.TXT](licenses/THIRD_PARTY.TXT). See [NOTICE.txt](NOTICE.txt).

---

## Acknowledgments

- **QBasic/QuickBASIC** - The original BASIC implementation by Microsoft that inspired this project
- **QB64** - Modern QBasic compiler that served as the foundation for QBNex
- **FreeGLUT, miniaudio, FreeType, STB Image, GLEW** - The C libraries the QB64 runtime was built on; the QBNex runtime now uses Rust crates instead (winit, softbuffer, cpal, Symphonia, skrifa, png, zune-jpeg, wgpu, Cranelift and more, see `licenses/THIRD_PARTY.TXT`)
- **Contributors** - Everyone who has contributed code, documentation, or feedback to this project

---

## Links

- **Repository**: [https://github.com/thirawat27/QBNex](https://github.com/thirawat27/QBNex)
- **Issues**: [https://github.com/thirawat27/QBNex/issues](https://github.com/thirawat27/QBNex/issues)
- **Changelog**: [CHANGELOG.md](CHANGELOG.md)
- **Security Policy**: [SECURITY.md](SECURITY.md)
- **Contributing Guide**: [CONTRIBUTING.md](CONTRIBUTING.md)

---

<div align="center">
  <strong>Built with ❤️</strong>
  <br>
  <p>Created by <a href="https://github.com/thirawat27">thirawat27</a></p>
</div>
