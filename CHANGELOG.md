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

#### Language Support
- **Control Flow**: IF/THEN/ELSE, SELECT CASE, FOR/NEXT, DO/LOOP, WHILE/WEND, GOTO, GOSUB
- **Data Types**: BIT, BYTE, INTEGER, LONG, _INTEGER64, SINGLE, DOUBLE, _FLOAT, STRING, OFFSET
- **Variables & Arrays**: DIM, REDIM, REDIM PRESERVE, multi-dimensional arrays, OPTION BASE
- **User-Defined Types**: TYPE...END TYPE with nested structures
- **Subroutines & Functions**: SUB, FUNCTION, DECLARE with parameter support
- **String Operations**: LEFT$, RIGHT$, MID$, INSTR, LCASE$, UCASE$, LTRIM$, RTRIM$, TRIM$, etc.
- **Math Functions**: ABS, SGN, SIN, COS, TAN, ATN, EXP, LOG, SQR, INT, FIX, RND, etc.
- **Type Conversion**: CINT, CLNG, CSNG, CDBL, CSTR, MKI$, MKL$, CVS, CVD, etc.

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

#### Documentation
- Comprehensive README.md with code examples
- Supported commands reference by category
- Installation guides for all platforms
- Usage examples and common patterns

#### Bug Fixes
- Cross-platform compilation issues on Windows x86, x64, Linux, and macOS
- Graphics rendering consistency across different platforms
- Audio output stability and performance
- File I/O operations for sequential, random, and binary access modes

#### Security
- Added input validation for file operations to prevent buffer overflows
- Improved memory safety in pointer operations
- Enhanced network socket security with proper connection handling

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
