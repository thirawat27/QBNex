# คำสั่ง `qb` / `qbnex`

[สารบัญคู่มือ](../README.md) · [English](../en/commands.md)

`qb` กับ `qbnex` คือคอมไพเลอร์ตัวเดียวกัน แต่ `qbnex` (ซึ่งเป็นสคริปต์
สำหรับเรียกใช้) รันเครื่องมือสำหรับนักพัฒนาได้ด้วย

## คอมไพล์

```bash
qb hello.bas                  # ได้ hello.exe บน Windows หรือ ./hello บนระบบอื่น
qb hello.bas -o tools/hi.exe  # กำหนดไฟล์ผลลัพธ์เอง
qb hello.bas -w               # แสดงคำเตือนด้วย
```

ถ้าไม่ใส่ `-o` ไฟล์ executable จะไปอยู่ในโฟลเดอร์ QBNex (โฟลเดอร์ที่มี `qb`
และ `crates/`) เหมือน QB64 ถ้าตั้ง `-s:exewithsource=true` จะไปอยู่ข้างไฟล์
source แทน path ของ source แบบ relative จะนับจากโฟลเดอร์ปัจจุบัน

| Switch | ความหมาย |
| --- | --- |
| `<file>` | ไฟล์ source (UTF-8 หรือไบต์ CP437 แบบที่ QBasic บันทึกไว้) |
| `-c` | คอมไพล์ (ค่าเริ่มต้น) |
| `-o <file>` | ไฟล์ executable ที่ต้องการ ถ้าชื่อลงท้ายด้วย `.qir` จะเขียน QIR text ของโปรแกรมแทน (สิ่งที่ [qb-web](../../../crates/web/README.md) ใช้รัน) |
| `--emit-ir` | เขียน QIR text แทน executable (`-o x.exe` จะกลายเป็น `x.qir`) |
| `-x` | คอมไพล์และส่งข้อความของคอมไพเลอร์ออกทาง console |
| `-w` | แสดงคำเตือน |
| `-Werror`, `--warnings-as-errors` | ถือว่าคำเตือนเป็น error |
| `-q` | โหมดเงียบ (ยังแสดงคำเตือนและ error) |
| `-m` | ข้อความของคอมไพเลอร์ไม่มีสี |
| `-k`, `--compact-errors` | diagnostic แบบสั้น (ค่าเริ่มต้นคือแบบละเอียด ส่วน `-d` เป็นชื่อเดิม) |
| `-e` | เปิด `OPTION _EXPLICIT` สำหรับการคอมไพล์ครั้งนี้ |
| `-s[:setting=true/false]` | ดูหรือแก้ค่าตั้งของคอมไพเลอร์ (`exewithsource` ส่วน `debuginfo` ยังรับอยู่แต่ไม่มีผล) |
| `-p`, `-z`, `--ir` | ยังรับไว้เพื่อให้สคริปต์เก่าใช้ได้ แต่ไม่เปลี่ยนอะไร (ไม่มี object ที่คอมไพล์ไว้ก่อน ไม่มีขั้น C++ และ QIR เป็น back end เดียว) |
| `-h`, `--help` · `-v`, `--version` · `-i`, `--info` · `-g`, `--examples` | วิธีใช้, เวอร์ชัน, ข้อมูลโปรเจกต์, ตัวอย่าง |

exit code 0 แปลว่าคอมไพล์ผ่าน diagnostic จะบอกชื่อไฟล์ บรรทัด คอลัมน์ และ
ยกบรรทัด source ที่มีปัญหามาให้ดู

## รันโปรแกรม

รัน executable ที่ build ได้ เช่น `hello.exe` หรือ `./hello`

- ถ้าเกิด runtime error ที่ไม่ได้ดักไว้ระหว่างรันใน console แบบโต้ตอบ
  จะขึ้นกล่องข้อความ "Unhandled Error #n ... Continue?" แบบเดิม แต่ถ้า
  output ถูก redirect หรือตั้ง `QBNEX_NO_ERROR_DIALOG=1` ไว้ จะพิมพ์หนึ่ง
  บรรทัดออกทาง stderr แล้วจบโปรแกรมด้วย exit code 1 แทน สคริปต์และ CI
  ควรตั้งตัวแปรนี้ไว้เสมอ
- `qb run file.bas` (รันบน VM โดยไม่เขียนไฟล์ executable) อยู่ในสถานะ
  **วางแผนไว้**

## โปรแกรมถูก build อย่างไร

คอมไพเลอร์จะเขียน QIR (ภาษากลางของ QBNex, `docs/dev/qir.md`)
แล้วต่อท้ายไว้ในสำเนาของ runner ภาษา Rust ที่ build ไว้แล้ว
(`crates/engine/exec/bin/<os>/qb-exec`) โดยไม่มีการเรียกคอมไพเลอร์ C/C++ หรือ
linker เลย runner จะแปลง QIR เป็น bytecode รันบน register VM และคอมไพล์
loop ที่ทำงานบ่อยเป็นโค้ดเครื่องด้วย Cranelift (`QB_JIT=off|eager|<threshold>`)
ถ้าโปรแกรมใช้คำสั่งที่ QIR back end ยังไม่รองรับ การ build จะหยุดพร้อมข้อความ
`QIR: unsupported: line N: ...`

executable แบบ native (**ทดลอง**): ถ้ามี native kit อยู่ข้าง runner
(`crates/engine/exec/bin/<os>/native/kit.txt` สร้างบน Windows ด้วย
`scripts/native-kit.ps1` และบน Linux/macOS ด้วย `scripts/native-kit.sh`)
คอมไพเลอร์จะลิงก์เป็น executable ปกติด้วย `rust-lld` แทนการต่อท้าย QIR
และถ้าลิงก์ไม่ผ่านจะกลับไปใช้สำเนาของ runner

## `qbnex fmt`: จัดรูปแบบโค้ด

```bash
qbnex fmt file.bas                 # พิมพ์ไฟล์ที่จัดรูปแบบแล้วออกมา
qbnex fmt -w *.bas                 # เขียนทับไฟล์เดิม
qbnex fmt --check src/*.bas        # exit 1 พร้อมรายชื่อไฟล์ที่จะถูกเปลี่ยน
type file.bas | qbnex fmt          # อ่าน stdin เขียน stdout (Windows; ระบบอื่นใช้ cat)
```

ตัวเลือก: `--keyword-case upper|lower|preserve` (ค่าเริ่มต้น `upper`),
`--indent N` (ค่าเริ่มต้น 4), `--tabs`, `--line-ending auto|lf|crlf`

formatter จะปรับตัวพิมพ์ของ keyword ย่อหน้าของ block (CASE เยื้องเข้าไป
หนึ่งระดับใน SELECT ส่วนเลขบรรทัดกับ label อยู่คอลัมน์ 0) และช่องว่าง
โดยไม่แตะ comment, string, รายการ `DATA`, metacommand และชื่อใน
`DECLARE LIBRARY` คง encoding ของไฟล์ไว้ (CP437 ยังเป็น CP437) และจัดซ้ำ
กี่รอบก็ได้ผลเท่าเดิม บน Windows ใช้ `qb.cmd fmt` ได้เช่นกัน

## `qbnex lsp`: language server

เปิด server ตามโปรโตคอล Language Server Protocol ทาง stdin/stdout สำหรับ
editor อื่นนอกจาก VS Code (Neovim, Helix, Emacs, Sublime LSP ...) ให้ตั้ง
editor ให้รัน `qbnex lsp` (หรือ `qb-lsp`) กับไฟล์ `.bas`, `.bi`, `.bm`
ความสามารถ: diagnostics, outline, ไปยังจุดประกาศและค้นหาจุดที่ใช้งาน
ข้ามไฟล์ `$INCLUDE`, hover อธิบาย keyword และ procedure ของไลบรารีมาตรฐาน,
completion และ formatting ดู[VS Code และเครื่องมือ](vscode.md)

## `qbnex dap`: debugger

เปิด server ตามโปรโตคอล Debug Adapter Protocol ทาง stdin/stdout โดยปกติ
editor จะเรียกให้เอง (VS Code กด F5) ตัวมันจะคอมไพล์โปรแกรมด้วย `qb`
แล้วรันบน VM พร้อม breakpoint, การ step และการดูค่าตัวแปร ดู
[VS Code และเครื่องมือ](vscode.md#debugger)

## ตัวแปรสภาพแวดล้อม

| ตัวแปร | ผล |
| --- | --- |
| `QBNEX_NO_ERROR_DIALOG=1` | runtime error ออกทาง stderr, exit code 1, ไม่มีกล่องข้อความ |
| `QB_JIT=off` / `eager` / `<n>` | runner ของ QIR: ไม่ใช้ JIT, คอมไพล์ทันที หรือคอมไพล์หลัง loop วนครบ n รอบ |
| `QB_GPU=off` / `verbose` | build ที่เปิดฟีเจอร์ GPU: ไม่ใช้ GPU เลย / log ว่าเลือกอะไร |
| `QBNEX_HEADLESS` | รันโปรแกรมกราฟิกโดยไม่เปิดหน้าต่าง (ใช้ตอนทดสอบ) |
| `QB_JIT_THREADS=<n>` | จำนวน thread ที่ JIT ใช้คอมไพล์ (ค่าเริ่มต้น: ทุก thread ของ CPU) |
| `QB_JIT_STATS=1` | พิมพ์สถิติของ JIT รวมถึงเวลาคอมไพล์ ออกทาง stderr |
| `QB_PROFILE=<file>` | เขียนฟังก์ชันและบรรทัดที่ใช้เวลามากที่สุด พร้อมจำนวนครั้งที่เรียกและเวลารวม ลงไฟล์เมื่อโปรแกรมจบ (สุ่มเก็บทุก 1 มิลลิวินาที ควรใช้คู่กับ `QB_JIT=off`) |
| `QB_EXEC_DISASM=1` | พิมพ์ bytecode ที่ runner แปลงจาก QIR ออกทาง stderr |
| `QB_GPU_FALLBACK=1` | build ที่เปิดฟีเจอร์ GPU: โปรแกรมใช้เฉพาะ adapter แบบ software เช่น WARP บน Windows (ทั้งการวาดแบบ offscreen และหน้าต่าง ถ้า adapter นั้นแสดงผลบนหน้าต่างได้) |
| `QBNEX_NATIVE_KIT=<dir>` | โฟลเดอร์ native kit ที่จะใช้แทน `native/` ข้าง runner |
| `QBNEX_INPUT_SCRIPT=<file>` | ป้อนเหตุการณ์คีย์บอร์ด เมาส์ และหน้าต่างจาก script (ใช้ตอนทดสอบ บรรทัดละหนึ่งคำสั่ง: `wait 100`, `key a`, `type Hi`, `click 1`, `close`, `snapshot`, `pixel x y`, ... รายการเต็มอยู่ต้นไฟล์ `crates/runtime/core/src/script.rs`) |
| `QBNEX_FB_CAPTURE=<file>` | เขียนต่อท้ายภาพของหน้าจอที่แสดงลงไฟล์ทุกครั้งที่ `_DISPLAY` และตอนจบโปรแกรม (ใช้ใน conformance test) |
| `QB_AUDIO_DEVICE=null` / `none` | อุปกรณ์เสียงเงียบที่เดินตามเวลาจริง / ไม่เปิดเสียงเลย (ใช้ตอนทดสอบ) |
| `QBNEX_LPRINT=<file>` | `LPRINT` และ `_PRINTIMAGE` เขียนต่อท้ายไฟล์ข้อความนี้แทนการพิมพ์ |
| `QB_GL=off` | ไม่สร้าง OpenGL context: `SUB _GL` จะไม่ถูกเรียก |
| `NO_COLOR` | ตั้งค่าใดก็ได้: คอมไพเลอร์พิมพ์โดยไม่มีสี เหมือน `-m` |
