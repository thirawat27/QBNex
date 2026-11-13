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

**For Linux:** `gcc` (only as the system linker Rust uses) and the development packages for
ALSA, udev, xkbcommon and X11 with `pkg-config`:

```bash
sudo apt-get install gcc libasound2-dev libudev-dev libxkbcommon-dev libx11-dev pkg-config
```

**For macOS:** nothing beyond Rust.

**Optional:** PowerShell 7 (`pwsh`) for `tests/docs/check_docs.ps1` and the Windows helper scripts.

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

**Submit a bug report** as a new issue on the Issues tab, with the information above. The
complete compiler output (`qb file.bas -w -m`) and, for compiler bugs, the QIR of the build
(`target/qb-temp/program.qir`) help. Security problems go through [SECURITY.md](SECURITY.md),
not a public issue.

### Suggesting Features

Feature suggestions help QBNex grow. When suggesting features:

1. **Check existing issues** for similar requests
2. **Explain the use case** clearly
3. **Provide examples** if possible
4. **Consider scope** and implementation complexity

**Submit a feature request** as a new issue that describes the feature, its syntax if it is
a language change, and why it is needed. Planned language features are listed in
[docs/manual/en/language.md](docs/manual/en/language.md#planned).

### Code Contributions

QBNex is primarily written in **QBNex BASIC** itself (self-hosting compiler). The codebase structure:

- **`compiler/`**: Compiler source code (written in QBNex BASIC)
  - `qbnex.bas`: Main compiler entry point
  - `global/`: Version, constants, and settings
  - `subs_functions/`: Built-in functions and subroutines
  - `utilities/`: Helper modules
- **`crates/`**: Rust code (Cargo workspace)
  - `compiler/ir` (`qb-ir`): QIR types, parser, validator and builtin table; `compiler/syntax` (`qb-syntax`): lexer and keyword tables for the tools
  - `engine/exec` (`qb-exec`): runner for compiled QIR programs (VM); `engine/jit` (`qb-jit`): Cranelift JIT; `engine/link`, `engine/native`: native executables; `engine/ffi` (`qb-ffi`): `DECLARE LIBRARY`
  - `runtime/core` (`qb-runtime`): window, graphics, console, files, network; `runtime/audio` (`qb-audio`); `runtime/float80` (`qb-float80`); `runtime/gpu` (`qb-gpu`); `runtime/ext` (`qb-ext`, the `ext.*` modules)
  - `tools/fmt`, `tools/lsp`, `tools/dap`, `tools/setup` (`qb-fmt`, `qb-lsp`, `qb-dap`, the installer `qbnex-setup`); `web` (`qb-web`)
- **`bootstrap/qbnex.qir`**: the compiler in QIR, used by the setup scripts
- **`tests/`**: `smoke/` scripts, `fixtures/`, `conformance/` runner and cases, `docs/` documentation test
- **`scripts/`**: setup, self-host fixpoint, native kit, benchmarks
- **`.github/workflows/`**: `ci.yml`, `docs.yml`, `release.yml`

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

- **`docs/manual/en` and `docs/manual/th`**: the user manual. Change both languages together;
  each feature carries a status label (Available, Preview, Planned; see `docs/manual/README.md`).
- **`docs/dev`**: developer reference (QIR format, deviation IDs, FFI, 3D API). An intentional
  difference from QB64 gets a `D-xx` entry in `docs/dev/deviations.md`.
- **README.md**: overview, usage and examples; where it and the manual disagree, the manual wins.
- **CHANGELOG.md**: every user-visible change, under the version being prepared.
- **Code comments**: clarity in complex sections.

Every ```` ```basic ```` block of README.md and `docs/manual` is compiled by
`tests/docs/check_docs.ps1`, and blocks marked `<!-- docs-test: output -->` must print the
output shown below them ([tests/docs/README.md](tests/docs/README.md)). Run it before you
push documentation changes:

```bash
pwsh tests/docs/check_docs.ps1 -Compiler ./qb      # or qb.exe on Windows
```

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

What the setup scripts do, step by step:

```bash
cargo build --release -p qb-exec
# copy target/release/qb-exec(.exe) to crates/engine/exec/bin/<os>/, then pack
# bootstrap/qbnex.qir into a copy of it: that copy is qb-stage0
# (scripts/qir_pack.ps1 on Windows; setup_lnx.sh does the same in shell)
./qb-stage0 compiler/qbnex.bas -o qb
sh scripts/selfhost_fixpoint.sh   # gen2 == gen3 == bootstrap/qbnex.qir
```

Setup variables: `QBNEX_QB_EXEC` (use a prebuilt `qb-exec`), `QBNEX_RUST_TARGET`,
`QBNEX_BOOTSTRAP` (another bootstrap QIR), `QBNEX_QIR_PACK=ir|native`, `QBNEX_STAGE1_ARGS`
(extra switches for the stage-0 compile), `QBNEX_SKIP_SELFHOST=1` (stop at `qb-stage0`),
`QBNEX_KEEP_STAGE0=1`, and on Windows `QBNEX_CI=1` (no pause at the end).

After changing the compiler, rebuild `qb` with itself, run the fixpoint, and refresh
`bootstrap/qbnex.qir` from `target/qb-temp/program.qir` of the last generation once gen2 and
gen3 agree. `bootstrap/qbnex.qir` is the Windows build of the compiler (its QIR differs per OS
in a few places), so only a Windows run compares against it; elsewhere set
`QBNEX_FIXPOINT_BOOTSTRAP=0`.

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

The runtime is a Cargo workspace under `crates/`. A new built-in needs:

- its keyword in `compiler/subs_functions/subs_functions.bas` (syntax, argument types),
- an entry in the QIR table (`crates/compiler/ir/src/builtins.rs`),
- the emitter case in `compiler/utilities/ir_emit.bas` / `ir_stmt.bas` when it is not a plain call,
- its implementation in `crates/engine/exec/src/rt/` (and the runtime crates),
- the keyword in `crates/compiler/syntax` so the formatter and language server know it,
- a conformance case, and the manual page in English and Thai.

Rust changes should pass `cargo test --workspace` and `cargo clippy --workspace -- -D warnings` (as in CI). The toolchain
is pinned in `rust-toolchain.toml` (Rust 1.100.0).

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
# Rust unit and integration tests (all crates, or one: -p qb-runtime / -p qb-audio / -p qb-exec)
cargo test --workspace

# Conformance: compiles tests/conformance/cases/**.bas with the built qb in the
# repository root and compares the output with the golden files
cargo run --release -p qbnex-conformance

# Smoke scripts (the ones CI runs)
for t in cli diagnostics warnings labels encoding stdlib stdlib_modules compiler_robustness audio chain; do
    sh "tests/smoke/${t}_smoke.sh" || break      # Windows: tests\smoke\%t_smoke.cmd
done
sh tests/smoke/benchmark_smoke.sh               # compile-time benchmark (manual)

# Self-host fixpoint (after a compiler change)
sh scripts/selfhost_fixpoint.sh                 # Windows: scripts\selfhost_fixpoint.cmd

# Documentation examples
pwsh tests/docs/check_docs.ps1
```

Set `QBNEX_NO_ERROR_DIALOG=1` when you run compiled programs from scripts, so a runtime error
ends the program instead of waiting at a dialog.

**Conformance cases.** A behaviour change gets a case in `tests/conformance/cases/<area>/`:

| File | Meaning |
| --- | --- |
| `name.bas` | The program |
| `name.out` | Expected stdout (LPRINT output follows a `[LPRINT]` line) |
| `name.in` | Piped to stdin |
| `name.input` | Input script (keys, mouse, window) replayed headless (`QBNEX_INPUT_SCRIPT`) |
| `name.fb` | Expected framebuffer records, one line each (`QBNEX_FB_CAPTURE`) |
| `name.deviation` | Intentional difference from the legacy runtime; `.out` is written by hand, `--bless` leaves it alone |
| `name.xfail` | Known defect, first line is the reason; the run fails with `XPASS` once it passes |
| `name.windows` | Windows-only case (skipped elsewhere) |

`cargo run --release -p qbnex-conformance -- --bless` rewrites the `.out` files from the current
compiler; use it only with a build you trust.

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

- **Push and pull requests** (`main`, `rust-port`): `ci.yml` runs a dependency audit (also every Monday), clippy, and builds and tests on Linux x64, macOS arm64 and Windows x64 (workspace tests, smoke tests, conformance, native executables; the self-host fixpoint compares against `bootstrap/qbnex.qir` on Windows, where it is built, and checks gen2 == gen3 on Linux). Pushes that only change Markdown, `docs/` or `packaging/vscode/` skip it; `ci-skip` in the commit message skips it too.
- **Documentation** (`docs.yml`): runs `tests/docs/check_docs.ps1` for changes to `README.md`, `docs/` or `tests/docs/`.
- **Releases** (`v*` tags): `release.yml` builds, tests and packages Linux, macOS and Windows x64, and the one-file installers

#### Code signing (release secrets)

`release.yml` signs `qb-fmt`, `qb-lsp` and `qb-dap` when the repository has these Actions secrets
(Settings > Secrets and variables > Actions). Without them the step prints a notice and the release
is published unsigned.

| Secret | Contents |
| --- | --- |
| `WINDOWS_CERT_PFX_BASE64` | Code signing certificate with its private key, a `.pfx` file in base64 (`[Convert]::ToBase64String([IO.File]::ReadAllBytes('cert.pfx'))`) |
| `WINDOWS_CERT_PASSWORD` | Password of that `.pfx` |
| `APPLE_CERT_P12_BASE64` | "Developer ID Application" certificate with its private key, exported from Keychain Access as `.p12`, in base64 (`base64 -i cert.p12`) |
| `APPLE_CERT_PASSWORD` | Password of that `.p12` |
| `APPLE_TEAM_ID` | Apple Developer team ID (10 characters) |
| `APPLE_ID` | Apple ID email used for notarization |
| `APPLE_APP_PASSWORD` | App-specific password of that Apple ID (appleid.apple.com) |

Windows uses `signtool` from the runner's Windows SDK with a SHA-256 RFC 3161 timestamp (a `.pfx`
certificate, not Azure Trusted Signing). macOS signs with `codesign` (hardened runtime, timestamp)
and notarizes with `xcrun notarytool`; bare executables cannot be stapled, so Gatekeeper checks the
ticket online. A step fails the release if a certificate is set but signing or notarization fails.

`qb`, the runner `qb-exec` and the one-file installers are not signed: each carries data appended
after the executable (the compiler's QIR, the bundle zip), found by a trailer at the end of the file.
A Windows signature is written after that trailer, and `codesign` refuses a Mach-O with data after
it. Every compiled program is also a copy of the runner with data appended. Signing these needs a
payload format that leaves room for the signature.

There are no 32-bit Windows builds.

Ensure your changes pass CI before requesting review.

## Submitting Changes

### Pull Request Guidelines

1. **One feature/fix per PR**: Keep changes focused and reviewable
2. **Reference issues**: Use "Fixes #123" or "Closes #123" in description
3. **Describe changes**: Explain what changed and why
4. **Test thoroughly**: Ensure everything works before submitting
5. **Update documentation**: Update README, CHANGELOG, etc. as needed
6. **Describe it fully**: the repository has no PR template; use the outline below
7. **Agree to the CLA**: your first pull request must state that you agree to the
   [Contributor License Agreement](CLA.md) (the line to add is at its end). You keep the
   copyright of your work; the agreement lets the project keep offering QBNex under the GPL
   and under other licenses.

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
├── crates/                     # Cargo workspace
│   ├── compiler/               # ir (QIR types, parser, validator), syntax (lexer for tools)
│   ├── engine/                 # exec (runner, VM), jit (Cranelift), link, native, ffi
│   ├── runtime/                # core, audio, float80, gpu, ext
│   ├── tools/                  # fmt, lsp, dap, setup (the one-file installer)
│   └── web/                    # WebAssembly build of the runner
├── bootstrap/qbnex.qir         # The compiler in QIR (setup bootstrap)
├── tests/
│   ├── conformance/            # Conformance runner and cases
│   ├── fixtures/               # Test input programs
│   ├── smoke/                  # Smoke test scripts
│   └── docs/                   # Compiles the BASIC examples of README.md and docs/manual
├── scripts/                    # Platform setup scripts (setup_*.cmd/sh/command)
├── docker/                     # Dockerfile, Dockerfile.dev, docker-compose.yml
├── packaging/vscode/           # VS Code extension
├── docs/                       # manual/ (users, en + th), dev/ (developer reference)
├── .github/                    # ci.yml (build, test, conformance, native exe), docs.yml, release.yml
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
- **Documentation**: Read README and code comments

## Recognition

Contributors will be recognized in:
- Release notes
- Project documentation

Thank you for contributing to QBNex!

