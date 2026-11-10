# Installation

[Manual index](../README.md) · [ภาษาไทย](../th/installation.md)

## Release packages

Tagged releases publish `qbnex_<tag>_win-x64.zip`, `qbnex_<tag>_lnx.tar.gz`
and `qbnex_<tag>_osx.tar.gz`. Extract to a folder you can write to. On
Windows it helps to exclude that folder from antivirus scanning (compiled
programs are new executables, which scanners like to inspect).

## Building from source

You need [Rust](https://rustup.rs) (`cargo`) on every platform: the runtime
is a set of Rust crates (`crates/qb-*`).

### Windows

```cmd
git clone https://github.com/thirawat27/QBNex.git
cd QBNex
scripts\setup_win.cmd
```

The script downloads MinGW (about 150 MB) into `internal\c\c_compiler`,
builds the Rust runtime `libqb_runtime.a`, builds a stage-0 compiler and
lets it compile `compiler\qbnex.bas` into `qb.exe` in the repository root.
For a 32-bit build set `QBNEX_MINGW_ARCH=x86` first.

### Linux

```bash
# Debian/Ubuntu
sudo apt-get install g++ libasound2-dev libudev-dev pkg-config
chmod +x scripts/setup_lnx.sh
./scripts/setup_lnx.sh
```

`libasound2-dev` (sound) and `libudev-dev` (gamepads) are only needed for
their `.pc` files and libraries; nothing in C is compiled from them. Fedora:
`gcc-c++ alsa-lib-devel systemd-devel pkgconf-pkg-config`; Arch:
`gcc alsa-lib systemd-libs pkgconf`.

### macOS

```bash
xcode-select --install
chmod +x scripts/setup_osx.command
./scripts/setup_osx.command
```

### What you get

| File | What it is |
| --- | --- |
| `qb` / `qb.exe` | The compiler. |
| `qbnex` (Linux/macOS), `qbnex.cmd` (Windows) | Same compiler under the product name, plus the `fmt`, `lsp` and `dap` subcommands (see [commands](commands.md)). |
| `qb.cmd` | Windows wrapper with the same subcommands. Typing `qb` in a folder that holds `qb.exe` runs `qb.exe` itself, so use `qbnex` or `qb.cmd` for the subcommands. |

### Developer tools (optional)

The formatter, language server and debugger are separate Rust programs:

```bash
cargo build --release -p qb-fmt -p qb-lsp -p qb-dap
```

`qbnex fmt|lsp|dap` finds them next to the script, then in
`target/release` or `target/debug` of the checkout, then on `PATH`.

### The QIR back end (Preview)

`qb file.bas --ir` packages the program for the prebuilt Rust runner
`crates/qb-exec/bin/<os>/qb-exec` instead of calling g++. Build the runner
once with `crates/qb-exec/os/<os>/setup_build.*` (or
`cargo build --release -p qb-exec` and copy it there). The setup scripts that
build QBNex without any C/C++ toolchain (`scripts/setup_qir.sh`,
`scripts/setup_qir_win.cmd`) are switched off until the cutover:
`QBNEX_QIR_CUTOVER=1` turns them on for testing.

### CMake

```bash
cmake --preset windows-mingw && cmake --build --preset windows-mingw-selfhost   # Windows
cmake --preset linux-gcc && cmake --build --preset linux-gcc-stage0             # Linux
cmake --preset macos-clang && cmake --build --preset macos-clang-stage0         # macOS
```

### Docker

```bash
docker build -f docker/Dockerfile -t qbnex .
docker run --rm -v "$(pwd)":/project qbnex qb yourfile.bas
```

## Check the installation

```bash
qb --version
qb hello.bas        # writes hello.exe (Windows) or ./hello
```

Platform notes: the Windows build is the one tested most. Linux and macOS
builds of the Rust runtime are written and reviewed but less tested; see
`docs/rust-port/progress.md`.
