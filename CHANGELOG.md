# Changelog

All notable changes to QBNex will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2024-04-11

### Initial Release

#### Core Features
- Complete QBasic/QB4.5 compatible compiler
- Self-hosting compiler written in QBNex BASIC (~26,000 lines)
- Transpiles BASIC source to C++ and compiles to native binaries
- Command-line interface with comprehensive flag support
- Support for 150+ QBasic/QB64 keywords and functions
- TCP/IP networking support with `_OPENHOST`, `_OPENCLIENT`, and connection management
- OpenGL extension support with `_GLRENDER` and `_DISPLAYORDER` metacommands
- TrueType font rendering via FreeType library
- Image format loading support (BMP, PCX, PNG, JPEG, etc.) via STB Image
- Optional ZLIB compression support
- Game controller/gamepad input support
- Compiler settings management via INI configuration (`internal/config.ini`)
- `-z` flag to generate C code without compiling to executable
- `-s` flag to view/edit compiler settings
- `-p` flag to purge all pre-compiled content
- `-e` flag to enable `OPTION _EXPLICIT` for compilation
- Auto-build message support via `internal/version.txt` (Git integration)
- Debug mode with GDB debugging information (`DebugInfo` setting)
- `IgnoreWarnings` and `SaveExeWithSource` compiler settings
- Network socket support with conditional compilation (`DEPENDENCY_SOCKETS`)
- Audio output via miniaudio library with cross-platform support
- Support for unsigned integer types (`UNSIGNED BYTE`, `UNSIGNED INTEGER`, `UNSIGNED LONG`, `UNSIGNED _INTEGER64`)
- Pointer types (`OFFSET` / `UNSIGNED OFFSET`)
- 64-bit integer support (`_INTEGER64`)
- Extended precision floating-point (`_FLOAT`)

#### Standard Library
- **Complete stdlib implementation** with 18 modules covering collections, strings, I/O, system utilities, math, error handling, and OOP
- **Collections Module** (`collections.*`):
  - `list.bas` - Dynamic array with automatic resizing (List_Init, List_Add, List_Get, List_Remove, List_Count, List_Join, List_Free)
  - `stack.bas` - LIFO stack implementation (Stack_Init, Stack_Push, Stack_Pop, Stack_Peek, Stack_Count, Stack_Free)
  - `queue.bas` - FIFO queue implementation (Queue_Init, Queue_Enqueue, Queue_Dequeue, Queue_Peek, Queue_Count, Queue_Free)
  - `set.bas` - Hash-based set with unique values (HashSet_Init, HashSet_Add, HashSet_Contains, HashSet_Remove, HashSet_Count, HashSet_ToString, HashSet_Free)
  - `dictionary.bas` - Key-value store (Dict_Init, Dict_Set, Dict_Get, Dict_Remove, Dict_Count, Dict_HasKey, Dict_Free)
- **String Utilities** (`strings.*`):
  - `text.bas` - String manipulation (Text_PadLeft, Text_PadRight, Text_Repeat, Text_StartsWith, Text_EndsWith, Text_Contains)
  - `strbuilder.bas` - Efficient string concatenation (SB_Init, SB_Append, SB_AppendLine, SB_ToString, SB_Free)
- **I/O Utilities** (`io.*`):
  - `path.bas` - Cross-platform path manipulation (Path_Join, Path_FileName, Path_DirName, Path_Extension, Path_WithoutExtension, Path_Normalize)
  - `csv.bas` - CSV generation (CSV_Row3, CSV_Escape)
  - `json.bas` - JSON object creation and parsing (Json_Object3, Json_String, Json_Number, Json_Array, json_parse, json_get_str)
- **Network & Web Utilities** (`net.*` / Built-in):
  - `http.bas` - HTTP client implementation (get, post, put, delete, fetch)
  - `url.bas` - URL encoding, decoding, and parsing (encode, decode, url_parse)
  - Web Server - Built-in functionality for HTTP servers (server, route_get, route_post, listen)
- **System Utilities** (`sys.*`):
  - `env.bas` - Platform detection and environment variables (Env_Platform, Env_Is64Bit, Env_GetHome, Env_Get)
  - `args.bas` - Command-line argument access (Args_Count, Args_Get)
  - `datetime.bas` - Date/time utilities (Date_SetNow, Date_ToISOString, Date_GetFullYear, Date_GetMonth, Date_GetDay, Date_NowMs, Date_IsLeapYear)
- **Math Utilities** (`math.*`):
  - `numeric.bas` - Mathematical helpers (Math_Clamp, Math_Min, Math_Max)
- **Error Handling** (`error.*`):
  - `result.bas` - Result pattern for error handling (Result_Ok, Result_Fail, Result_FailCode, Result_IsOk, Result_Value, Result_Message, Result_ErrorChain)
- **OOP Support** (`oop.*`):
  - `class.bas` - Class registry and inheritance (QBNEX_RegisterClass, QBNEX_FindClass, QBNEX_RegisterMethod, QBNEX_FindMethodSlot, QBNEX_IsInstance, QBNEX_ObjectInit)
  - `interface.bas` - Interface implementation (QBNEX_RegisterInterface, QBNEX_FindInterface, QBNEX_Implements)
- **Core Library**:
  - `qbnex_stdlib.bas` - Unified stdlib combining all modules (18 modules in one file)
- **Modern Import System**:
  - Clean Python-style module imports (`IMPORT module.name`) alongside traditional syntax (`'$IMPORT:'module.name'`)
- **Example Programs** (9 comprehensive examples):
  - `stdlib_demo.bas` - Full stdlib demonstration
  - `class_syntax_demo.bas` - Native CLASS syntax examples
  - `import_smoke.bas` - Import system verification
  - `runtime_smoke.bas` - Runtime functionality tests
  - `data_smoke.bas` - Data structure tests
  - `ecosystem_smoke.bas` - Integration tests
  - `top_level_runtime_regression.bas` - Runtime regression tests
  - `method_chain_regression.bas` - Method chaining tests
  - `top_level_qbnex_runtime_min.bas` - Minimal runtime tests

#### Language Support
- **Control Flow**: IF/THEN/ELSE, SELECT CASE, FOR/NEXT, DO/LOOP, WHILE/WEND, GOTO, GOSUB
- **Data Types**: BIT, BYTE, INTEGER, LONG, _INTEGER64, SINGLE, DOUBLE, _FLOAT, STRING, OFFSET
- **Variables & Arrays**: DIM, REDIM, REDIM PRESERVE, multi-dimensional arrays, OPTION BASE
- **User-Defined Types**: TYPE...END TYPE with nested structures
- **Subroutines & Functions**: SUB, FUNCTION, DECLARE with parameter support
- **String Operations**: LEFT$, RIGHT$, MID$, INSTR, LCASE$, UCASE$, LTRIM$, RTRIM$, TRIM$, etc.
- **Math Functions**: ABS, SGN, SIN, COS, TAN, ATN, EXP, LOG, SQR, INT, FIX, RND, etc.
- **Type Conversion**: CINT, CLNG, CSNG, CDBL, CSTR, MKI$, MKL$, CVS, CVD, etc.
- **Modern Extended Syntax**:
  - `IMPORT` statement for modular ecosystem (`IMPORT module_name`)
  - Augmented assignment operators (`+=`, `-=`, `*=`, `/=`)
  - Alternative comment syntax (`# comment`)
  - Modern function type definitions (`FUNCTION name AS STRING` instead of `FUNCTION name$`)
  - Shorter function declarations (`FUNC name()`)
  - Lambda-like single-line functions (`DEF name(x) = x*2`)

#### Graphics Capabilities
- OpenGL-based graphics subsystem with FreeGLUT
- SCREEN modes (0-12+) with VGA and hi-res graphics
- Drawing primitives: PSET, PRESET, LINE, CIRCLE, PAINT, DRAW
- Image manipulation: GET/PUT with transfer modes (PSET, AND, OR, XOR)
- Viewport and window management: VIEW, WINDOW, PMAP
- Color and palette control: COLOR, PALETTE, POINT
- Text positioning: CLS, LOCATE, WIDTH
- TrueType font support via FreeType

#### Sound Capabilities
- BEEP, SOUND, PLAY commands
- Cross-platform audio via miniaudio (Windows, ALSA, CoreAudio)
- Music string syntax with tempo and octave control
- Tone generation with frequency and duration control

#### File I/O
- Sequential file access: OPEN, PRINT #, WRITE #, INPUT #, LINE INPUT #
- Random file access: OPEN FOR RANDOM, GET, PUT, FIELD, LSET, RSET
- Binary file access: OPEN FOR BINARY, GET, PUT
- File management: FREEFILE, EOF, LOF, LOC, SEEK
- Directory operations: KILL, NAME...AS, FILES, CHDIR, MKDIR, RMDIR
- Binary load/save: BLOAD, BSAVE

#### System Features
- Timer and date/time functions: TIMER, DATE$, TIME$
- Command-line argument access: COMMAND$
- Environment variables: ENVIRON$
- Memory operations: PEEK, POKE, DEF SEG, VARPTR, VARSEG, SADD
- Process control: SHELL, CHAIN, CALL, CALL ABSOLUTE
- Error handling: ON ERROR GOTO, RESUME, ERR, ERL

#### Build System
- Automated setup scripts for Windows (`setup_win.cmd`), Linux (`setup_lnx.sh`), and macOS (`setup_osx.command`)
- CI/CD pipelines for all platforms via GitHub Actions
- Cross-compilation support for Windows x86 and x64
- Dependency management with conditional compilation
- Build utilities and file management tools
- Refactored from QB64 to CLI-driven compiler without legacy IDE components
- Modernized build system with automated dependency installation
- Improved cross-platform compatibility (Windows, Linux, macOS)
- Enhanced graphics subsystem with OpenGL and FreeGLUT
- Updated sound system to use miniaudio instead of legacy APIs

#### Platform Support
- **Windows**: Windows 7+ (32-bit and 64-bit), MinGW g++ compiler
- **Linux**: OpenGL, ALSA, FreeGLUT, X11, g++ compiler
- **macOS**: Xcode command line tools, clang++, OpenGL, CoreAudio
- **Docker**: Ubuntu 22.04 base container with all dependencies pre-installed

#### Developer Tools
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

#### Error Handling
- Expanded `QBNex_Result` in `error.result` and `qbnex_stdlib.bas` from a minimal success/message container into a structured result type with `Code`, `Context`, `Source`, and `Cause` metadata.
- Added richer result helpers for contextual propagation and inspection: `Result_FailCode`, `Result_FailWithContext`, `Result_AddContext`, `Result_SetSource`, `Result_SetCause`, `Result_Propagate`, `Result_IsError`, `Result_Code`, `Result_Context`, `Result_Source`, `Result_Cause`, `Result_ErrorChain`, `Result_Describe`, and `Result_Expect`.
- Upgraded compiler diagnostics in `source/utils/error_handler.bas` with phase-aware reporting, context-flow tracing, duplicate suppression, and blocking-diagnostic summaries.
- Reworked diagnostic presentation into a QBNex-specific format using markers such as `[x]`, `[!!]`, `[@]`, `[#]`, `[>]`, `[::]`, and `[=]` instead of generic compiler-style notes.
- Added compact, modern diagnostic summaries such as `QBNex :: Build Halted` and `QBNex :: Build Complete` to better distinguish build outcome from individual errors.

#### Testing & Quality Assurance
- **Comprehensive test suite** (`test_all.bas`) with 17 test categories
- **Core language tests** (15 tests): Runtime paths, variables, math, strings, control flow, loops, arrays, types, functions, date/time, file I/O, SELECT CASE, type conversion, logical operations
- **Standard library compilation tests** (18 modules): All stdlib modules compile successfully
- **Example program tests** (9 programs): All example programs compile and run correctly
- **Test automation**: `run_tests.cmd` script for Windows
- **Test documentation**: `TEST_README.md` with comprehensive testing guide
- **Test results**: `TEST_RESULTS.md` with detailed test execution report
- **100% test success rate**: All 17 test categories pass
- **Zero compilation errors**: Clean compilation across all modules
- **Zero warnings**: No compiler warnings in stdlib or test suite

#### Docker Support
- Multi-stage Dockerfile for production builds (~300MB)
- Development Dockerfile with build cache support (~500MB)
- Docker Compose configuration for easy deployment
- Comprehensive `.dockerignore` for optimized build context
- Volume mounting for seamless development workflow
- Graphics support via X11 forwarding (Linux, macOS, Windows)
- Network mode support for TCP/IP applications
- Interactive development mode with bash shell
- Cross-platform compilation without local dependencies
- CI/CD ready Docker configuration
- Merged Docker documentation into main README.md

#### Documentation
- Comprehensive README.md with code examples
- Supported commands reference by category
- Installation guides for all platforms
- Usage examples and common patterns
- Docker usage guide integrated into main README
- Complete Docker documentation with troubleshooting
- Updated Table of Contents with Docker sections
- Standard library reference with function signatures
- Testing documentation with usage examples
- CHANGELOG.md with detailed version history

#### Bug Fixes
- **Fixed missing DIM declarations** for loop variables in stdlib modules (path.bas, file.bas, opengl_methods.bas)
- **Fixed missing return values** in FUNCTION definitions (Date_IsLeapYear, Date_PartValue, Text_Repeat, Text_StartsWith, Text_EndsWith, Text_Contains, Stack_Peek, Stack_Pop, Queue_Peek, Queue_Dequeue)
- **Fixed unreachable code warnings** by converting inline EXIT FUNCTION to multi-line format
- **Fixed GOTO label case sensitivity** issues (errorCleanup → ErrorCleanup in file.bas)
- **Replaced SYSTEM 1 calls** with graceful error handling in stdlib (List_Init, QBNEX_RegisterClass, QBNEX_RegisterInterfaceName, QBNEX_RegisterMethod, QBNEX_RegisterInterface)
- **Fixed missing TYPE declarations** in interface.bas (added QBNex_ClassInfo and related shared variables)
- **Fixed OOP function return values** (QBNEX_FindClass, QBNEX_FindMethodSlot, QBNEX_ClassName, QBNEX_IsInstance, QBNEX_FindInterface, QBNEX_Implements)
- **Cross-platform compilation issues** on Windows x86, x64, Linux, and macOS
- **Graphics rendering consistency** across different platforms
- **Audio output stability** and performance
- **File I/O operations** for sequential, random, and binary access modes

#### Security
- Added input validation for file operations to prevent buffer overflows
- Improved memory safety in pointer operations
- Enhanced network socket security with proper connection handling
- Graceful error handling instead of abrupt program termination

---

## Notes

### Versioning Scheme
- **Major**: Significant breaking changes or architectural shifts
- **Minor**: New features, backward-compatible enhancements
- **Patch**: Bug fixes, performance improvements, documentation updates

### Development Channels
- Stable releases use standard versioning (e.g., `1.0.0`)
- Development builds may include auto-build messages from Git commits
- Version information stored in `source/global/version.bas`

### Compiler Settings
Configuration is managed via `internal/config.ini` with the following options:
- `SaveExeWithSource`: Include source code in compiled executable
- `IgnoreWarnings`: Suppress warning messages during compilation
- `DebugInfo`: Include GDB debugging information in output

### Dependencies
Core dependencies vary by platform:
- **Common**: OpenGL, FreeGLUT, GLEW
- **Linux**: ALSA (`libasound2-dev`), X11, `g++`
- **macOS**: Xcode CLT, CoreAudio, Apple GLUT, Cocoa
- **Windows**: MinGW (automatically downloaded), Windows Multimedia

Optional dependencies:
- FreeType (TrueType fonts)
- ZLIB (compression)
- Socket libraries (networking)

---

[1.0.0]: https://github.com/thirawat27/QBNex/releases/tag/v1.0.0
