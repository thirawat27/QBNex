# Contributing to QBNex

Thank you for your interest in contributing to QBNex! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [How to Contribute](#how-to-contribute)
  - [Reporting Bugs](#reporting-bugs)
  - [Suggesting Features](#suggesting-features)
  - [Code Contributions](#code-contributions)
  - [Documentation](#documentation)
- [Development Setup](#development-setup)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Submitting Changes](#submitting-changes)
- [Review Process](#review-process)
- [Project Structure](#project-structure)

## Code of Conduct

This project and everyone participating in it is governed by our commitment to providing a welcoming and inclusive environment. By participating, you are expected to uphold this code. Please be respectful, constructive, and professional in all interactions.

## Getting Started

### Prerequisites

Before contributing, familiarize yourself with:

- **QBasic/QuickBASIC**: Understanding of classic BASIC syntax
- **Rust**: the runner `qb-exec` and the runtime crates (`crates/`)
- **QIR**: the text intermediate language the compiler emits (`docs/dev/qir.md`)
- **Cross-platform development**: QBNex targets Windows, Linux, and macOS

### System Requirements

**All platforms:** Git and the Rust toolchain (`cargo`, from https://rustup.rs). No C/C++ compiler is needed.

**For Linux:**
- ALSA development libraries (`libasound2-dev`)
- udev development libraries (`libudev-dev`) and `pkg-config`

**For macOS:**
- Nothing beyond Rust

## How to Contribute

### Reporting Bugs

We use GitHub Issues to track bugs. Before creating a bug report:

1. **Check existing issues** to avoid duplicates
2. **Reproduce the issue** with a minimal code example
3. **Gather information**:
   - QBNex version (`qb --version` or `qbnex --version`)
   - Operating system and version
   - Exact commands used
   - Source code that triggers the issue
   - Expected vs. actual behavior
   - Error messages and screenshots

**Submit a bug report** using the bug report template:
1. Go to the Issues tab
2. Click "New Issue"
3. Select "Bug Report"
4. Fill in the template with details

### Suggesting Features

Feature suggestions help QBNex grow. When suggesting features:

1. **Check existing issues** for similar requests
2. **Explain the use case** clearly
3. **Provide examples** if possible
4. **Consider scope** and implementation complexity

**Submit a feature request** using the feature request template:
1. Go to the Issues tab
2. Click "New Issue"
3. Select "Feature Request"
4. Describe the feature and its benefits

### Code Contributions

QBNex is primarily written in **QBNex BASIC** itself (self-hosting compiler). The codebase structure:

- **`compiler/`**: Compiler source code (written in QBNex BASIC)
  - `qbnex.bas`: Main compiler entry point
  - `global/`: Version, constants, and settings
  - `subs_functions/`: Built-in functions and subroutines
  - `utilities/`: Helper modules
- **`crates/`**: Native runtime (Cargo workspace)
  - `qb-runtime`: runtime library (window, graphics, sound, files, network)
  - `qb-ir`: QIR parser, validator and builtin table
  - `qb-audio`: audio engine
  - `qb-exec`: runner for compiled QIR programs
- **`bootstrap/qbnex.qir`**: the compiler in QIR, used by the setup scripts
- **`tests/`**: `smoke/` scripts, `fixtures/`, `conformance/` corpus and runner
- **`.github/workflows/`**: CI/CD pipelines
- **`.ci/`**: Build automation scripts

#### Finding Work

Good starting points:
- Issues labeled `good first issue`
- Issues labeled `help wanted`
- Documentation improvements
- Test case additions
- Bug fixes

#### Making Changes

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR-USERNAME/QBNex.git
   cd QBNex
   ```
3. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```
4. **Make your changes** following the coding standards below
5. **Test your changes** thoroughly
6. **Commit with clear messages** (see below)
7. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```
8. **Submit a Pull Request**

### Documentation

Documentation improvements are always welcome:

- **README.md**: Usage examples, clarifications, corrections
- **CHANGELOG.md**: Keeping version history current
- **Code comments**: Improving clarity in complex sections
- **Wiki pages**: Tutorials, guides, FAQs (if enabled)
- **Issue templates**: Making them more helpful

## Development Setup

### Windows

```cmd
git clone https://github.com/thirawat27/QBNex.git
cd QBNex
scripts\setup_win.cmd
```

The setup script builds the runner `qb-exec` with cargo, packs `bootstrap/qbnex.qir`
into it as a stage-0 compiler, and lets that compile `qb.exe` in the repository root
(run it as `qb` or `qbnex`). Linux and macOS work the same way.

### Linux

```bash
git clone https://github.com/thirawat27/QBNex.git
cd QBNex
chmod +x scripts/setup_lnx.sh
./scripts/setup_lnx.sh
```

### macOS

```bash
git clone https://github.com/thirawat27/QBNex.git
cd QBNex
chmod +x scripts/setup_osx.command
./scripts/setup_osx.command
```

### Manual Build (Advanced)

```bash
cargo build --release -p qb-exec
# pack bootstrap/qbnex.qir into the runner (scripts/qir_pack.ps1 on Windows)
# and let it compile compiler/qbnex.bas:
./qb-stage0 compiler/qbnex.bas -o qb
sh scripts/selfhost_fixpoint.sh   # gen2 == gen3 == bootstrap/qbnex.qir
```

After changing the compiler, refresh `bootstrap/qbnex.qir` from `target/qb-temp/program.qir`
once the fixpoint holds.

## Coding Standards

### QBNex BASIC Code

The compiler is written in QBNex BASIC itself. Follow these conventions:

**Naming:**
- Use descriptive, meaningful names
- Variables: `camelCase` or `snake_case`
- Functions/Subroutines: `PascalCase`
- Constants: `UPPER_CASE`

**Formatting:**
- Use consistent indentation (spaces preferred)
- Keep lines under 120 characters when possible
- Use whitespace to improve readability
- Add comments for complex logic

**Example:**
```basic
' Good: Clear function name and parameters
FUNCTION CalculateTotal (price AS SINGLE, quantity AS INTEGER)
    DIM total AS SINGLE
    total = price * quantity
    CalculateTotal = total
END FUNCTION

' Good: Comment explaining complex logic
' Convert QB line numbers to sequential labels
' This handles GOTO/GOSUB targets in legacy code
FUNCTION GenerateLabel$ (lineNumber AS INTEGER)
    GenerateLabel = "LABEL_" + STR$(lineNumber)
END FUNCTION
```

**Error Handling:**
- Validate inputs where appropriate
- Use QBNex's error handling mechanisms
- Provide clear error messages
- Log errors in debug mode

### Runtime Code

The runtime is a Cargo workspace under `crates/` (`qb-runtime`, `qb-audio`, `qb-ir`, `qb-exec`).
A new built-in needs an entry in the QIR table (`crates/compiler/ir/src/builtins.rs`), its
implementation in `crates/engine/exec/src/rt/`, and the emitter case in
`compiler/utilities/ir_emit.bas` / `ir_stmt.bas`. Rust changes should pass `cargo test --workspace`.

### Commit Messages

Write clear, descriptive commit messages:

**Format:**
```
type: Brief description (50 chars or less)

Optional detailed explanation (wrap at 72 chars)
explaining what changed and why
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, no logic changes)
- `refactor`: Code refactoring (no feature changes)
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `build`: Build system or dependency changes
- `ci`: CI/CD pipeline changes

**Examples:**
```
fix: Handle REDIM PRESERVE with multi-dimensional arrays

The previous implementation incorrectly calculated array bounds
when resizing dimensions. This fix properly preserves data during
complex redimension operations.

feat: Add TCP/IP networking support with _OPENHOST

Implements socket API for server and client connections.
Supports Windows (ws2_32) and POSIX (BSD sockets).

docs: Update README with comprehensive command reference
```

## Testing

### Automated Tests

```bash
# Runtime unit tests (all crates, or one: -p qb-runtime / -p qb-audio / -p qb-exec)
cargo test --workspace

# Conformance corpus (needs a built qb / qbnex in the repository root)
cargo run --release -p qbnex-conformance

# Smoke tests
./tests/smoke/cli_smoke.sh          # Linux/macOS
tests\smoke\cli_smoke.cmd           # Windows
```

### Manual Testing

Test your changes manually:

1. **Compile test programs** that use the changed functionality
2. **Verify on multiple platforms** if possible (Windows, Linux, macOS)
3. **Test edge cases** and error conditions
4. **Check backward compatibility** with existing QBasic code

### Test Cases

Create test programs that demonstrate:

- **Correct behavior**: The feature works as expected
- **Error handling**: Appropriate errors for invalid input
- **Edge cases**: Boundary conditions, empty inputs, etc.
- **Cross-platform compatibility**: Works on all target platforms

### Continuous Integration

QBNex uses GitHub Actions for CI:

- **Push to master**: Builds on Linux
- **Pull requests**: Builds on Linux
- **Releases**: Builds on Linux, macOS, Windows x64

If you need a Windows x86 / 32-bit artifact, build it locally with `scripts\setup_win.cmd` from the repository instead of relying on GitHub Releases.

Ensure your changes pass CI before requesting review.

## Submitting Changes

### Pull Request Guidelines

1. **One feature/fix per PR**: Keep changes focused and reviewable
2. **Reference issues**: Use "Fixes #123" or "Closes #123" in description
3. **Describe changes**: Explain what changed and why
4. **Test thoroughly**: Ensure everything works before submitting
5. **Update documentation**: Update README, CHANGELOG, etc. as needed
6. **Follow the template**: Fill out the PR template completely

### Pull Request Template

When creating a PR, include:

```markdown
## Description
Brief summary of changes

## Type of Change
- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Code refactoring

## Testing
- [ ] Tests pass locally
- [ ] Tested on Windows
- [ ] Tested on Linux
- [ ] Tested on macOS (if applicable)
- [ ] Added test cases for new functionality

## Checklist
- [ ] My code follows the project's style guidelines
- [ ] I have performed a self-review of my code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] I have made corresponding changes to the documentation
- [ ] My changes generate no new warnings
```

## Review Process

### What Reviewers Look For

- **Correctness**: Does the code work as intended?
- **Compatibility**: Does it maintain QBasic compatibility?
- **Performance**: Does it introduce performance regressions?
- **Code quality**: Is it well-structured and readable?
- **Testing**: Are there adequate tests?
- **Documentation**: Is documentation updated?
- **Standards**: Does it follow project conventions?

### Addressing Review Feedback

- Respond to all review comments
- Make requested changes promptly
- Push additional commits to address feedback
- Request re-review when ready
- Be respectful and professional in discussions

### Merging

- PRs require approval from maintainers
- All CI checks must pass
- Resolve merge conflicts before merging
- Squash commits if requested for cleaner history

## Project Structure

```
QBNex/
├── compiler/                   # Compiler source code (QBNex BASIC)
│   ├── qbnex.bas               # Main compiler
│   ├── global/                 # Version, constants, settings
│   ├── stdlib/                 # Standard library modules
│   ├── subs_functions/         # Built-in functions/subs
│   └── utilities/              # Helper modules
├── crates/                     # Native runtime (Cargo workspace)
│   ├── runtime/core/           # Runtime library
│   ├── compiler/ir/            # QIR parser, validator, builtin table
│   ├── runtime/audio/          # Audio engine
│   └── engine/exec/            # Runner for compiled QIR programs
├── bootstrap/qbnex.qir         # The compiler in QIR (setup bootstrap)
├── tests/
│   ├── conformance/            # Conformance runner and cases
│   ├── fixtures/               # Test input programs
│   └── smoke/                  # Smoke test scripts
├── scripts/                    # Platform setup scripts (setup_*.cmd/sh/command)
├── docker/                     # Dockerfile, Dockerfile.dev, docker-compose.yml
├── packaging/installer/        # Inno Setup installer script
├── docs/                       # Design notes
├── .github/                    # GitHub workflows and templates
├── assets/                     # Logo and icons
├── licenses/                   # License files
├── README.md                   # Main documentation
├── CHANGELOG.md                # Version history
├── SECURITY.md                 # Security policies
└── CONTRIBUTING.md             # This file
```

## Getting Help

- **Questions**: Open a Discussion on GitHub
- **Bugs**: Create an Issue with details
- **Chat**: Check if project has communication channels
- **Documentation**: Read README and code comments

## Recognition

Contributors will be recognized in:
- README.md acknowledgments section
- Release notes
- Project documentation

Thank you for contributing to QBNex!

---

**Last Updated**: April 11, 2024
