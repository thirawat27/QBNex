# QBNex

![QBNex](source/qbnex.png)

QBNex is a modern extended BASIC+OpenGL language that retains QB4.5/QBasic compatibility and compiles native binaries for Windows, Linux, and macOS.

Version: `1.0.0`
Repository: https://github.com/thirawat27/QBNex

## Installation

Download the appropriate package for your operating system from the repository releases page.

### Windows

Extract the package to a folder with full write permissions.

It is advisable to whitelist the QBNex folder in your antivirus or antimalware software.

### macOS

Install the Xcode command line tools first:

```bash
xcode-select --install
```

Run `./setup_osx.command` to compile QBNex for your macOS version.

### Linux

Run `./setup_lnx.sh` to compile QBNex.

Required packages generally include OpenGL, ALSA, and the GNU C++ compiler.

## Usage

QBNex runs as a command-line compiler.

Use `qb` as the command name:

```bash
qb yourfile.bas
qb yourfile.bas -o outputname.exe
qb yourfile.bas -x
```
