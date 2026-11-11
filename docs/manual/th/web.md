# รันบนเว็บ (WebAssembly)

[สารบัญคู่มือ](../README.md) · [English](../en/web.md)

สถานะ: **ทดลอง** โปรแกรม QBNex รันในเบราว์เซอร์ได้ โดย runner ของ QIR
(QIR -> bytecode -> VM ไม่มี JIT) ถูก build สำหรับ `wasm32-unknown-unknown`
ออกมาเป็น `qb_web.wasm` และมีหน้าเว็บ static เล็กๆ สำหรับรันไฟล์ `.qir`

## Build

```bash
cargo build -p qb-web --release --target wasm32-unknown-unknown
# (รัน rustup target add wasm32-unknown-unknown หนึ่งครั้งก่อน)
cp target/wasm32-unknown-unknown/release/qb_web.wasm crates/qb-web/www/
```

## แปลงโปรแกรมเป็น QIR

```bash
qb game.bas -o game.exe
```

ข้อความ QIR คือ payload ที่เก็บไว้ก่อน trailer `QBNEXIR0` ท้ายไฟล์
`game.exe` (16 ไบต์สุดท้ายคือความยาวกับ trailer) ดูวิธีอ่านได้จากโค้ดของ
debugger (`qir_payload` ใน `crates/qb-dap/src/adapter.rs`)

## รัน

เปิด `crates/qb-web/www` ผ่าน HTTP (module worker โหลดจาก `file://` ไม่ได้)
เช่นรัน `python -m http.server` ในโฟลเดอร์นั้น แล้วเปิดหน้าเว็บและเลือกไฟล์
`.qir` หน้าเว็บมี canvas สำหรับหน้าจอ ช่องใส่ข้อความ standard input และ
ช่อง console

- `worker.js` รันโปรแกรมแยกจาก thread ของหน้าเว็บ `_DELAY`, `_LIMIT` และ
  `SLEEP` จึงรอจริง และเฟรมถูกส่งไปวาดบน canvas ระหว่างที่โปรแกรมทำงาน
- `qbnex.js` สร้าง instance ใหม่ทุกครั้งที่รัน
- หน้าจอข้อความใช้ฟอนต์ 8x16 ในตัว ส่วน SCREEN 13, 32-bit และหน้าที่ใช้
  palette วาดเป็นพิกเซล

## อะไรใช้ได้ / อะไรยังไม่ได้

| ใช้ได้ | ยังไม่ได้ |
| --- | --- |
| PRINT, LOCATE, COLOR, CLS, กราฟิก SCREEN 0/13/32-bit, TIMER, `_DELAY`, `_LIMIT`, `_DISPLAY` | คีย์บอร์ดแบบโต้ตอบ (`INKEY$` ว่างเสมอ ส่วน INPUT อ่านข้อความที่ใส่ไว้ก่อนรัน) |
| INPUT / LINE INPUT จากช่อง stdin (แสดงข้อความที่อ่านบนจอ) | ไฟล์ (OPEN ล้มเหลวด้วย error ปกติที่ดักได้) |
| `ON ERROR` สำหรับคำสั่งที่ไม่รองรับ | TCP, HTTP, SHELL |
| | เสียง (เงียบ), JIT |

คำสั่งที่ไม่รองรับจะจบด้วย error ของ QBasic ไม่ใช่โปรแกรมพัง เทสต์:
`node crates/qb-web/tests/run.mjs` (ตรวจ 33 จุด)
