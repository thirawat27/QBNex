# การติดตั้ง

[สารบัญคู่มือ](../README.md) · [English](../en/installation.md)

## แพ็กเกจสำเร็จรูป

รุ่นที่ติด tag จะมีไฟล์ `qbnex_<tag>_win-x64.zip`, `qbnex_<tag>_lnx.tar.gz`
และ `qbnex_<tag>_osx.tar.gz` ให้แตกไฟล์ไว้ในโฟลเดอร์ที่เขียนไฟล์ได้ บน
Windows ควรยกเว้นโฟลเดอร์นี้จากการสแกนของโปรแกรมแอนตี้ไวรัส
(โปรแกรมที่คอมไพล์ออกมาเป็นไฟล์ exe ใหม่ ซึ่งตัวสแกนมักจะเข้าไปตรวจ)

## Build จาก source

ทุกระบบปฏิบัติการต้องมี [Rust](https://rustup.rs) (`cargo`) เพราะ runtime
เป็นชุด crate ภาษา Rust (`crates/qb-*`)

### Windows

```cmd
git clone https://github.com/thirawat27/QBNex.git
cd QBNex
scripts\setup_win.cmd
```

สคริปต์จะดาวน์โหลด MinGW (ประมาณ 150 MB) ไว้ที่ `internal\c\c_compiler`
build runtime ภาษา Rust (`libqb_runtime.a`) สร้างคอมไพเลอร์ stage-0
แล้วให้มันคอมไพล์ `compiler\qbnex.bas` ออกมาเป็น `qb.exe` ที่ root ของ
repository ถ้าต้องการ build แบบ 32-bit ให้ตั้ง `QBNEX_MINGW_ARCH=x86` ก่อน

### Linux

```bash
# Debian/Ubuntu
sudo apt-get install g++ libasound2-dev libudev-dev pkg-config
chmod +x scripts/setup_lnx.sh
./scripts/setup_lnx.sh
```

`libasound2-dev` (เสียง) และ `libudev-dev` (จอยเกม) ใช้แค่ไฟล์ `.pc` กับ
ไลบรารีของระบบ ไม่มีการคอมไพล์โค้ด C จากแพ็กเกจเหล่านี้ Fedora ใช้
`gcc-c++ alsa-lib-devel systemd-devel pkgconf-pkg-config` ส่วน Arch ใช้
`gcc alsa-lib systemd-libs pkgconf`

### macOS

```bash
xcode-select --install
chmod +x scripts/setup_osx.command
./scripts/setup_osx.command
```

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

### QIR back end (ทดลอง)

`qb file.bas --ir` จะแพ็กโปรแกรมให้ runner ภาษา Rust ที่ build ไว้แล้ว
(`crates/qb-exec/bin/<os>/qb-exec`) รันแทนการเรียก g++ ให้ build runner
ครั้งเดียวด้วย `crates/qb-exec/os/<os>/setup_build.*` (หรือ
`cargo build --release -p qb-exec` แล้วคัดลอกไปวางไว้ที่นั่น) ส่วนสคริปต์
setup ที่ build QBNex โดยไม่ใช้ toolchain C/C++ เลย (`scripts/setup_qir.sh`,
`scripts/setup_qir_win.cmd`) ยังปิดไว้จนกว่าจะถึงจังหวะ cutover
ถ้าต้องการลองให้ตั้ง `QBNEX_QIR_CUTOVER=1`

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

## ตรวจว่าติดตั้งได้

```bash
qb --version
qb hello.bas        # ได้ hello.exe (Windows) หรือ ./hello
```

หมายเหตุเรื่องแพลตฟอร์ม: build บน Windows ผ่านการทดสอบมากที่สุด ส่วน
runtime บน Linux และ macOS เขียนและตรวจโค้ดแล้วแต่ทดสอบน้อยกว่า ดูรายละเอียด
ใน `docs/rust-port/progress.md`
