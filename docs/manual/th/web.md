# รันบนเว็บ (WebAssembly)

[สารบัญคู่มือ](../README.md) · [English](../en/web.md)

สถานะ: **ทดลอง** โปรแกรม QBNex รันในเบราว์เซอร์ได้ โดย runner ของ QIR
(QIR -> bytecode -> VM ไม่มี JIT) ถูก build สำหรับ `wasm32-unknown-unknown`
ออกมาเป็น `qb_web.wasm` และมีหน้าเว็บ static เล็ก ๆ ใน `crates/web/www`
สำหรับรันไฟล์ `.qir` ไม่มีการคอมไพล์ในเบราว์เซอร์ รายละเอียดสำหรับนักพัฒนา:
[crates/web/README.md](../../../crates/web/README.md)

## Build

```bash
rustup target add wasm32-unknown-unknown        # ครั้งเดียว
cargo build -p qb-web --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/qb_web.wasm crates/web/www/
```

ไฟล์ module มีขนาดประมาณ 4 MB

## แปลงโปรแกรมเป็น QIR

```bash
qb game.bas -o game.qir          # หรือ: qb game.bas --emit-ir
```

ถ้าชื่อไฟล์ผลลัพธ์ลงท้ายด้วย `.qir` (หรือใช้ `--emit-ir`) คอมไพเลอร์จะเขียน
ข้อความ QIR ของโปรแกรมแทน executable ข้อความนี้คือสิ่งที่หน้าเว็บนำไปรัน

## รัน

```bash
node crates/web/www/serve.mjs          # http://localhost:8080/
```

`serve.mjs` เปิด `www/` พร้อม header cross-origin isolation (COOP/COEP)
ที่คีย์บอร์ดและเมาส์แบบโต้ตอบต้องใช้ (หน้าเว็บแชร์ `SharedArrayBuffer` กับ worker)
และส่ง `qb_web.wasm` จาก `www/` หรือถ้าไม่มีจะส่งจาก
`target/wasm/wasm32-unknown-unknown/release/` (build ด้วย
`CARGO_TARGET_DIR=target/wasm`) จะใช้ static server ตัวอื่นก็ได้ถ้ามี
`qb_web.wasm` อยู่ใน `www/` ถ้าไม่มี header เหล่านี้โปรแกรมยังรันได้ แต่ INPUT
จะอ่านได้แค่คำตอบที่เตรียมไว้

หน้าเว็บมีรายการตัวอย่าง (`hello`: ข้อความ, COLOR, BEEP; `graphics`:
ภาพเคลื่อนไหว SCREEN 13; `guess`: เกม INPUT ที่มี SOUND และ PLAY) ช่องเปิดหรือวาง
ไฟล์ `.qir` canvas ของหน้าจอ (คลิกแล้วพิมพ์ได้) ช่องคำตอบที่เตรียมไว้ให้ INPUT
และช่อง console สำหรับ output ของ `$CONSOLE` / `_DEST _CONSOLE`

- `worker.js` รันโปรแกรมแยกจาก thread ของหน้าเว็บ `_DELAY`, `_LIMIT` และ
  `SLEEP` จึงรอจริง และเฟรมถูกส่งไปวาดบน canvas ระหว่างที่โปรแกรมทำงาน
- `qbnex.js` สร้าง instance ใหม่ทุกครั้งที่รัน
- หน้าจอข้อความใช้ฟอนต์ 8x16 ในตัว ส่วน SCREEN 13, 32-bit และหน้าที่ใช้
  palette วาดเป็นพิกเซล

## อะไรใช้ได้ / อะไรยังไม่ได้

| QBasic | ในเบราว์เซอร์ |
| --- | --- |
| PRINT, CLS, LOCATE, COLOR, กราฟิก SCREEN 0/13/32-bit, `_DISPLAY` | runtime เป็นคนวาด เฟรมถูกส่งตอน `_DISPLAY`, `_LIMIT`, `_DELAY` และระหว่างที่โปรแกรมรออ่าน input |
| INKEY$, `_KEYHIT`, `_KEYDOWN`, `_MOUSEINPUT`, `_MOUSEX`, `_MOUSEY`, `_MOUSEBUTTON`, `_MOUSEWHEEL` | เหตุการณ์คีย์บอร์ดและเมาส์ของ canvas (ต้องมี header isolation) |
| INPUT, LINE INPUT | อ่านคำตอบที่เตรียมไว้ก่อน แล้วจึงรอพิมพ์บนหน้าจอ |
| SOUND, BEEP, PLAY, `_SND*` | ผสมเสียงที่ 44.1 kHz แล้วเล่นผ่าน WebAudio ใช้เฉพาะขีดจำกัดคิวของ `PLAY "MB"` |
| TIMER, `_DELAY`, `_LIMIT`, `SLEEP n` | ใช้นาฬิกาของหน้าเว็บ |
| ไฟล์, SHELL, TCP, HTTP | ใช้ไม่ได้: ได้ error ปกติของ QBasic (OPEN ได้ error 75 หรือ 53) ซึ่ง `ON ERROR` ดักได้ |
| JIT | ไม่มี (ใช้ VM อย่างเดียว) |

คำสั่งที่ไม่รองรับจะจบด้วย error ของ QBasic ไม่ใช่โปรแกรมพัง เทสต์:
`node crates/web/tests/run.mjs [path/to/qb_web.wasm]` (headless ใน Node) และ
`cargo test -p qb-web`
