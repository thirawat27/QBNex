# QB64

![QB64](source/qb64.png)

QB64 is a modern extended BASIC+OpenGL language that retains QB4.5/QBasic compatibility and compiles native binaries for Windows (XP and up), Linux and macOS.
=======
[![contributions welcome](https://img.shields.io/badge/contributions-welcome-brightgreen.svg?style=flat)](https://github.com/QB64Team/qb64/issues)

# Table of Contents
1. [Installation](#Installation)
    1. [Windows](#Windows)
    2. [macOS](#macOS)
    3. [Linux](#Linux)

2.  [Usage](#Usage)
3.  [Additional Info](#Additional_Info)

# Installation <a name="Installation"></a>
Download the appropriate package for your operating system. Check the Releases page.

<a name="Windows"></a>
## Windows

Make sure to extract the package contents to a folder with full write permissions (failing to do so may result in packaging or compilation errors).

* It is advisable to to whitelist the QB64 folder in your antivirus/antimalware software *

<a name="macOS"></a>
## macOS
Before using QB64 make sure to install the Xcode command line tools with:
```bash
xcode-select --install
```

Run ```./setup_osx.command``` to compile QB64 for your OS version.

<a name="Linux"></a>
## Linux
Compile QB64 with ```./setup_lnx.sh```.

Dependencies should be automatically installed. Required packages include OpenGL, ALSA and the GNU C++ Compiler.

<a name="Usage"></a>
# Usage
QB64 now runs as a command-line compiler only.

Use `qb` as the primary command, or `qb64` if you want the full executable name:

```qb yourfile.bas```

```qb yourfile.bas -o outputname.exe```

Use `-x` to keep compiler output in the terminal:

```qb yourfile.bas -x```
