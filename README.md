<div align="center">
  <img src="assets/QBNex.png" alt="QBNex Logo" width="256" height="256">
  
  # QBNex
  
  **Modern QBasic/QuickBASIC Compiler**
  
  > **A high-performance, modernized BASIC compiler.** Experience the classic nostalgia of full QB4.5/QBasic compatibility, seamlessly supercharged with advanced OpenGL graphics, modern syntax, TCP/IP networking, and native cross-platform binaries for Windows, Linux, and macOS.
  
</div>

---

## Table of Contents

- [About](#about)
- [Features](#features)
- [System Requirements](#system-requirements)
- [Installation](#installation)
  - [Windows Setup](#windows-setup)
  - [macOS Setup](#macos-setup)
  - [Linux Setup](#linux-setup)
  - [Docker Setup](#docker-setup)
  - [Build from Source](#build-from-source)
- [Quick Start](#quick-start)
- [Usage Guide](#usage-guide)
  - [Basic Compilation](#basic-compilation)
  - [Compiler Flags Reference](#compiler-flags-reference)
  - [Compiler Settings Configuration](#compiler-settings-configuration)
  - [Standard Library Imports](#standard-library-imports)
  - [Working with Modules](#working-with-modules)
- [Docker Complete Guide](#docker-complete-guide)
  - [Basic Docker Usage](#basic-docker-usage)
  - [Docker Compose Workflow](#docker-compose-workflow)
  - [Graphics Programs in Docker](#graphics-programs-in-docker)
  - [Network Programs in Docker](#network-programs-in-docker)
  - [Interactive Development](#interactive-development)
  - [Advanced Docker Configuration](#advanced-docker-configuration)
  - [Docker Troubleshooting](#docker-troubleshooting)
- [Standard Library Reference](#standard-library-reference)
  - [Collection Libraries](#collection-libraries)
  - [String Libraries](#string-libraries)
  - [I/O Libraries](#io-libraries)
  - [System Libraries](#system-libraries)
  - [Math & Error Handling](#math--error-handling)
  - [OOP Support](#oop-support)
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
- [Testing & Verification](#testing--verification)
- [Development](#development)
- [Contributing](#contributing)
- [License](#license)
- [Acknowledgments](#acknowledgments)

---

## About

**QBNex** is a modern QBasic/QuickBASIC compiler that translates BASIC source code into optimized C++ and compiles to native binaries for Windows, Linux, and macOS. It was significantly refactored from QB64 to act as a sleek, CLI-driven compiler without the legacy IDE components.

**Version**: 1.0.0

The compiler is self-hosting, written in QBNex BASIC itself (~26,000 lines), and supports 150+ QBasic/QB64 keywords. It features comprehensive graphics via OpenGL/FreeGLUT, sound synthesis via miniaudio, TCP/IP networking, and full file I/O operations.

Repository: https://github.com/thirawat27/QBNex

Additional documentation:
- Architecture: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- Development workflow: [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)
- Benchmark baseline: [docs/BENCHMARKS.md](docs/BENCHMARKS.md)

---

## Features

- **Comprehensive Language Support**
  - Full support for classic QBasic/QB4.5 syntax (100% backward compatible)
  - Modern extended syntax: `IMPORT module`, `x += 1`, `# comments`
  - AS TYPE syntax: `FUNCTION name AS STRING` instead of `FUNCTION name$`
  - User-defined types (TYPE...END TYPE) with nested structures
  - Subroutines and functions with parameters
  - Multi-dimensional arrays with REDIM PRESERVE
  - Standard library imports via `IMPORT module` or `'$IMPORT:'module.name'`
  - 150+ QBasic/QB64 keywords and functions
  - Extended data types: BIT, BYTE, _INTEGER64, _FLOAT, OFFSET (pointers)
  - Unsigned integer types (UNSIGNED BYTE, UNSIGNED INTEGER, UNSIGNED LONG, UNSIGNED _INTEGER64)

- **Modern Execution**
  - Self-hosting compiler written in QBNex BASIC (~26,000 lines)
  - Transpiles BASIC source to optimized C++ before compilation
  - Compiles to native binaries for maximum performance
  - CLI-driven compilation optimized for modern terminal workflows
  - Cross-platform support: Windows (x86/x64), Linux, macOS

- **Advanced Graphics & Sound**
  - OpenGL-based graphics subsystem with FreeGLUT
  - Automatic detection of graphics/sound features
  - SCREEN modes with VGA and hi-res graphics support (SCREEN 0-12+)
  - Drawing primitives (LINE, CIRCLE, PAINT, DRAW with macro strings)
  - Image manipulation (GET/PUT with PSET, AND, OR, XOR transfer modes)
  - TrueType font rendering via FreeType library
  - Image format loading (BMP, PCX, PNG, JPEG, etc.) via STB Image
  - Sound synthesis via miniaudio library (SOUND, PLAY, BEEP)
  - Cross-platform audio: ALSA (Linux), CoreAudio (macOS), Windows Multimedia

- **Network Capabilities**
  - TCP/IP networking with socket support
  - Server sockets (`_OPENHOST`)
  - Client connections (`_OPENCLIENT`)
  - Connection management functions
  - Conditional compilation via DEPENDENCY_SOCKETS

- **File System Operations**
  - Sequential file access (OPEN, PRINT #, WRITE #, INPUT #, LINE INPUT #)
  - Random file access (OPEN FOR RANDOM, GET, PUT, FIELD, LSET, RSET)
  - Binary file access (OPEN FOR BINARY, GET, PUT)
  - Directory operations (MKDIR, CHDIR, RMDIR)
  - File management (KILL, NAME...AS, FILES)
  - Binary load/save (BLOAD, BSAVE)

- **Developer Tools**
  - Compiler version tracking (`-v` flag)
  - Help and documentation (`-h`, `--help`)
  - Examples display (`-g` flag)
  - Warning system with `-w` flag
  - Quiet mode (`-q` flag)
  - Monochrome output option (`-m` flag)
  - Settings management (`-s` flag)
  - Pre-compiled content purge (`-p` flag)
  - C code generation without compilation (`-z` flag)
  - OPTION _EXPLICIT enforcement (`-e` flag)
  - Compile and run immediately (`-x` flag)
  - Custom output naming (`-o` flag)
  - Debug mode with GDB information
  - Configurable compiler settings via INI file

- **System Integration**
  - Timer and date/time functions
  - Command-line argument access
  - Environment variable queries
  - Low-level memory operations (PEEK, POKE, DEF SEG, VARPTR)
  - Process control (SHELL, CHAIN, CALL, CALL ABSOLUTE)
  - Error handling with ON ERROR GOTO, RESUME

---

## System Requirements

### Platform Requirements

**Windows:**
- Windows 7 or newer (32-bit or 64-bit)
- No additional setup required (MinGW is downloaded automatically by setup script)
- Recommended: Whitelist QBNex folder in antivirus software

**macOS:**
- macOS with Xcode Command Line Tools installed
- Install with: `xcode-select --install`
- OpenGL and GLUT libraries (typically pre-installed)
- CoreAudio for sound output

**Linux:**
- GNU C++ compiler (`g++`)
- OpenGL development libraries (`libglu1-mesa-dev`)
- ALSA development libraries (`libasound2-dev`)
- FreeGLUT development libraries
- X11 libraries
- ncurses library

### Dependencies

**Core Dependencies (installed automatically or required):**
- OpenGL, GLU, GLEW, FreeGLUT (graphics)
- miniaudio library (audio)
- FreeType (TrueType fonts, optional)
- STB Image (image format loading)

**Platform-Specific:**
- Windows: MinGW g++ (auto-downloaded), Windows Multimedia library
- Linux: ALSA (`libasound2-dev`), X11
- macOS: CoreAudio, Apple GLUT, Cocoa

**Optional:**
- ZLIB (compression support)
- Socket libraries (networking support)

---

## Installation

Download the appropriate package for your operating system from the repository releases page, or build from source using the provided setup scripts.

### Windows Setup

Extract the package to a folder with full write permissions.

It is advisable to whitelist the QBNex folder in your antivirus or antimalware software.

**Building from source:**
```cmd
setup_win.cmd
```

The setup script will:
1. Download MinGW compiler (64-bit or 32-bit based on your choice)
2. Build library files (LibQB, FreeType, FreeGLUT)
3. Compile the QBNex compiler
4. Create `qb.exe` in the project root

**Note:** The script downloads ~150MB of MinGW binaries. Internet connection required.

### macOS Setup

Install the Xcode command line tools first:

```bash
xcode-select --install
```

Run the setup script:

```bash
chmod +x setup_osx.command
./setup_osx.command
```

The script will:
- Install required dependencies via Homebrew
- Compile OpenGL, FreeGLUT, and audio libraries
- Build the QBNex compiler
- Create `qb` executable

**Required packages:** OpenGL, GLUT, CoreAudio, Cocoa

### Linux Setup

Run the setup script:

```bash
chmod +x setup_lnx.sh
./setup_lnx.sh
```

**Required packages:**
```bash
# Debian/Ubuntu
sudo apt-get install build-essential libglu1-mesa-dev libasound2-dev freeglut3-dev libx11-dev libncurses5-dev

# Fedora/RHEL
sudo dnf install gcc-c++ libglu-devel libasound-devel freeglut-devel libX11-devel ncurses-devel

# Arch Linux
sudo pacman -S gcc glu alsa-lib freeglut libx11 ncurses
```

The setup script compiles all libraries and creates the `qb` compiler binary.

### Docker Setup

QBNex provides Docker support for consistent cross-platform builds without installing dependencies locally.

**Quick Start:**

```bash
# Build the Docker image
docker build -t qbnex .

# Or using docker-compose
docker-compose build
```

**Compile a program:**

```bash
# Linux/macOS
docker run --rm -v $(pwd):/project qbnex qb yourfile.bas

# Windows (PowerShell)
docker run --rm -v ${PWD}:/project qbnex qb yourfile.bas

# Windows (Command Prompt)
docker run --rm -v %cd%:/project qbnex qb yourfile.bas
```

**Compile and run immediately:**

```bash
docker run --rm -v $(pwd):/project qbnex qb yourfile.bas -x
```

**Full Docker documentation:** See the [Docker Complete Guide](#docker-complete-guide) section below.

### Build from Source

**Windows:**
```cmd
git clone https://github.com/thirawat27/QBNex.git
cd QBNex
setup_win.cmd
```

**Linux:**
```bash
git clone https://github.com/thirawat27/QBNex.git
cd QBNex
chmod +x setup_lnx.sh
./setup_lnx.sh
```

**macOS:**
```bash
git clone https://github.com/thirawat27/QBNex.git
cd QBNex
chmod +x setup_osx.command
./setup_osx.command
```

**Docker:**
```bash
git clone https://github.com/thirawat27/QBNex.git
cd QBNex
docker build -t qbnex .
docker run --rm -v $(pwd):/project qbnex qb source/qbnex.bas -w
```

---

## Quick Start

Get up and running with QBNex in 3 simple steps:

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

**3. Run the executable:**
```bash
# Windows
hello.exe

# Linux/macOS
./hello
```

That's it! You've successfully compiled your first QBNex program.

---

## Usage Guide

### Basic Compilation

Use `qb` or `qbnex` as the command name (depending on your setup):

```bash
# Compile to executable (creates hello.exe or ./hello)
qb hello.bas

# Compile with custom output name
qb hello.bas -o myprogram.exe

# Compile and run immediately
qb hello.bas -x

# Generate C code without compiling
qb hello.bas -z

# Show compiler version
qb --version

# Show help
qb --help
```

**Command Pattern:**
```bash
qb <source.bas> [flags]
```

### Compiler Flags Reference

| Flag | Aliases | Description | Example |
|------|---------|-------------|---------|
| `-h` | `--help` | Show help information | `qb -h` |
| `-v` | `--version` | Show compiler version | `qb -v` |
| `-i` | `--info`, `--about` | Show project information | `qb -i` |
| `-g` | `--examples` | Show common CLI examples | `qb -g` |
| `-c` | - | Compile the source file (default) | `qb file.bas -c` |
| `-o` | - | Specify output filename | `qb file.bas -o myapp.exe` |
| `-x` | - | Compile with console-mode CLI behavior | `qb file.bas -x` |
| `-w` | - | Show warnings during compilation | `qb file.bas -w` |
| `-Werror` | `--warnings-as-errors` | Promote warnings to blocking diagnostics | `qb file.bas --warnings-as-errors` |
| `-q` | - | Quiet mode (minimal output) | `qb file.bas -q` |
| `-m` | - | Monochrome (no color) output | `qb file.bas -m` |
| `-d` | `--verbose-errors` | Legacy alias for detailed diagnostics (enabled by default) | `qb file.bas` |
| `-k` | `--compact-errors` | Use compact diagnostics (hide detailed notes) | `qb file.bas -k` |
| `-e` | - | Enable OPTION _EXPLICIT | `qb file.bas -e` |
| `-s` | - | View/edit compiler settings | `qb -s:DebugInfo=true` |
| `-p` | - | Purge all pre-compiled content | `qb file.bas -p` |
| `-z` | - | Generate C code only (no exe) | `qb file.bas -z` |

**Recommended for debugging diagnostics:**
```bash
# Detailed diagnostics are enabled by default
qb myprogram.bas

# Combine warnings when chasing follow-on failures
qb myprogram.bas -w

# Switch back to compact diagnostics when you only want headline + source snippet
qb myprogram.bas --compact-errors
```

Detailed output sections used by the modern QBNex formatter (`cause`, `example`, `where`, `while`, and related context notes) are now shown by default. `-d` and `--verbose-errors` are kept as backward-compatible aliases.

**Combining Flags:**
```bash
# Compile with warnings and custom output
qb myprogram.bas -w -o myapp.exe

# Quiet mode, generate C code only
qb myprogram.bas -q -z

# Compile, run immediately, with explicit mode
qb myprogram.bas -e -x
```

### Compiler Settings Configuration

QBNex supports configurable compiler settings via the `-s` flag or `internal/config.ini`:

**Available Settings:**

| Setting | Values | Default | Description |
|---------|--------|---------|-------------|
| `SaveExeWithSource` | `true`/`false` | `false` | Include source code in compiled executable |
| `IgnoreWarnings` | `true`/`false` | `false` | Suppress warning messages during compilation |
| `DebugInfo` | `true`/`false` | `false` | Include GDB debugging information in output |

**Viewing Settings:**
```bash
# View all current settings
qb -s

# Output shows:
# SaveExeWithSource = false
# IgnoreWarnings = false
# DebugInfo = false
```

**Modifying Settings:**
```bash
# Enable debug mode for GDB
qb -s:DebugInfo=true

# Disable warnings
qb -s:IgnoreWarnings=true

# Include source in executable
qb -s:SaveExeWithSource=true

# Combine multiple settings
qb -s:DebugInfo=true -s:IgnoreWarnings=false
```

**Using Config File:**

Edit `internal/config.ini` directly:
```ini
[Compiler]
SaveExeWithSource=false
IgnoreWarnings=false
DebugInfo=false
```

**When to Use Each Setting:**

- **DebugInfo=true**: When debugging with GDB, adds symbol information
- **IgnoreWarnings=true**: For clean output in CI/CD pipelines
- **SaveExeWithSource=true**: For distributing source with binary

### Standard Library Imports

QBNex supports both traditional and modern import syntax for the bundled standard library:

**Modern Syntax (Recommended):**
```basic
' Modern import syntax - cleaner and easier to read
IMPORT qbnex
IMPORT json
IMPORT url
```

**Traditional Syntax (Still Supported):**
```basic
' Traditional import syntax
'$IMPORT:'qbnex'
'$IMPORT:'json'
'$IMPORT:'url'
```

**Basic Import Syntax:**
```basic
' Import individual modules
'$IMPORT:'sys.env'
'$IMPORT:'io.path'
'$IMPORT:'strings.text'
'$IMPORT:'collections.list'
'$IMPORT:'math.numeric'
```

**Import Core Library:**
```basic
' Import the full qbnex stdlib core (place at top of file)
'$IMPORT:'qbnex'

' Now you can use QBNex_ObjectHeader, QBNex_List, etc.
DIM myList AS QBNex_List
List_Init myList
List_Add myList, "Hello"
```

**Best Practices:**

1. **Function-only imports**: Place at end of file
```basic
SUB Main ()
    PRINT Env_Platform$
    PRINT Path_Join$("root", "demo.txt")
END SUB

'$IMPORT:'sys.env'
'$IMPORT:'io.path'
```

2. **Full core imports**: Place at top of file (for TYPE, CLASS, SUB, FUNCTION)
```basic
'$IMPORT:'qbnex'

CLASS Dog
    Name AS STRING * 32
    
    CONSTRUCTOR (petName AS STRING)
        ME.Name = petName
    END CONSTRUCTOR
END CLASS
```

**Modern QBNex Syntax (QBasic + Extended):**

QBNex now supports a modern, cleaner syntax while maintaining full backward compatibility with traditional QBasic:

| Feature | Modern Syntax | Traditional Syntax | Description |
|---------|--------------|-------------------|-------------|
| Import | `IMPORT module` | `'$IMPORT:'module'` | Import stdlib modules |
| Function Type | `FUNCTION name AS STRING` | `FUNCTION name$` | AS TYPE syntax |
| Short Function | `FUNC name()` | `FUNCTION name()` | Shorter declaration |
| Single-line Func | `DEF name(x) = x*2` | `FUNCTION...END FUNCTION` | Lambda-like syntax |
| Augmented Assign | `x += 1` | `x = x + 1` | += -= *= /= operators |
| Alternative Comment | `# comment` | `' comment` | # for comments |

**Modern API Examples:**

```basic
# Import standard library
IMPORT qbnex

# JSON handling
json = json_parse("{""name"": ""John""}")
name = json_get_str(json)
output = json_string(json)

# URL encoding/decoding
encoded = encode("hello world")
decoded = decode(encoded)

# Augmented assignment (like other modern languages)
count += 1      # count = count + 1
total -= 5      # total = total - 5
value *= 2      # value = value * 2
average /= n    # average = average / n
```

**Available Standard Library Modules:**

| Module | Modern Import | Description |
|--------|-------------|-------------|
| QBNex Core | `IMPORT qbnex` | Full stdlib with JSON and URL helpers |
| JSON | Built-in | json_parse, json_string, json_obj, json_array |
| URL | Built-in | encode, decode, url_parse, path_join |

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

'$IMPORT:'io.json'
'$IMPORT:'error.result'
```

---

## Docker Complete Guide

QBNex provides Docker containers for easy deployment without local dependencies installation. This is the recommended approach for CI/CD, reproducible builds, and development environments.

### Basic Docker Usage

**Building the Image:**
```bash
# Build using Dockerfile directly
docker build -t qbnex .

# Or using docker-compose (recommended)
docker-compose build
```

**Running Basic Commands:**
```bash
# Compile a BASIC program
docker run --rm -v $(pwd):/project qbnex qb yourfile.bas

# Show help
docker run --rm qbnex qb --help

# Show version
docker run --rm qbnex qb --version

# List examples
docker run --rm qbnex qb --examples
```

**Platform-Specific Volume Mounting:**
```bash
# Linux/macOS
docker run --rm -v $(pwd):/project qbnex qb yourfile.bas

# Windows PowerShell
docker run --rm -v ${PWD}:/project qbnex qb yourfile.bas

# Windows Command Prompt
docker run --rm -v %cd%:/project qbnex qb yourfile.bas
```

### Docker Compose Workflow

**docker-compose.yml** provides simplified management:

```yaml
version: '3.8'
services:
  qbnex:
    build:
      context: .
      dockerfile: Dockerfile
    volumes:
      - .:/project
    working_dir: /project
```

**Using Docker Compose:**

```bash
# Build the image
docker-compose build

# Compile a program
docker-compose run --rm qbnex qb yourfile.bas

# Compile and run immediately
docker-compose run --rm qbnex qb yourfile.bas -x

# Generate C code without compiling
docker-compose run --rm qbnex qb yourfile.bas -z

# Compile with custom output name
docker-compose run --rm qbnex qb yourfile.bas -o myprogram

# Compile with warnings
docker-compose run --rm qbnex qb yourfile.bas -w

# Quiet mode compilation
docker-compose run --rm qbnex qb yourfile.bas -q
```

**Complete Workflow Example:**
```bash
# 1. Build image once
docker-compose build

# 2. Create hello.bas
echo 'PRINT "Hello from Docker!"' > hello.bas

# 3. Compile and run
docker-compose run --rm qbnex qb hello.bas -x

# 4. Check compiled binary
ls -la hello
./hello
```

### Graphics Programs in Docker

Graphics programs require X11 display forwarding to work. Here's how to set it up:

**Linux (X11):**
```bash
# Allow Docker to access X11
xhost +local:docker

# Run with display forwarding
docker run --rm \
  -v $(pwd):/project \
  -e DISPLAY=$DISPLAY \
  -v /tmp/.X11-unix:/tmp/.X11-unix \
  qbnex qb graphics.bas -x

# Or with docker-compose
docker-compose run --rm \
  -e DISPLAY=$DISPLAY \
  -v /tmp/.X11-unix:/tmp/.X11-unix \
  qbnex qb graphics.bas -x
```

**macOS (XQuartz):**
```bash
# 1. Install XQuartz
brew install --cask xquartz

# 2. Start XQuartz and enable network connections
# Open XQuartz → Preferences → Security → 
# Check "Allow connections from network clients"

# 3. Run with display forwarding
docker run --rm \
  -v $(pwd):/project \
  -e DISPLAY=host.docker.internal:0 \
  qbnex qb graphics.bas -x
```

**Windows (VcXsrv):**
```bash
# 1. Install VcXsrv X Server
# Download from: https://sourceforge.net/projects/vcxsrv/

# 2. Start VcXsrv with "Disable access control" checked

# 3. Run with display forwarding
docker run --rm \
  -v ${PWD}:/project \
  -e DISPLAY=host.docker.internal:0 \
  qbnex qb graphics.bas -x
```

**Graphics Demo Example:**
```basic
' graphics.bas
SCREEN 12
COLOR 15, 1
CLS

PRINT "QBNex Graphics in Docker!"
PRINT

' Draw shapes
LINE (50, 50)-(300, 200), 14, B
CIRCLE (400, 125), 75, 12

FOR i = 0 TO 639 STEP 20
    LINE (i, 0)-(639 - i, 479), 9
NEXT i

PRINT "Graphics working!"
SLEEP
SCREEN 0
```

Compile and run:
```bash
docker-compose run --rm -e DISPLAY=$DISPLAY -v /tmp/.X11-unix:/tmp/.X11-unix qbnex qb graphics.bas -x
```

### Network Programs in Docker

For TCP/IP networking, use host network mode:

```bash
# Using docker run with host network
docker run --rm \
  -v $(pwd):/project \
  --network host \
  qbnex qb server.bas -x

# Using docker-compose with host network
docker-compose run --rm --network host qbnex qb server.bas -x
```

**Server Example:**
```basic
' server.bas
DIM serverHandle AS LONG
DIM clientHandle AS LONG

serverHandle = _OPENHOST("TCP/IP:8080")
IF serverHandle = 0 THEN
    PRINT "Failed to create server"
    END
END IF

PRINT "Server listening on port 8080..."

DO
    clientHandle = _OPENCONNECTION(serverHandle)
    IF clientHandle > 0 THEN EXIT DO
    SLEEP 1
LOOP

PRINT "Client connected!"
_CLOSECONNECTION clientHandle
_CLOSECONNECTION serverHandle
```

**Client Example:**
```basic
' client.bas
DIM clientHandle AS LONG

clientHandle = _OPENCLIENT("TCP/IP:8080:localhost")
IF clientHandle = 0 THEN
    PRINT "Failed to connect"
    END
END IF

PRINT "Connected to server"
PRINT #clientHandle, "Hello!"
_CLOSECONNECTION clientHandle
```

Run server:
```bash
docker-compose run --rm --network host qbnex qb server.bas -x
```

Run client (in another terminal):
```bash
docker-compose run --rm --network host qbnex qb client.bas -x
```

### Interactive Development

For development with shell access:

```bash
# Start interactive shell
docker-compose run --rm qbnex bash

# Inside container:
# - Compile programs: qb myfile.bas
# - Run programs: ./myfile
# - List files: ls -la
# - Access source: /project (mounted from host)
```

**Interactive Session Example:**
```bash
$ docker-compose run --rm qbnex bash
root@abc123:/project# ls
hello.bas  test.bas

root@abc123:/project# qb hello.bas
Compiling...
Build complete: hello

root@abc123:/project# ./hello
Hello from QBNex!

root@abc123:/project# exit
```

**Development Mode with Extended Image:**

Use `Dockerfile.dev` for development with build cache:
```bash
# Build development image (~500MB with build cache)
docker build -f Dockerfile.dev -t qbnex-dev .

# Run with dev image
docker run --rm -v $(pwd):/project qbnex-dev qb yourfile.bas -z
```

### Advanced Docker Configuration

**Dockerfile (Production):**
```dockerfile
FROM ubuntu:22.04

# Install dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    libglu1-mesa-dev \
    libasound2-dev \
    freeglut3-dev \
    libx11-dev \
    libncurses5-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy QBNex source
WORKDIR /opt/qbnex
COPY . .

# Build compiler
RUN chmod +x setup_lnx.sh && ./setup_lnx.sh

# Add to PATH
ENV PATH="/opt/qbnex:${PATH}"

# Default command
CMD ["bash"]
```

**Dockerfile.dev (Development):**
```dockerfile
FROM ubuntu:22.04

# Install dependencies with build cache preserved
RUN apt-get update && apt-get install -y \
    build-essential \
    libglu1-mesa-dev \
    libasound2-dev \
    freeglut3-dev \
    libx11-dev \
    libncurses5-dev \
    gdb \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /opt/qbnex
COPY . .

# Build with debug info
RUN chmod +x setup_lnx.sh && ./setup_lnx.sh

ENV PATH="/opt/qbnex:${PATH}"
CMD ["bash"]
```

**docker-compose.yml:**
```yaml
version: '3.8'

services:
  qbnex:
    build:
      context: .
      dockerfile: Dockerfile
    volumes:
      - .:/project
    working_dir: /project
    # Optional: Enable host networking
    # network_mode: host
    # Optional: Enable graphics
    # environment:
    #   - DISPLAY=${DISPLAY}
    # volumes:
    #   - /tmp/.X11-unix:/tmp/.X11-unix
```

**CI/CD Integration:**
```yaml
# .github/workflows/docker-build.yml
name: Docker Build
on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Build Docker image
        run: docker build -t qbnex .
      
      - name: Compile test program
        run: docker run --rm -v $(pwd):/project qbnex qb test.bas
      
      - name: Run tests
        run: docker run --rm -v $(pwd):/project qbnex qb test.bas -x
```

### Docker Troubleshooting

**Permission Issues:**
```bash
# Fix permissions after compilation
sudo chmod +x ./yourprogram

# Or run with specific user
docker run --rm -v $(pwd):/project -u $(id -u):$(id -g) qbnex qb yourfile.bas
```

**Missing Libraries:**
```bash
# Check installed packages in container
docker run --rm qbnex dpkg -l | grep -E "libgl|libasound|x11"

# Rebuild without cache
docker-compose build --no-cache qbnex
```

**Display/Graphics Issues:**
```bash
# Test X11 forwarding
docker run --rm -e DISPLAY=$DISPLAY -v /tmp/.X11-unix:/tmp/.X11-unix xterm

# Check DISPLAY variable
echo $DISPLAY

# Allow Docker access (Linux)
xhost +local:docker
```

**Network Issues:**
```bash
# Test network connectivity
docker run --rm --network host qbnex ping localhost

# Check if port is available
docker run --rm --network host qbnex netstat -tulpn
```

**Volume Mount Issues:**
```bash
# Verify mounted files
docker run --rm -v $(pwd):/project qbnex ls -la /project

# Use absolute path
docker run --rm -v /absolute/path:/project qbnex qb yourfile.bas
```

**Docker Files Summary:**

| File | Purpose | Size | Use Case |
|------|---------|------|----------|
| `Dockerfile` | Production image | ~300MB | Final builds, CI/CD |
| `Dockerfile.dev` | Development image | ~500MB | Development, debugging |
| `.dockerignore` | Build exclusions | - | Optimizes build context |
| `docker-compose.yml` | Easy management | - | Recommended workflow |

**Best Practices:**

1. **Use docker-compose**: Simplifies commands and configuration
2. **Volume mounting**: Keep source on host, compile in container
3. **--rm flag**: Clean up containers after use
4. **Rebuild periodically**: Update dependencies with `docker-compose build --no-cache`
5. **Use dev image for debugging**: Includes GDB and build artifacts

**Security Notes:**

- Docker containers run as root by default
- Use `-u $(id -u):$(id -g)` for non-root execution
- Review Dockerfile for any security concerns
- Don't expose ports unless necessary
- Use `.dockerignore` to exclude sensitive files

---

## Standard Library Reference

QBNex includes a comprehensive standard library with modern data structures, I/O utilities, and OOP support. All libraries use Python-style import syntax and are located in `source/stdlib/`.

### Collection Libraries

#### List (`collections.list`)
Dynamic array with automatic resizing.

**Functions:**
- `List_Init(list)` - Initialize list
- `List_Add(list, item)` - Add item to end
- `List_Insert(list, index, item)` - Insert at position
- `List_Remove(list, index)` - Remove at position
- `List_Get$(list, index)` - Get item as string
- `List_Count&(list)` - Get item count
- `List_Join$(list, separator$)` - Join items with separator
- `List_Free(list)` - Free memory

**Example:**
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
LIFO (Last In, First Out) data structure.

**Functions:**
- `Stack_Init(stack)` - Initialize stack
- `Stack_Push(stack, item)` - Push item onto stack
- `Stack_Pop$(stack)` - Remove and return top item
- `Stack_Peek$(stack)` - View top item without removing
- `Stack_Count&(stack)` - Get item count
- `Stack_Free(stack)` - Free memory

**Example:**
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
FIFO (First In, First Out) data structure.

**Functions:**
- `Queue_Init(queue)` - Initialize queue
- `Queue_Enqueue(queue, item)` - Add item to back
- `Queue_Dequeue$(queue)` - Remove and return front item
- `Queue_Peek$(queue)` - View front item
- `Queue_Count&(queue)` - Get item count
- `Queue_Free(queue)` - Free memory

**Example:**
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
Hash-based collection with unique values.

**Functions:**
- `HashSet_Init(set)` - Initialize set
- `HashSet_Add(set, item)` - Add item (returns 0 if duplicate)
- `HashSet_Contains&(set, item)` - Check if item exists
- `HashSet_Remove(set, item)` - Remove item
- `HashSet_Count&(set)` - Get item count
- `HashSet_ToString$(set, separator$)` - Convert to string
- `HashSet_Free(set)` - Free memory

**Example:**
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

**Functions:**
- `Dict_Init(dict)` - Initialize dictionary
- `Dict_Set(dict, key$, value$)` - Set key-value pair
- `Dict_Get$(dict, key$, default$)` - Get value by key
- `Dict_Remove(dict, key$)` - Remove key-value pair
- `Dict_Count&(dict)` - Get item count
- `Dict_HasKey&(dict, key$)` - Check if key exists
- `Dict_Free(dict)` - Free memory

**Example:**
```basic
DIM metadata AS QBNex_Dictionary
Dict_Init metadata

Dict_Set metadata, "name", "QBNex"
Dict_Set metadata, "version", "1.0.0"
Dict_Set metadata, "kind", "compiler"

PRINT "Name: "; Dict_Get$(metadata, "name", "")
PRINT "Count: "; Dict_Count&(metadata)
' Output: Name: QBNex
' Output: Count: 3

Dict_Free metadata
```

### String Libraries

#### StringBuilder (`strings.strbuilder`)
Efficient string concatenation for building large strings.

**Functions:**
- `SB_Init(sb)` - Initialize string builder
- `SB_Append(sb, text$)` - Append text
- `SB_AppendLine(sb, text$)` - Append text with newline
- `SB_ToString$(sb)` - Convert to string
- `SB_Free(sb)` - Free memory

**Example:**
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
String manipulation and formatting utilities.

**Functions:**
- `Text_PadLeft$(text$, length, padChar$)` - Pad string on left
- `Text_PadRight$(text$, length, padChar$)` - Pad string on right
- Additional text manipulation functions

**Example:**
```basic
PRINT Text_PadRight$("QBNex", 10, ".")
' Output: QBNex.....

PRINT Text_PadLeft$("123", 8, "0")
' Output: 00000123
```

### I/O Libraries

#### Path Utilities (`io.path`)
Cross-platform file path manipulation.

**Functions:**
- `Path_Join$(path1$, path2$)` - Join path components
- `Path_FileName$(path$)` - Extract filename
- `Path_Directory$(path$)` - Extract directory
- `Path_Extension$(path$)` - Extract file extension
- `Path_WithoutExtension$(path$)` - Remove extension

**Example:**
```basic
PRINT Path_Join$("src", "stdlib/demo.bas")
' Output: src/stdlib/demo.bas

PRINT Path_FileName$("src/stdlib/demo.bas")
' Output: demo.bas

PRINT Path_Extension$("src/stdlib/demo.bas")
' Output: .bas
```

#### CSV Generation (`io.csv`)
CSV row creation and parsing.

**Functions:**
- `CSV_Row3$(col1$, col2$, col3$)` - Create 3-column CSV row
- Additional CSV parsing functions

**Example:**
```basic
PRINT CSV_Row3$("name", "score", "status")
' Output: name,score,status

PRINT CSV_Row3$("Alice", "100", "pass")
' Output: Alice,100,pass
```

#### JSON Generation (`io.json`)
JSON object creation.

**Functions:**
- `Json_Object3$(key1$, val1$, key2$, val2$, key3$, val3$)` - Create JSON with 3 pairs
- `Json_String$(text$)` - Create JSON string value
- `Json_Number$(num$)` - Create JSON number value
- Additional JSON builders

**Example:**
```basic
PRINT Json_Object3$("name", Json_String$("QBNex"), "version", Json_String$("1.0.0"), "status", Json_String$("ok"))
' Output: {"name":"QBNex","version":"1.0.0","status":"ok"}
```

### System Libraries

#### Environment (`sys.env`)
Platform detection and environment variables.

**Functions:**
- `Env_Platform$` - Get platform name (Windows/Linux/macOS)
- `Env_Is64Bit&` - Check if 64-bit platform
- `Env_GetHome$` - Get home directory
- Additional environment functions

**Example:**
```basic
PRINT "Platform: "; Env_Platform$
PRINT "64-bit: "; Env_Is64Bit&
PRINT "Home: "; Env_GetHome$
' Output (Linux):
' Platform: Linux
' 64-bit: 1
' Home: /home/user
```

#### Arguments (`sys.args`)
Command-line argument access.

**Functions:**
- `Args_Count&` - Get argument count
- `Args_Get$(index)` - Get argument at index

**Example:**
```basic
PRINT "Argument count: "; Args_Count&
FOR i = 0 TO Args_Count& - 1
    PRINT "Arg "; i; ": "; Args_Get$(i)
NEXT i
```

#### DateTime (`sys.datetime`)
Date and time utilities.

**Functions:**
- `Date_SetNow(date)` - Set to current time
- `Date_ToISOString$(date)` - Convert to ISO 8601 string
- `Date_GetFullYear&(date)` - Get year
- `Date_GetMonth&(date)` - Get month
- `Date_GetDay&(date)` - Get day
- `Date_NowMs#` - Get current timestamp in milliseconds

**Example:**
```basic
DIM now AS QBNex_Date
Date_SetNow now

PRINT "ISO: "; Date_ToISOString$(now)
PRINT "Year: "; Date_GetFullYear&(now)
PRINT "Timestamp: "; Date_NowMs#
' Output:
' ISO: 2026-04-13T19:30:00Z
' Year: 2026
' Timestamp: 1744564200000
```

### Math & Error Handling

#### Numeric Utilities (`math.numeric`)
Mathematical helper functions.

**Functions:**
- `Math_Clamp#(value#, min#, max#)` - Clamp value to range
- Additional numeric utilities

**Example:**
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
```basic
DIM pet AS Dog
New_Dog pet, "Buddy", 3, "Collie"

PRINT "Name: "; pet.Name
PRINT "Describe: "; pet.Describe$
PRINT "Sound: "; pet.Bark$
```

**Interfaces:**
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
```basic
DIM pet AS Dog
New_Dog pet, "Buddy", 3, "Collie"

PRINT "Class: "; QBNEX_ObjectClassName$(pet.Header)
PRINT "Is Animal: "; QBNEX_ObjectIs&(pet.Header, "Animal")
PRINT "Is Dog: "; QBNEX_ObjectIs&(pet.Header, "Dog")
PRINT "Has IPet: "; QBNEX_Implements&(pet.Header.ClassID, "IPet")
```

---

## Testing & Verification

QBNex includes comprehensive test suites to verify compiler and library functionality.

### Comprehensive Test Suite

**Main Test Suite** (`test_all.bas`):
```bash
# Compile and run all tests
qb test_all.bas -o test_all.exe
test_all.exe

# View test summary
type test_summary.txt    # Windows
cat test_summary.txt     # Linux/macOS

# Or use the test runner script (Windows)
run_tests.cmd
```

The comprehensive test suite includes:
- **17 test categories** covering all major features
- **15 core language tests**: Runtime paths, variables, math, strings, control flow, loops, arrays, types, functions, date/time, file I/O, SELECT CASE, type conversion, logical operations
- **18 stdlib module compilation tests**: All standard library modules
- **9 example program tests**: All example programs compile and run correctly
- **100% success rate**: All tests pass with zero errors and zero warnings

### Running Individual Tests

**Smoke Tests:**
```bash
# Import smoke test (verifies import system)
qb source/stdlib/examples/import_smoke.bas -x

# Runtime smoke test (verifies stdlib functions)
qb source/stdlib/examples/runtime_smoke.bas -x

# Data smoke test (verifies data structures)
qb source/stdlib/examples/data_smoke.bas -z

# Ecosystem smoke test (verifies integration)
qb source/stdlib/examples/ecosystem_smoke.bas -z
```

**Diagnostics Smoke Test (Windows):**
```cmd
tests\diagnostics_smoke.cmd
```

**Diagnostics Smoke Test (Linux/macOS):**
```bash
chmod +x tests/diagnostics_smoke.sh
./tests/diagnostics_smoke.sh
```

**Compiler Smoke Suite (Windows):**
```cmd
set QBNEX_CI=1
set QBNEX_BOOTSTRAP=1
setup_win.cmd
tests\diagnostics_smoke.cmd
tests\warnings_smoke.cmd
tests\labels_smoke.cmd
tests\encoding_smoke.cmd
tests\cli_smoke.cmd
```

Current CLI smoke coverage includes:
- `--help`, `--version`, unknown switch, invalid output path
- quiet mode (`-q`)
- settings output (`-s`)
- warnings-as-errors
- console-mode CLI flag (`-x`)
- C-generation mode (`-z`)

This script validates both behaviors in one run:
- Default compilation output includes detailed markers such as `[!] cause` and `[+] example` without requiring `-d`
- `--compact-errors` hides those detailed sections and keeps output compact
- Source fixture for the test is `tests/fixtures/diagnostics_compile_error.bas`

**Regression Tests:**
```bash
# Top-level runtime regression
qb source/stdlib/examples/top_level_runtime_regression.bas -z

# Method chain regression
qb source/stdlib/examples/method_chain_regression.bas -z

# Top-level QBNex runtime (minimal)
qb source/stdlib/examples/top_level_qbnex_runtime_min.bas -z
```

**Demo Programs:**
```bash
# Full stdlib demonstration
qb source/stdlib/examples/stdlib_demo.bas -z

# Class syntax examples
qb source/stdlib/examples/class_syntax_demo.bas -z
```

### Library Compilation Status

As of version 1.0.0, the compilation status is:

✅ **Successfully Compiling (100%)**
- Collections: List, Stack, Queue, Set, Dictionary (5/5)
- Strings: StringBuilder, Text (2/2)
- I/O: CSV, JSON, Path (3/3)
- System: Args, DateTime, Env (3/3)
- Math: Numeric (1/1)
- Error: Result (1/1)
- OOP: Class, Interface (2/2)
- Core: qbnex_stdlib.bas (1/1)
- Examples: All test files (9/9)

**Total: 27/27 modules (100%)**

### Test Results Summary

```
QBNex Test Suite Results
========================

Total Tests:  17
Passed:  17  (100%)
Failed:  0

Status: ALL TESTS PASSED

QBNex Compiler Version: 1.0.0
```

### Verification Checklist

Use this checklist to verify your QBNex installation:

- [x] **Compiler executable exists**: `qb --version` shows 1.0.0
- [x] **Help works**: `qb --help` displays usage
- [x] **Basic compilation**: `qb hello.bas` creates executable
- [x] **Comprehensive test suite**: `test_all.bas` passes all 17 tests
- [x] **Import system**: `import_smoke.bas` compiles successfully
- [x] **Collections**: All collection libraries compile with `-z` flag
- [x] **String utilities**: Text and StringBuilder compile
- [x] **I/O libraries**: CSV, JSON, Path libraries compile
- [x] **System libraries**: Env, Args, DateTime compile
- [x] **OOP support**: Class and Interface modules compile
- [x] **Example programs**: All 9 example programs compile
- [ ] **Docker (optional)**: Docker image builds successfully

### Test Documentation

For detailed testing information, see:
- `TEST_README.md` - Comprehensive testing guide
- `TEST_RESULTS.md` - Detailed test execution report
- `test_summary.txt` - Quick test results summary (generated after running tests)

### Creating Your Own Tests

**Simple Test Program:**
```basic
' test_basic.bas
DIM failures AS LONG

' Test arithmetic
IF 2 + 2 <> 4 THEN failures = failures + 1
IF 10 - 5 <> 5 THEN failures = failures + 1
IF 3 * 4 <> 12 THEN failures = failures + 1
IF 20 / 4 <> 5 THEN failures = failures + 1

' Test strings
IF LEN("Hello") <> 5 THEN failures = failures + 1
IF LCASE$("HELLO") <> "hello" THEN failures = failures + 1

' Report results
IF failures = 0 THEN
    PRINT "ALL_TESTS_PASSED"
    SYSTEM 0
ELSE
    PRINT "TEST_FAILURES: "; failures
    SYSTEM 1
END IF
```

**Library Test:**
```basic
' test_collections.bas
'$IMPORT:'qbnex'

DIM failures AS LONG

' Test List
DIM myList AS QBNex_List
List_Init myList
List_Add myList, "item1"
List_Add myList, "item2"

IF List_Count&(myList) <> 2 THEN failures = failures + 1
IF List_Get$(myList, 0) <> "item1" THEN failures = failures + 1

List_Free myList

' Test Stack
DIM stack AS QBNex_Stack
Stack_Init stack
Stack_Push stack, "first"
Stack_Push stack, "second"

IF Stack_Count&(stack) <> 2 THEN failures = failures + 1
IF Stack_Peek$(stack) <> "second" THEN failures = failures + 1

Stack_Free stack

' Report
IF failures = 0 THEN
    PRINT "COLLECTION_TESTS_PASSED"
ELSE
    PRINT "COLLECTION_TEST_FAILURES: "; failures
END IF
```

Run test:
```bash
qb test_collections.bas -x
```

### Continuous Integration

QBNex uses GitHub Actions for automated testing:

```yaml
name: QBNex Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup QBNex
        run: |
          chmod +x setup_lnx.sh
          ./setup_lnx.sh
      
      - name: Run smoke tests
        run: |
          qb source/stdlib/examples/import_smoke.bas -z
          qb source/stdlib/examples/runtime_smoke.bas -z
          qb source/stdlib/examples/stdlib_demo.bas -z
      
      - name: Verify all libraries
        run: |
          for f in source/stdlib/collections/*.bas; do
            qb "$f" -z || exit 1
          done
```

### Test Output Interpretation

**Successful Compilation:**
```
  QQQQ    BBBB    N   N   EEEEE   X   X  
 Q    Q   B   B   NN  N   E        X X   
 Q  QQ    BBBB    N N N   EEEE      X    
 Q   Q    B   B   N  NN   E        X X   
  QQQQ    BBBB    N   N   EEEEE   X   X  

QBNex Compiler

Preparing build files... [########################################] 100%
```

**Compilation Error:**
```
[x] QBNex :: Error [E1203]  Expression mixes incompatible types
  [@] source/qbnex.bas(1708,28)
  [#] source
    1708 | IF UserDefine(0, i) = l$ THEN
         |                            ^ string value compared against incompatible type
  [>] next     Convert the value to the correct type using an explicit conversion function.
  [::] flow    Parsing :: parse source file -> file: source/qbnex.bas
```

**Detailed Diagnostics (default):**
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

### Diagnostic Output Style

QBNex diagnostics use compact markers so the important parts of an error can be scanned quickly in a plain terminal:

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
| `[^]` | Verbose location context |
| `[.]` | Verbose detail note |
| `[>>]` | Verbose in-progress context |
| `[!]` | Verbose cause note |
| `[+]` | Verbose example or corrected form |

Detailed diagnostics are enabled by default so the full trail, root-cause notes, and remediation examples appear automatically. Use `-k` or `--compact-errors` when you prefer compact output.

**Runtime Success:**
```
RUNTIME_SMOKE_OK
```

**Runtime Failure:**
```
RUNTIME_SMOKE_FAIL 3
(Exit code: 1)
```

---

## Compilation Pipeline

Understanding how QBNex translates BASIC code to native binaries.

### How It Works

QBNex is a self-hosting compiler written in QBNex BASIC itself (~26,000 lines). The compilation process involves multiple stages:

```
┌─────────────────┐
│  BASIC Source   │  (yourfile.bas)
│    (.bas file)  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   Pre-pass      │  Handle $INCLUDE, $DEFINE, $IFDEF
│   (Parsing)     │  Validate syntax, process imports
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Code Generation│  Translate BASIC to C++ code
│   (Transpiler)  │  Generate optimized C++ in internal/temp/
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  C++ Compiler   │  Invoke g++ (Windows/Linux) or clang++ (macOS)
│   (Native)      │  Link against OpenGL, audio, etc.
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Native Binary  │  yourfile.exe (Windows)
│   (Executable)  │  ./yourfile (Linux/macOS)
└─────────────────┘
```

### Stage 1: Pre-pass (Parsing)

During the pre-pass stage, the compiler:

1. **Reads source file**: Loads your `.bas` file into memory
2. **Processes directives**:
   - `$INCLUDE`: Includes external files
   - `$DEFINE`/`$IFDEF`/`$ENDIF`: Conditional compilation
   - `$IMPORT:`: Loads standard library modules
3. **Validates syntax**: Checks for syntax errors
4. **Builds symbol table**: Records variables, functions, types
5. **Handles metacommands**: Processes `OPTION _EXPLICIT`, etc.

**Example:**
```basic
' This is processed during pre-pass
'$IMPORT:'qbnex'
'$INCLUDE:'mylib.bas'

$DEFINE DEBUG_MODE
$IFDEF DEBUG_MODE
    PRINT "Debug mode enabled"
$ENDIF
```

### Stage 2: Code Generation

The code generation stage:

1. **Translates BASIC to C++**: Each BASIC statement becomes C++ code
2. **Optimizes output**: Removes redundant code, inlines functions
3. **Generates runtime calls**: Links to LibQB runtime library
4. **Handles types**: Converts BASIC types to C++ equivalents
5. **Creates temp files**: Outputs to `internal/temp/`

**BASIC to C++ Translation:**
```basic
' BASIC Input
PRINT "Hello, World!"
FOR i = 1 TO 10
    PRINT i
NEXT i
```

Becomes (simplified):
```cpp
// Generated C++ Output
print_string("Hello, World!\n");
for (int i = 1; i <= 10; i++) {
    print_integer(i);
    print_string("\n");
}
```

### Stage 3: C++ Compilation

The final stage:

1. **Invokes C++ compiler**:
   - Windows: `g++` from MinGW
   - Linux: `g++` from GCC
   - macOS: `clang++` from Xcode
2. **Links libraries**:
   - OpenGL, FreeGLUT, GLEW (graphics)
   - miniaudio (sound)
   - FreeType (fonts)
   - Platform-specific libraries
3. **Produces native binary**: Platform-specific executable
4. **Cleans up**: Removes temporary files (unless `-z` flag used)

**Compilation Command (Windows):**
```bash
g++ -mconsole -s -Wfatal-errors -w -Wall \
  qbx.cpp \
  libqb\os\win\libqb_setup.o \
  parts\video\font\ttf\os\win\src.o \
  parts\core\os\win\src.a \
  -lopengl32 -lglu32 -static-libgcc -static-libstdc++ \
  -D GLEW_STATIC -D FREEGLUT_STATIC \
  -lws2_32 -lwinmm -lgdi32 \
  -o "output.exe"
```

### Generated Files

When you compile with `-z` flag (generate C code only), you can inspect the output:

```bash
qb myprogram.bas -z
```

Generated files appear in `internal/temp/`:
- `qbx.cpp` - Main C++ source file
- `*.h` - Header files
- Resource files (icons, etc.)

**View generated C++ code:**
```bash
# Windows
type internal\temp\qbx.cpp | more

# Linux/macOS
less internal/temp/qbx.cpp
```

### Compilation Flags Deep Dive

**Flag `-z` (Generate C code only):**
```bash
qb myprogram.bas -z
# Creates internal/temp/qbx.cpp
# Does NOT compile to executable
# Useful for: debugging, inspection, custom builds
```

**Flag `-x` (Compile and run):**
```bash
qb myprogram.bas -x
# Compiles to myprogram.exe
# Immediately executes myprogram.exe
# Shows program output in console
```

**Flag `-o` (Custom output name):**
```bash
qb myprogram.bas -o myapp.exe
# Creates myapp.exe instead of myprogram.exe
# Useful for: deployment, versioning
```

**Flag `-w` (Show warnings):**
```bash
qb myprogram.bas -w
# Shows potential issues that aren't errors
# Recommended for development
```

**Flag `-e` (OPTION _EXPLICIT):**
```bash
qb myprogram.bas -e
# Requires all variables to be declared with DIM
# Catches typos in variable names
# Best practice for production code
```

### Performance Considerations

**Compilation Speed:**
- Simple programs: 1-3 seconds
- Medium programs: 3-10 seconds
- Large programs (1000+ lines): 10-30 seconds
- Compiler itself (~26K lines): 1-2 minutes

**Binary Size:**
- Hello World: ~1-2 MB (includes runtime)
- Graphics program: ~3-5 MB (includes OpenGL libs)
- Full stdlib program: ~2-4 MB

**Optimization Tips:**
1. Use `-q` flag for cleaner output during development
2. Use `-w` flag to catch potential issues early
3. Use `-e` flag to enforce variable declaration
4. Use `-z` flag for debugging compilation issues

---

## Troubleshooting Comprehensive Guide

Common issues and their solutions.

### Compilation Issues

**Problem: "Cannot convert number to string"**
```
Cannot convert number to string
Caused by: IF USERDEFINE ( 0 , I ) = L$ THEN
LINE 1708
```

**Solution:**
- Check type mismatches in comparisons
- Ensure proper type conversion with `STR$()`, `VAL()`
- Verify variable types match in operations
- Known issue in main compiler self-hosting (line 1708)

**Problem: "File not found"**
```
Error: File not found: myprogram.bas
```

**Solution:**
```bash
# Check file exists
ls -la myprogram.bas  # Linux/macOS
dir myprogram.bas     # Windows

# Use absolute path
qb /path/to/myprogram.bas

# Check current directory
pwd   # Linux/macOS
cd    # Windows
```

**Problem: "Permission denied"**
```
bash: ./myprogram: Permission denied
```

**Solution:**
```bash
# Make executable
chmod +x myprogram

# Or run with explicit interpreter
./myprogram
```

### Runtime Issues

**Problem: Program compiles but doesn't run**

**Solution:**
```bash
# Check if executable exists
ls -la myprogram

# Check dependencies (Linux)
ldd myprogram

# Run with strace (Linux debugging)
strace ./myprogram

# Check for missing libraries
./myprogram
# Error message will show missing library
```

**Problem: Graphics not displaying**

**Solution:**
```basic
' Ensure proper SCREEN mode
SCREEN 12  ' 640x480 graphics mode

' Check if graphics initialized
CLS  ' Clear screen

' Add delay to see output
SLEEP 2
```

**Docker graphics:**
```bash
# Enable X11 forwarding
xhost +local:docker
docker run --rm -e DISPLAY=$DISPLAY -v /tmp/.X11-unix:/tmp/.X11-unix qbnex qb graphics.bas -x
```

**Problem: Sound not working**

**Solution:**
- Linux: Check ALSA drivers installed (`libasound2-dev`)
- macOS: CoreAudio should work automatically
- Windows: Check Windows Multimedia library
- Use `BEEP` for simple test

```basic
BEEP  ' Should produce system beep
SOUND 440, 18  ' 440Hz for 1 second
```

### Docker Issues

**Problem: "Volume mount not working"**

**Solution:**
```bash
# Use absolute paths
docker run --rm -v /absolute/path:/project qbnex qb file.bas

# Check Docker Compose config
docker-compose config

# Verify files mounted
docker run --rm -v $(pwd):/project qbnex ls -la /project
```

**Problem: "Graphics not working in Docker"**

**Solution:**
```bash
# Linux: Allow Docker X11 access
xhost +local:docker

# macOS: Install XQuartz
brew install --cask xquartz
# Enable "Allow connections from network clients" in XQuartz preferences

# Windows: Install VcXsrv
# Run with "Disable access control" checked
```

**Problem: "Build fails in Docker"**

**Solution:**
```bash
# Rebuild without cache
docker-compose build --no-cache

# Check Dockerfile syntax
docker build --no-cache -t qbnex .

# Verify all files present
ls -la
# Should include: Dockerfile, docker-compose.yml, source/
```

### Standard Library Issues

**Problem: "Import not working"**

**Solution:**
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

**Problem: "Module not found"**

**Solution:**
- Modules are in `source/stdlib/` relative to compiler root
- Ensure compiler installation is complete
- Check file exists: `source/stdlib/collections/list.bas`

```bash
# Verify stdlib files exist
ls -la source/stdlib/collections/
ls -la source/stdlib/strings/
ls -la source/stdlib/sys/
```

**Problem: "QBNex_List not defined"**

**Solution:**
```basic
' Must import qbnex core first
'$IMPORT:'qbnex'

' Then you can use QBNex types
DIM myList AS QBNex_List
List_Init myList
```

### Platform-Specific Issues

**Windows:**

**Problem: "g++ not found"**
- Run `setup_win.cmd` to download MinGW
- Check antivirus isn't blocking download
- Whitelist QBNex folder in antivirus

**Problem: "Antivirus flags qb.exe"**
- Add QBNex folder to exclusion list
- This is a false positive (compiled binary heuristic)
- QBNex is open source (MIT License)

**Linux:**

**Problem: "Missing libraries"**
```bash
# Install dependencies (Debian/Ubuntu)
sudo apt-get install build-essential libglu1-mesa-dev libasound2-dev freeglut3-dev libx11-dev

# Install dependencies (Fedora)
sudo dnf install gcc-c++ libglu-devel libasound-devel freeglut-devel libX11-devel

# Install dependencies (Arch)
sudo pacman -S gcc glu alsa-lib freeglut libx11
```

**macOS:**

**Problem: "xcode-select not found"**
```bash
# Install Xcode Command Line Tools
xcode-select --install

# Verify installation
gcc --version
clang --version
```

### Getting Help

**Check compiler version:**
```bash
qb --version
# Should show: QBNex Compiler 1.0.0
```

**Show help:**
```bash
qb --help
# Shows all available flags and usage
```

**Show examples:**
```bash
qb --examples
# Shows common usage patterns
```

**Check settings:**
```bash
qb -s
# Shows current compiler settings
```

**Verbose compilation:**
```bash
# Compile with warnings
qb myprogram.bas -w

# Compile in quiet mode (less output)
qb myprogram.bas -q
```

**Generate C code for inspection:**
```bash
qb myprogram.bas -z
# Check internal/temp/qbx.cpp for generated code
```

### Known Issues

1. **Main compiler self-hosting** (line 1708)
   - Type conversion issue in `UserDefine()` function
   - Pre-compiled `qb.exe` works fine
   - Affects recompiling compiler from source
   - Workaround: Use existing `qb.exe`

2. **utilities/config.bas compilation**
   - Minor syntax issue preventing compilation
   - Low impact (configuration utility)
   - Use `internal/config.ini` directly instead

3. **Executable timeout in automated tests**
   - Console I/O in certain environments
   - Manual execution works correctly
   - Use `-x` flag for immediate testing

### Reporting Bugs

When reporting bugs, include:

1. **Platform**: Windows/Linux/macOS version
2. **Compiler version**: `qb --version`
3. **Source code**: Minimal reproducing example
4. **Expected behavior**: What you expected
5. **Actual behavior**: What happened
6. **Error messages**: Complete error output
7. **Steps to reproduce**: How to trigger the bug

**Create minimal bug report:**
```basic
' bug_demo.bas
' Expected: Should print "Hello"
' Actual: Prints nothing or error

PRINT "Hello"  ' This line causes issue
```

```bash
qb bug_demo.bas -w
# Copy complete output
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

### Data Types

QBNex supports a comprehensive range of data types:

```basic
' types_demo.bas
DIM bitVar AS BIT
DIM byteVar AS BYTE
DIM intVar AS INTEGER
DIM longVar AS LONG
DIM int64Var AS _INTEGER64
DIM singleVar AS SINGLE
DIM doubleVar AS DOUBLE
DIM floatVar AS _FLOAT
DIM stringVar AS STRING * 20
DIM offsetVar AS OFFSET

' Unsigned types
DIM uByte AS UNSIGNED BYTE
DIM uInt AS UNSIGNED INTEGER
DIM uLong AS UNSIGNED LONG
DIM uInt64 AS UNSIGNED _INTEGER64

' Type conversion
DIM num AS INTEGER
num = CINT("123")
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

### Networking Example

QBNex supports TCP/IP networking for server and client applications:

```basic
' network_server.bas
' Simple TCP server example
DIM serverHandle AS LONG
DIM clientHandle AS LONG
DIM message AS STRING

' Create server socket on port 8080
serverHandle = _OPENHOST("TCP/IP:8080")
IF serverHandle = 0 THEN
    PRINT "Failed to create server socket"
    END
END IF

PRINT "Server listening on port 8080..."
PRINT "Waiting for connection..."

' Wait for client connection
DO
    clientHandle = _OPENCONNECTION(serverHandle)
    IF clientHandle > 0 THEN EXIT DO
    SLEEP 1
LOOP

PRINT "Client connected!"

' Receive and display message
DO
    message = INPUT$(1024, clientHandle)
    IF LEN(message) > 0 THEN
        PRINT "Received: "; message
    END IF
    SLEEP 1
LOOP UNTIL message = "QUIT"

PRINT "Client disconnected"
_CLOSECONNECTION clientHandle
_CLOSECONNECTION serverHandle
```

```basic
' network_client.bas
' Simple TCP client example
DIM clientHandle AS LONG

' Connect to server
clientHandle = _OPENCLIENT("TCP/IP:8080:localhost")
IF clientHandle = 0 THEN
    PRINT "Failed to connect to server"
    END
END IF

PRINT "Connected to server"

' Send message
PRINT #clientHandle, "Hello from client!"
PRINT "Message sent"

' Close connection
_CLOSECONNECTION clientHandle
PRINT "Connection closed"
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

Primary compiler modules now live under `source/utilities/` with `source/qbnex.bas`
acting mainly as the main orchestrator. Shared startup/build/temp-workspace state is
grouped in `source/utilities/state.bas`.

For rebuild steps, minimum verification, and module-splitting guidance, see
[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

### How It Works

QBNex is a self-hosting compiler written in QBNex BASIC itself (~26,000 lines). The compilation process:

1. **Bootstrap Phase**: A minimal C++ compiler (`internal/c/qbx.cpp`) compiles the bootstrap data in `internal/source/` into a stage0 compiler
2. **Self-Hosting Phase**: The build generates a stage0-compatible source from `source/qbnex.bas` and its modules, then the stage0 compiler recompiles that source into the final `qb` / `qb.exe`
3. **Code Generation**: The QBNex compiler translates BASIC source code into optimized C++ code
4. **Native Compilation**: Platform C++ compiler (g++/clang++) compiles the generated C++ to native binary
5. **Runtime Linking**: Generated code links against OpenGL, miniaudio, FreeType, and platform libraries

### Project Structure

```
QBNex/
├── source/                      # Compiler source code (QBNex BASIC)
│   ├── qbnex.bas               # Main compiler (~26,000 lines)
│   ├── global/                 # Version, constants, settings
│   │   ├── version.bas        # Version 1.0.0
│   │   ├── constants.bas      # ASCII codes, key codes
│   │   └── compiler_settings.bas # INI-based configuration
│   ├── subs_functions/         # Built-in functions and subroutines
│   │   └── extensions/opengl/  # OpenGL extension definitions
│   └── utilities/              # Helper modules
├── internal/                   # Internal build files
│   ├── c/                      # C++ runtime library
│   │   ├── qbx.cpp            # C++ compiler entry point
│   │   ├── libqb/             # Platform-specific runtime (win/lnx/osx)
│   │   └── parts/             # Feature modules
│   │       ├── core/          # OpenGL, FreeGLUT, GLEW
│   │       ├── audio/         # miniaudio library
│   │       ├── video/         # FreeType, STB Image
│   │       ├── network/       # Socket implementation
│   │       └── input/         # Game controller support
│   └── source/                 # Bootstrap data files for the stage0 compiler
├── .github/workflows/          # GitHub Actions CI/CD
├── assets/                     # Logo and icons
├── licenses/                   # License files
└── setup_*.cmd/sh             # Platform setup scripts
```

### Building from Source

**Windows:**
```cmd
git clone https://github.com/thirawat27/QBNex.git
cd QBNex
setup_win.cmd
```

**Linux:**
```bash
git clone https://github.com/thirawat27/QBNex.git
cd QBNex
chmod +x setup_lnx.sh
./setup_lnx.sh
```

**macOS:**
```bash
git clone https://github.com/thirawat27/QBNex.git
cd QBNex
chmod +x setup_osx.command
./setup_osx.command
```

**Docker:**
```bash
git clone https://github.com/thirawat27/QBNex.git
cd QBNex
docker build -t qbnex .
docker run --rm -v $(pwd):/project qbnex qb source/qbnex.bas -w
```

### Adding New Features

- **Built-in Functions/Subs**: Add to `source/subs_functions/subs_functions.bas`
- **Runtime Library**: Modify C++ code in `internal/c/` and `internal/c/parts/`
- **Graphics Extensions**: Edit `source/subs_functions/extensions/opengl/`
- **Compiler Core**: Main compiler logic in `source/qbnex.bas`

### Continuous Integration

QBNex uses GitHub Actions for automated builds:

- **Push to master**: Linux build
- **Pull requests**: Linux build
- **Releases**: Linux, macOS, Windows x86, Windows x64

CI workflows call the platform `setup_*` scripts directly in CI mode and skip with commit message containing `ci-skip`.

---

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details on our code of conduct and the process for submitting pull requests.

### Ways to Contribute

- **Report bugs** and issues via GitHub Issues
- **Suggest new features** and enhancements
- **Improve documentation** and examples
- **Submit pull requests** with fixes or features
- **Write test cases** for better coverage
- **Optimize performance** of compiler or runtime
- **Help others** by answering questions in discussions

### Code of Conduct

Please note that this project follows a code of conduct. By participating, you are expected to uphold this code and maintain a respectful, inclusive community.

### Security

For security vulnerabilities, please read [SECURITY.md](SECURITY.md) and follow the responsible disclosure process. **Do not** create public GitHub issues for security vulnerabilities.

---

## License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.

The MIT License is a permissive open source license that allows free use, modification, and distribution of the software with minimal restrictions.

---

## Acknowledgments

- **QBasic/QuickBASIC** - The original BASIC implementation by Microsoft that inspired this project
- **QB64** - Modern QBasic compiler that served as the foundation for QBNex
- **FreeGLUT & OpenGL** - Cross-platform window management and graphics rendering
- **miniaudio** - Single-file audio playback library enabling cross-platform sound
- **FreeType** - TrueType font rendering engine
- **STB Image** - Single-header image loading library
- **GLEW** - OpenGL Extension Wrangler for advanced graphics features
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
  <p>Created by <a href="https//github.com/thirawat27">thirawat27</a></p>
</div>
