<div align="center">
  <img src="assets/QBNex.png" alt="QBNex Logo" width="256" height="256">
  
  # QBNex
  
  **Modern QBasic/QuickBASIC Compiler**
  
  > A modern extended BASIC+OpenGL language that retains QB4.5/QBasic compatibility and compiles native binaries for Windows, Linux, and macOS.
  
</div>

---

## Table of Contents

- [About](#about)
- [Features](#features)
- [System Requirements](#system-requirements)
- [Installation](#installation)
- [Usage](#usage)
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
- [Acknowledgments](#acknowledgments)

---

## About

**QBNex** is a modern QBasic/QuickBASIC compiler that translates BASIC source code into optimized C++ and compiles to native binaries for Windows, Linux, and macOS. It was significantly refactored from QB64 to act as a sleek, CLI-driven compiler without the legacy IDE components.

**Version**: 1.0.0

The compiler is self-hosting, written in QBNex BASIC itself (~26,000 lines), and supports 150+ QBasic/QB64 keywords. It features comprehensive graphics via OpenGL/FreeGLUT, sound synthesis via miniaudio, TCP/IP networking, and full file I/O operations.

Repository: https://github.com/thirawat27/QBNex

---

## Features

- **Comprehensive Language Support**
  - Full support for classic QBasic/QB4.5 syntax
  - User-defined types (TYPE...END TYPE) with nested structures
  - Subroutines and functions with parameters
  - Multi-dimensional arrays with REDIM PRESERVE
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

### Windows

Extract the package to a folder with full write permissions.

It is advisable to whitelist the QBNex folder in your antivirus or antimalware software.
*(If building from source, run `setup_win.cmd`)*

### macOS

Install the Xcode command line tools first:

```bash
xcode-select --install
```

Run `./setup_osx.command` to compile QBNex for your macOS version.

### Linux

Run `./setup_lnx.sh` to compile QBNex.

Required packages generally include OpenGL, ALSA, and the GNU C++ compiler.

---

## Usage

QBNex runs as a command-line compiler.

### Basic Usage

Use `qb` or `qbnex` as the command name (depending on your setup):

```bash
# Compile to executable
qb yourfile.bas

# Compile with custom output name
qb yourfile.bas -o outputname.exe

# Compile and run immediately
qb yourfile.bas -x

# Generate C code without compiling
qb yourfile.bas -z
```

### Compiler Flags

| Flag | Description |
|------|-------------|
| `-h`, `--help` | Show help information |
| `-v`, `--version` | Show compiler version |
| `-i`, `--info`, `--about` | Show project information |
| `-g`, `--examples` | Show common CLI examples |
| `-c` | Compile the source file (default behavior) |
| `-o <file>` | Write output executable to specified file |
| `-x` | Compile and run immediately |
| `-w` | Show warnings during compilation |
| `-q` | Quiet mode (minimal output) |
| `-m` | Monochrome (no color) output |
| `-e` | Enable OPTION _EXPLICIT for this compilation |
| `-s[:switch=true/false]` | View/edit compiler settings |
| `-p` | Purge all pre-compiled content |
| `-z` | Generate C code without compiling to executable |

### Compiler Settings

QBNex supports configurable compiler settings via the `-s` flag or `internal/config.ini`:

- **SaveExeWithSource**: Include source code in compiled executable
- **IgnoreWarnings**: Suppress warning messages during compilation
- **DebugInfo**: Include GDB debugging information in output

Example:
```bash
# View current settings
qb -s

# Enable debug mode
qb -s:DebugInfo=true

# Disable warnings
qb -s:IgnoreWarnings=true
```

### Compilation Pipeline

1. **Pre-pass**: Reads BASIC source, handles `$INCLUDE` directives, processes `$DEFINE`/`$IFDEF` conditional compilation, validates syntax
2. **Code generation**: Translates QB commands into C++ code in `internal/temp/`
3. **C++ compilation**: Invokes platform-native C++ compiler (g++ on Windows/Linux, clang++ on macOS) to produce final binary
4. The generated C++ links against OpenGL, audio libraries, and other dependencies

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

### How It Works

QBNex is a self-hosting compiler written in QBNex BASIC itself (~26,000 lines). The compilation process:

1. **Bootstrap Phase**: A minimal C++ compiler (`internal/c/qbx.cpp`) compiles the QBNex source
2. **Code Generation**: The QBNex compiler translates BASIC source code into optimized C++ code
3. **Native Compilation**: Platform C++ compiler (g++/clang++) compiles the generated C++ to native binary
4. **Runtime Linking**: Generated code links against OpenGL, miniaudio, FreeType, and platform libraries

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
│   └── source/                 # Bootstrap data files
├── .ci/                        # CI build scripts
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

CI workflows skip with commit message containing `ci-skip`.

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
