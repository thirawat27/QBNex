# Installation

[Manual index](../README.md) · [ภาษาไทย](../th/installation.md)

## Release packages

Tagged releases publish `qbnex_<tag>_win-x64.zip`, `qbnex_<tag>_lnx.tar.gz`
and `qbnex_<tag>_osx.tar.gz`. Extract to a folder you can write to. On
Windows it helps to exclude that folder from antivirus scanning (compiled
programs are new executables, which scanners like to inspect).

The same releases have a one-file installer per system:
`qbnex_<tag>_win-x64-setup.exe`, `qbnex_<tag>_lnx-setup` and
`qbnex_<tag>_osx-setup`. Run it: a setup window leads through the license,
the folder and the tasks below, shows the progress, and can be cancelled at
any point (a cancelled or failed install puts back what was there before).
It installs for the current user, without administrator rights:

- Windows: `%LOCALAPPDATA%\Programs\QBNex`, added to your PATH, to the Start
  menu and to "Apps & features" (uninstall from there).
- Linux and macOS: `~/.local/share/qbnex`, with `qb` and `qbnex` linked in
  `~/.local/bin` (add that folder to PATH if it is not there) and a
  "QBNex Terminal" entry in the applications menu (macOS: a script in
  `~/Applications`).

Optional tasks: a desktop shortcut, and "Compile with QBNex" in the file
manager's menu for `.bas` files (Windows and Linux).

Where no window can be shown (no display, a remote shell) the installer asks
in the terminal instead; `--console` asks there always. For scripts:
`qbnex-setup --yes` installs without asking (`/S`, `/SILENT` and `/VERYSILENT`
mean the same), `--dir <folder>` (or `/DIR=<folder>`) picks another folder,
`--no-path` leaves PATH alone, `--no-menu` adds no menu entry, `--desktop` and
`--associate` turn the optional tasks on. `qbnex-setup --uninstall` (the copy
in the install folder) removes everything it installed and leaves files you
added. The installer and `qb`
are not code-signed: Windows SmartScreen and macOS Gatekeeper may warn before
the first run (macOS: `xattr -d com.apple.quarantine qbnex_<tag>_osx-setup`).

## Building from source

You need [Rust](https://rustup.rs) (`cargo`) on every platform. No C or C++
compiler is used: the compiler is written in BASIC, translated to QIR (a
text intermediate form) and run by the Rust runner `qb-exec`.

### Windows

```cmd
git clone https://github.com/thirawat27/QBNex.git
cd QBNex
scripts\setup_win.cmd
```

### Linux

```bash
# Debian/Ubuntu: libraries for sound, gamepads and the window
sudo apt-get install gcc libasound2-dev libudev-dev libxkbcommon-dev libx11-dev pkg-config
sh scripts/setup_lnx.sh
```

Fedora: `gcc alsa-lib-devel systemd-devel libxkbcommon-devel libX11-devel
pkgconf-pkg-config`; Arch: `gcc alsa-lib systemd-libs libxkbcommon libx11
pkgconf`. `gcc` is only the system linker Rust uses on Linux; no C or C++
source is compiled.

Programs that open a window also need the X11 keyboard library at run time:
`libxkbcommon-x11-0` on Debian/Ubuntu, `libxkbcommon-x11` on Fedora and Arch
(desktop systems have it). Without it a program runs without a window and
says which library is missing.

### macOS

```bash
sh scripts/setup_osx.command
```

### What the setup does

1. Builds the runner `qb-exec` (`cargo build --release -p qb-exec`) into
   `crates/engine/exec/bin/<os>/`.
2. Packs `bootstrap/qbnex.qir` (the compiler, already translated to QIR)
   into that runner, giving a stage-0 compiler.
3. Lets stage 0 compile `compiler/qbnex.bas` into `qb` / `qb.exe` in the
   repository root, then runs a smoke test.

A compiled program is a copy of the runner with the program's QIR appended,
so it needs no other files.

Setup variables: `QBNEX_QB_EXEC` (use a prebuilt `qb-exec` instead of
building it), `QBNEX_RUST_TARGET`, `QBNEX_BOOTSTRAP`,
`QBNEX_QIR_PACK=ir|native`, `QBNEX_STAGE1_ARGS` (extra switches for the
stage-0 compile), `QBNEX_SKIP_SELFHOST=1` (stop at `qb-stage0`),
`QBNEX_KEEP_STAGE0=1`, and on Windows `QBNEX_CI=1` (no pause at the end).
See the header of `scripts/setup_win.cmd` and `scripts/setup_lnx.sh`.

**Native executables (Preview):** `scripts/native-kit.ps1` (Windows) or
`scripts/native-kit.sh` builds the native kit (`qb-native` plus the Rust
toolchain's `rust-lld`). Copied to `crates/engine/exec/bin/<os>/native/`, it makes
the compiler link ordinary executables instead of appending QIR to the runner
(see [commands](commands.md)).

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

### Docker

```bash
docker build -f docker/Dockerfile -t qbnex .
docker run --rm -v "$(pwd)":/project qbnex qb yourfile.bas
```

## Check the installation

```bash
qb --version
qb hello.bas        # writes hello.exe (Windows) or hello in the QBNex folder
```

Without `-o` the output goes to the QBNex folder (see [commands](commands.md)).

Platform notes: the Windows build is the one tested most. Linux and macOS
builds are built and tested in CI (GitHub Actions) only.
