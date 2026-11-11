# QBNex manual / คู่มือ QBNex

QBNex compiles QBasic, QuickBASIC 4.5 and QB64 programs. The compiler
translates them to QIR, a small intermediate language, which the Rust
runner `qb-exec` runs (VM + JIT). No C or C++ compiler is involved.

QBNex คอมไพล์โปรแกรม QBasic, QuickBASIC 4.5 และ QB64 โดยแปลเป็น QIR
ซึ่งเป็นภาษากลางขนาดเล็ก แล้วให้ runner ภาษา Rust ชื่อ `qb-exec` รัน
(VM + JIT) ไม่ต้องใช้คอมไพเลอร์ C หรือ C++

| English | ภาษาไทย |
| --- | --- |
| [Installation](en/installation.md) | [การติดตั้ง](th/installation.md) |
| [The `qb` / `qbnex` commands](en/commands.md) | [คำสั่ง `qb` / `qbnex`](th/commands.md) |
| [Language extensions](en/language.md) | [ส่วนขยายของภาษา](th/language.md) |
| [Standard library](en/stdlib.md) | [ไลบรารีมาตรฐาน](th/stdlib.md) |
| [Graphics and 3D](en/graphics-3d.md) | [กราฟิกและ 3 มิติ](th/graphics-3d.md) |
| [Web (WebAssembly) build](en/web.md) | [รันบนเว็บ (WebAssembly)](th/web.md) |
| [VS Code, formatter, language server, debugger](en/vscode.md) | [VS Code, formatter, language server, debugger](th/vscode.md) |
| [Differences from QB64 and QBasic](en/differences.md) | [ความแตกต่างจาก QB64 และ QBasic](th/differences.md) |
| [Limitations and differences from QB64](en/limitations.md) | [ข้อจำกัดและความต่างจาก QB64](th/limitations.md) |

## Status labels / ป้ายสถานะ

Each feature in this manual carries one of these labels. ฟีเจอร์ในคู่มือนี้มีป้ายบอกสถานะดังนี้

- **Available / ใช้ได้แล้ว**: in the default build, covered by tests.
  อยู่ใน build ปกติและมีเทสต์ครอบคลุม
- **Preview / ทดลอง**: works, but behind a build option or with listed
  limits. ใช้ได้ แต่ต้องเปิด option ตอน build หรือมีข้อจำกัดตามที่ระบุ
- **Planned / วางแผนไว้**: designed (syntax fixed), not in the compiler yet.
  The editor tools may already know the words. ออกแบบแล้ว (syntax
  กำหนดแล้ว) แต่ยังไม่อยู่ในคอมไพเลอร์ เครื่องมือ editor อาจรู้จักคำเหล่านี้แล้ว

Developer documents about the port live in [`docs/rust-port`](../rust-port/README.md).
เอกสารสำหรับนักพัฒนาเรื่องการ port อยู่ใน [`docs/rust-port`](../rust-port/README.md)
