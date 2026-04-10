# QBNex

![QBNex](assets/QBNex.png)

**QBNex** is a modern extended BASIC programming language that retains QB4.5/QBasic compatibility and compiles native binaries for Windows, Linux, and macOS.

QBNex is a CLI-only compiler derived from QB64, with the IDE component completely removed and architecture significantly improved for better performance, maintainability, and extensibility.

---

[![contributions welcome](https://img.shields.io/badge/contributions-welcome-brightgreen.svg?style=flat)](https://github.com/thirawat27/QBNex/issues)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/version-1.0.0-blue.svg)](https://github.com/thirawat27/QBNex/releases)

## Table of Contents

1. [About QBNex](#about-qbnex)
2. [Features](#features)
3. [Installation](#installation)
   - [Windows](#windows)
   - [macOS](#macos)
   - [Linux](#linux)
4. [Usage](#usage)
5. [Project Structure](#project-structure)
6. [Building from Source](#building-from-source)
7. [Configuration](#configuration)
8. [Contributing](#contributing)
9. [License](#license)
10. [Acknowledgments](#acknowledgments)

---

## About QBNex <a name="about-qbnex"></a>

QBNex is a reimagined version of QB64, transformed into a streamlined CLI-only compiler. The project removes all IDE components, refactors the codebase for modern standards, and introduces architectural improvements to eliminate bottlenecks. 

Key differences from QB64:
- **CLI-Only**: No integrated development environment; focuses purely on compilation
- **Modern Architecture**: Reorganized file structure following industry standards
- **Enhanced Performance**: Optimized algorithms and reduced overhead
- **Clean Codebase**: Removed legacy code, improved comments, and enhanced maintainability
- **Unique Identity**: Custom branding, icons, and output messages distinct from QB64

**Owner**: thirawat27  
**Version**: 1.0.0  
**Year**: 2026  
**Repository**: [https://github.com/thirawat27/QBNex](https://github.com/thirawat27/QBNex)

---

## Features <a name="features"></a>

- Full QB4.5/QBasic syntax compatibility
- Extended BASIC features including OpenGL support
- Cross-platform compilation (Windows, Linux, macOS)
- 32-bit and 64-bit binary generation
- Modern C++ backend runtime
- Command-line interface with intuitive commands
- Configurable compilation options
- Enhanced error reporting and debugging support

---

## Installation <a name="installation"></a>

Download the appropriate package for your operating system from the [Releases page](https://github.com/thirawat27/QBNex/releases).

### Windows <a name="windows"></a>

1. Extract the package to a folder with full write permissions
2. Add the `bin` directory to your system PATH (optional but recommended)
3. Verify installation by running `qb --version`

> **Note**: It is advisable to whitelist the QBNex folder in your antivirus/antimalware software.

### macOS <a name="macos"></a>

1. Install Xcode command line tools:
   ```bash
   xcode-select --install
   ```
2. Run the setup script:
   ```bash
   ./scripts/setup_osx.command
   ```
3. Verify installation: `qb --version`

### Linux <a name="linux"></a>

1. Run the setup script:
   ```bash
   ./scripts/setup_lnx.sh
   ```
2. Dependencies (OpenGL, ALSA, GNU C++ Compiler) will be automatically installed
3. Verify installation: `qb --version`

---

## Usage <a name="usage"></a>

QBNex provides a simple and intuitive CLI for compiling QBasic programs:

### Basic Compilation
```bash
qb main.bas
```

### Compile with Custom Output Name
```bash
qb main.bas -o myprogram.exe
```

### Compile Without Running (Generate Only)
```bash
qb main.bas -c
```

### Compile and Run Immediately
```bash
qb main.bas -r
```

### Show Help
```bash
qb --help
```

### Show Version
```bash
qb --version
```

### Compile Options

| Flag | Description |
|------|-------------|
| `-c` | Compile only, do not execute |
| `-r` | Run after compilation |
| `-o <name>` | Specify output filename |
| `-v` | Verbose output |
| `--debug` | Enable debug mode |
| `--help` | Display help information |
| `--version` | Display version information |

---

## Project Structure <a name="project-structure"></a>

```
QBNex/
├── assets/                 # Branding assets (icons, images)
│   ├── icons/
│   │   ├── linux/
│   │   ├── macos/
│   │   └── windows/
│   ├── QBNex.ico
│   └── QBNex.png
├── bin/                    # Compiled binaries
├── config/                 # Configuration files
│   └── qbnex.ini
├── docs/                   # Documentation
├── include/                # Header files
├── lib/                    # Library files
├── licenses/               # License files
├── scripts/                # Setup and utility scripts
├── src/                    # Source code
│   ├── compiler/           # Compiler implementation
│   ├── core/               # Core runtime and utilities
│   ├── runtime/            # Runtime library (C++ backend)
│   └── utils/              # Utility functions
├── .gitignore
├── README.md
└── setup_*                 # Platform-specific setup scripts
```

---

## Building from Source <a name="building-from-source"></a>

### Prerequisites

- GCC/G++ or compatible C++ compiler
- OpenGL development libraries
- ALSA development libraries (Linux only)

### Build Steps

1. Clone the repository:
   ```bash
   git clone https://github.com/thirawat27/QBNex.git
   cd QBNex
   ```

2. Run the appropriate setup script for your platform:
   - **Windows**: `scripts\\setup_win.cmd`
   - **macOS**: `./scripts/setup_osx.command`
   - **Linux**: `./scripts/setup_lnx.sh`

3. The compiler will be built and placed in the `bin/` directory

---

## Configuration <a name="configuration"></a>

QBNex uses a centralized configuration file located at `config/qbnex.ini`. This file controls:

- Default compilation options
- Include paths
- Library paths
- Output settings
- Debug options

Example configuration:
```ini
[compiler]
default_output = ./bin
optimization_level = 2
verbose = false

[paths]
include = ./include
lib = ./lib
temp = ./tmp

[runtime]
audio_enabled = true
opengl_version = 2.1
```

---

## Contributing <a name="contributing"></a>

We welcome contributions! Please read our [Contributing Guidelines](.github/CONTRIBUTING.md) before submitting pull requests or issues.

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

---

## License <a name="license"></a>

This project is licensed under the MIT License - see the [LICENSE](licenses/LICENSE) file for details.

Copyright © 2026 thirawat27

---

## Acknowledgments <a name="acknowledgments"></a>

- Original QB64 Team for the foundational work
- FreeGLUT library for OpenGL utility functions
- GLEW library for OpenGL extension loading
- MinGW-w64 project for Windows GCC toolchain
- All contributors and the QBasic community

---

**QBNex** - Compiling QBasic to Native Executables Since 2026
