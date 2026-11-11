# การติดตั้ง

[สารบัญคู่มือ](../README.md) · [English](../en/installation.md)

## แพ็กเกจสำเร็จรูป

รุ่นที่ติด tag จะมีไฟล์ `qbnex_<tag>_win-x64.zip`, `qbnex_<tag>_lnx.tar.gz`
และ `qbnex_<tag>_osx.tar.gz` ให้แตกไฟล์ไว้ในโฟลเดอร์ที่เขียนไฟล์ได้ บน
Windows ควรยกเว้นโฟลเดอร์นี้จากการสแกนของโปรแกรมแอนตี้ไวรัส
(โปรแกรมที่คอมไพล์ออกมาเป็นไฟล์ exe ใหม่ ซึ่งตัวสแกนมักจะเข้าไปตรวจ)

## Build จาก source

ทุกระบบปฏิบัติการต้องมี [Rust](https://rustup.rs) (`cargo`) และไม่ต้องใช้
คอมไพเลอร์ C หรือ C++ เลย ตัวคอมไพเลอร์เขียนด้วย BASIC แปลเป็น QIR
(รูปแบบกลางที่เป็นข้อความ) แล้วรันด้วย runner ภาษา Rust ชื่อ `qb-exec`

### Windows

```cmd
git clone https://github.com/thirawat27/QBNex.git
cd QBNex
scripts\setup_win.cmd
```

### Linux

```bash
# Debian/Ubuntu: ไลบรารีสำหรับเสียง จอยเกม และหน้าต่าง
sudo apt-get install gcc libasound2-dev libudev-dev libxkbcommon-dev libx11-dev pkg-config
sh scripts/setup_lnx.sh
```

Fedora ใช้ `gcc alsa-lib-devel systemd-devel libxkbcommon-devel libX11-devel
pkgconf-pkg-config` ส่วน Arch ใช้ `gcc alsa-lib systemd-libs libxkbcommon
libx11 pkgconf` โดย `gcc` เป็นแค่ linker ของระบบที่ Rust ใช้บน Linux
ไม่มีการคอมไพล์ source ภาษา C หรือ C++

### macOS

```bash
sh scripts/setup_osx.command
```

### สิ่งที่สคริปต์ setup ทำ

1. build runner `qb-exec` (`cargo build --release -p qb-exec`) ไว้ที่
   `crates/engine/exec/bin/<os>/`
2. แพ็ก `bootstrap/qbnex.qir` (คอมไพเลอร์ที่แปลเป็น QIR ไว้แล้ว) เข้ากับ
   runner ได้เป็นคอมไพเลอร์ stage-0
3. ให้ stage 0 คอมไพล์ `compiler/qbnex.bas` ออกมาเป็น `qb` / `qb.exe` ที่
   root ของ repository แล้วรัน smoke test

โปรแกรมที่คอมไพล์แล้วคือสำเนาของ runner ที่ต่อท้ายด้วย QIR ของโปรแกรม
จึงไม่ต้องใช้ไฟล์อื่นประกอบ

ตัวแปรของสคริปต์ setup: `QBNEX_QB_EXEC` (ใช้ `qb-exec` ที่ build ไว้แล้ว
แทนการ build ใหม่), `QBNEX_RUST_TARGET`, `QBNEX_BOOTSTRAP`,
`QBNEX_QIR_PACK=ir|native`, `QBNEX_SKIP_SELFHOST=1` (หยุดที่ `qb-stage0`),
`QBNEX_KEEP_STAGE0=1` ดูหัวไฟล์ของ `scripts/setup_win.cmd` และ
`scripts/setup_lnx.sh`

**executable แบบ native (ทดลอง):** `scripts/native-kit.ps1` (Windows) หรือ
`scripts/native-kit.sh` จะ build native kit (`qb-native` กับ `rust-lld` ของ
toolchain Rust) ถ้าคัดลอกไปไว้ที่ `crates/engine/exec/bin/<os>/native/`
คอมไพเลอร์จะลิงก์ executable ปกติแทนการต่อ QIR ท้าย runner
(ดู[คำสั่ง](commands.md))

### ไฟล์ที่ได้

| ไฟล์ | คืออะไร |
| --- | --- |
| `qb` / `qb.exe` | ตัวคอมไพเลอร์ |
| `qbnex` (Linux/macOS), `qbnex.cmd` (Windows) | คอมไพเลอร์ตัวเดียวกันภายใต้ชื่อผลิตภัณฑ์ และรับคำสั่งย่อย `fmt`, `lsp`, `dap` (ดู[คำสั่ง](commands.md)) |
| `qb.cmd` | wrapper บน Windows ที่มีคำสั่งย่อยชุดเดียวกัน แต่ถ้าพิมพ์ `qb` ในโฟลเดอร์ที่มี `qb.exe` อยู่ Windows จะรัน `qb.exe` โดยตรง ดังนั้นให้ใช้ `qbnex` หรือ `qb.cmd` เมื่อต้องการคำสั่งย่อย |

### เครื่องมือสำหรับนักพัฒนา (ไม่บังคับ)

formatter, language server และ debugger เป็นโปรแกรม Rust แยกต่างหาก

```bash
cargo build --release -p qb-fmt -p qb-lsp -p qb-dap
```

`qbnex fmt|lsp|dap` จะหาโปรแกรมเหล่านี้ข้างสคริปต์ก่อน แล้วจึงหาใน
`target/release` หรือ `target/debug` ของ checkout แล้วค่อยหาใน `PATH`

### Docker

```bash
docker build -f docker/Dockerfile -t qbnex .
docker run --rm -v "$(pwd)":/project qbnex qb yourfile.bas
```

## ตรวจว่าติดตั้งได้

```bash
qb --version
qb hello.bas        # ได้ hello.exe (Windows) หรือ hello ในโฟลเดอร์ QBNex
```

ถ้าไม่ใส่ `-o` ไฟล์ผลลัพธ์จะไปอยู่ในโฟลเดอร์ QBNex (ดู[คำสั่ง](commands.md))

หมายเหตุเรื่องแพลตฟอร์ม: build บน Windows ผ่านการทดสอบมากที่สุด ส่วนบน
Linux และ macOS build และทดสอบใน CI (GitHub Actions) เท่านั้น
