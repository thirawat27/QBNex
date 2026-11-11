# VS Code, formatter, language server, debugger

[สารบัญคู่มือ](../README.md) · [English](../en/vscode.md)

## ส่วนขยาย VS Code (ใช้ได้แล้ว)

ส่วนขยายอยู่ที่ `packaging/vscode` build และติดตั้งดังนี้

```bash
cd packaging/vscode
npm install
npx vsce package                      # ได้ qbnex-2.0.0.vsix
code --install-extension qbnex-2.0.0.vsix
```

ต้องมี `qb` (หรือตั้ง `qbnex.compilerPath`) และสำหรับความสามารถด้านภาษา
ต้องมี `qb-lsp` กับ `qb-dap` (`cargo build --release -p qb-lsp -p qb-dap`
แล้ววางไว้ใน `PATH` หรือตั้ง `qbnex.lsp.path` / `qbnex.dap.path` หรือแนบไป
กับแพ็กเกจด้วย `npm run bundle-server` ก่อน package)

| ความสามารถ | ทำอย่างไร |
| --- | --- |
| ไฮไลต์โค้ด | TextMate grammar ที่สร้างจากตาราง keyword ชุดเดียวกับเครื่องมืออื่น (ครบทุก built-in ของคอมไพเลอร์) |
| Snippet | `if`, `for`, `do`, `select`, `sub`, `function`, `type`, `openin`, `onerror`, `gameloop`, `include` ... (17 ชุด) |
| Diagnostics | ขณะพิมพ์: error ของ syntax และ block (`FOR` ที่ไม่มี `NEXT` ...), ไฟล์ `$INCLUDE` ที่หาไม่เจอ |
| Outline, ไปยังจุดประกาศ, ค้นหาจุดที่ใช้ | SUB, FUNCTION, TYPE และ field, CONST, label, เลขบรรทัด ข้ามไฟล์ `$INCLUDE` |
| Hover, completion | keyword พร้อม syntax, procedure ของไลบรารีมาตรฐานพร้อมชื่อโมดูล, สัญลักษณ์ที่เขียนเอง |
| จัดรูปแบบเมื่อบันทึก | เปิดเป็นค่าเริ่มต้นสำหรับไฟล์ QBasic (ปิดด้วย `"[qbasic]": {"editor.formatOnSave": false}`) |
| `QBNex: Run` (Ctrl+F5, ปุ่ม play) | บันทึก คอมไพล์ด้วย `qb` แล้วรันใน terminal ถ้าตั้ง `"qbnex.runMode": "qbRun"` จะใช้ `qb run` (วางแผนไว้) |
| `QBNex: Compile` (Ctrl+Shift+B) | คอมไพล์ ผลลัพธ์อยู่ในแผง *QBNex* |
| Debug (F5) | ดูด้านล่าง |

## Debugger

กด F5 บนไฟล์ `.bas` ได้เลย ไม่ต้องมี `launch.json` ตัว `qb-dap` จะคอมไพล์
โปรแกรมด้วย `qb` แล้วรันบน VM ของ qb-exec (ปิด JIT) ให้ใช้งานได้ดังนี้

- breakpoint ตามบรรทัดในไฟล์ที่ launch (บรรทัดที่ไม่มี statement จะเลื่อนไป
  บรรทัดถัดไป), pause, continue, step over / into / out (ทีละ statement)
- call stack ของ frame SUB/FUNCTION
- ตัวแปรของแต่ละ frame และตัวแปร global: ตัวเลข, string, array (แสดงได้
  สูงสุด 1000 สมาชิก), field ของ TYPE กางดูได้
- watch และ hover แบบ `name`, `name.field`, `name(i, j)`
- output ของโปรแกรมใน Debug Console

ตัวเลือกใน `launch.json`: `program`, `qir` (ใช้ไฟล์ QIR แทนการคอมไพล์),
`compiler`, `cwd`, `stopOnEntry`, `input` (ไฟล์ที่โปรแกรมอ่านเป็น
standard input)

ข้อจำกัด: ยังตั้ง breakpoint ในไฟล์ที่ `$INCLUDE` เข้ามาไม่ได้ (QIR ยังไม่มี
ข้อมูลไฟล์) ถ้าไม่ระบุ `input` คำสั่ง INPUT จะเจอ end of file, GOSUB ไม่เพิ่ม
frame ใน stack และ watch ใช้ได้เฉพาะเส้นทางของตัวแปร

## Editor อื่น

`qbnex lsp` และ `qbnex dap` ใช้โปรโตคอลมาตรฐานผ่าน stdin/stdout
ตัวอย่างสำหรับ Neovim (แบบ nvim-lspconfig)

```lua
vim.lsp.config('qbnex', { cmd = { 'qbnex', 'lsp' }, filetypes = { 'basic', 'qbasic' } })
vim.lsp.enable('qbnex')
```

formatter ใช้แยกเดี่ยวได้ด้วย: `qbnex fmt -w file.bas`
(ดู[คำสั่ง](commands.md#qbnex-fmt-จัดรูปแบบโค้ด))
