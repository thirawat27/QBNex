# R11 — Backend ใหม่ที่ไม่มี C/C++: การออกแบบ + spike

> สถานะ: **design + spike** (2026-11-09, branch `claude/r11-emitter-5f5d4d` จาก `4aa6f1f`)
> เป้าหมายที่อนุมัติแล้ว: ลบ C/C++ ชิ้นสุดท้าย (`internal/c/qbx.cpp` + fragment C++ ที่ generate) โดย **compiler ยังเขียนด้วย BASIC (D10)**
> ป้ายหลักฐาน: `Measured` = รัน/วัดจริงในรอบนี้ · `Direct` = เห็นในโค้ด · `Estimate` = ประมาณการ

## 1. คำแนะนำ (TL;DR)

**เลือก B: compiler (BASIC) emit IR ข้อความ ("QIR") → รันด้วย runner Rust ที่ build ไว้แล้ว (`qb-exec`, ลิงก์ `qb-runtime`)**
และออกแบบ QIR ให้ lower ไป Cranelift ได้ เพื่อเพิ่ม **C เป็น tier ความเร็ว (JIT ใน `qb-exec`) ภายหลัง** — ไม่ใช่ทางเลือกแยก

เหตุผลหลัก (หลักฐานอยู่ §2–§3):

1. ผู้ใช้ **ไม่ต้องมี toolchain ใดๆ** ตอน compile: exe = สำเนา `qb-exec` + QIR ต่อท้าย (Measured: compile spike 45 ms–0.6 s, รันได้ขณะถอด MinGW ออกทั้งหมด)
2. control flow ที่ทำให้ A ยาก (GOTO, GOSUB/RETURN, ON ERROR/RESUME, ON TIMER) เป็นแค่ jump ใน IR แบบ flat — spike รองรับ GOSUB/RETURN และ `ON ERROR GOTO`/`RESUME NEXT` ใน main module แล้ว (กรณีที่ C++ path ปัจจุบัน compile ไม่ได้: L-01)
3. Cranelift (C) ต้องมี IR กลางอยู่ดี และถ้าทำ AOT ต้องมี linker; ทำ B ก่อนได้ IR + runtime binding ที่ C ใช้ต่อทันที
4. A (emit Rust) ต้องให้ผู้ใช้ติดตั้ง rustc (ใหญ่กว่า MinGW), compile ช้า และต้องแปลง goto เป็น state machine ซึ่งก็คือ interpreter dispatch ที่ให้ rustc compile อีกรอบ

ต้นทุนที่ยอมรับ: interpreter ช้ากว่า native บน loop แน่นๆ (Measured ~95 ns/รอบ ของ loop 13 op ใน spike ที่ยังไม่ optimize) → แก้ด้วย typed register VM (R11.5) และ Cranelift JIT tier (R11.6)

## 2. หลักฐานจาก ABI จริงของ C++ backend

ตรวจจาก `internal/c/qbx.cpp` (2214 บรรทัด) และ output จริงของ `qb.exe s.bas -z` (Direct/Measured):

| ข้อเท็จจริง | หลักฐาน | ผลต่อ backend ใหม่ |
| --- | --- | --- |
| Fragment ถูก `#include` เข้า **กลางตัวฟังก์ชัน** C++ | `qbx.cpp:2176-2193` (`maindata.txt`, `runline.txt`, `mainerr.txt`, `ontimerj.txt`, `main.txt` อยู่ใน `QBMAIN`); `#include "data1.txt"` / `"free1.txt"` / `"ret2.txt"` อยู่ในตัว SUB | ABI คือ "ข้อความ C++ ที่ต้องถูกวางในที่เฉพาะ" ไม่ใช่ interface — แปลงตรงเป็นภาษาอื่นไม่ได้ ต้อง lower ใหม่จาก semantics |
| GOSUB/RETURN = `return_point[]` + `goto` + `switch` ใน `retN.txt` | `return_point[next_return_point++]=1; goto LABEL_SUB1; RETURN_1:;` และ `switch(return_point[..]){case 1: goto RETURN_1;}` | ต้องการ computed goto — Rust ไม่มี |
| ทุก statement ห่อด้วย `do{ ... if(!qbevent)break;evnt(N);}while(r);` | `main.txt` | `evnt()` = จุด poll event + จุด retry ของ error (`r=1` → ทำ statement ซ้ำ = RESUME) |
| ON ERROR: `error()` → `fix_error()` → เรียก `QBMAIN(NULL)` ซ้อนแล้ว goto handler ผ่าน `mainerr.txt` | `qbx.cpp:2100-2130`, `README.md` §5.2 | re-entrant main + longjmp-ish control — ใน IR เป็นแค่ "pc = handler" |
| ตัวแปรเป็น pointer ที่ `mem_static_malloc` | `float *_SUB_QBNEX_TOPLEVEL_RUNTIME0_SINGLE_X=(float*)mem_static_malloc(4);` | layout ของตัวแปรต้องย้ายมาเป็น slot/cell ของ runtime |
| main module ถูกห่อเป็น SUB `QBNEX_TOPLEVEL_RUNTIME0` แบบ text lowering | `main.txt` (`SUB_QBNEX_TOPLEVEL_RUNTIME0();`) และ `evnt(19)` สำหรับบรรทัด 3 | ต้นเหตุ L-01; IR emitter ไม่ต้องผ่าน lowering นี้ |
| `internal/temp` มี **1138 ไฟล์** หลัง compile โปรแกรม 17 บรรทัด (ปนของเก่าจาก build compiler) | Measured | IR = ไฟล์เดียว |
| ทั้ง `qb.exe` และ exe ที่ C++ path สร้าง ต้องมี DLL ของ MinGW บน PATH | Measured: ถอด MinGW ออกจาก PATH → `qb.exe` และ `bench_cpp.exe` exit `0xC0000135` (DLL not found); exe จาก IR รันได้ปกติ | B แก้ปัญหาแจก exe ด้วย |

ตัวเลขที่วัด (Windows, MinGW g++ 15.2, rustc 1.98, runner build แบบ release):

| สิ่งที่วัด | C++ path (ปัจจุบัน) | QIR path (spike) |
| --- | --- | --- |
| compile `spike.bas` (libqb `.o` cache อุ่นแล้ว) | 5.2 s | 0.05–0.6 s (ส่วนใหญ่คือ startup ของ `qb.exe`) |
| ขนาด exe | 2.59 MB | 0.25 MB |
| รัน hello world (`PRINT 1`) | 1.11 s | 0.14 s |
| loop 5M รอบ `s& = s& + i& MOD 7` | 0.95 s (loop เองต่ำกว่า noise) | 0.56 s |
| loop 50M รอบ | 0.98 s | 4.9 s (~95 ns/รอบ, ~7 ns/op) |
| runner แบบ debug build (ก่อน resolve slot) | — | 20.8 s ที่ 5M รอบ (บทเรียน: ต้องใช้ release + slot) |
| front end ของ compiler กับ `compiler/qbnex.bas` (`-z`) | 22.6 s native | ถ้า interpret ด้วย VM ปัจจุบัน ≈ 40–100x → ไม่รับได้ → ต้องมี JIT ก่อน self-host (§8) |

## 3. ประเมินทางเลือก

| เกณฑ์ | A: emit Rust → rustc | **B: emit QIR → `qb-exec`** | C: emit IR → Cranelift tool |
| --- | --- | --- | --- |
| เครื่องผู้ใช้ต้องมี | rustc + cargo (~1 GB, ใหญ่กว่า MinGW) | **ไม่มีอะไร** | Cranelift tool ที่ build ไว้แล้ว + **linker** (AOT) หรือไม่มีเลย (JIT) |
| GOTO / GOSUB / RESUME / ON ERROR | ต้องแปลงทั้งโปรแกรมเป็น `loop { match state }` (labels ข้าม SUB, re-entry) — ก็คือ interpreter dispatch ที่ rustc compile อีกรอบ | jump ธรรมดา (spike ทำแล้ว) | block + branch ธรรมดา, ON ERROR ต้องการ landing pad |
| เวลา compile ของผู้ใช้ | ช้า (rustc บน match ขนาดใหญ่; โปรแกรม 10k บรรทัด → ฟังก์ชันยักษ์) | ms | ms–s |
| ความเร็วตอนรัน | native | interpreter (ต้องจูน) | native |
| งานฝั่ง runtime | qb-runtime เป็น crate dependency — ดี | qb-runtime ผ่าน Rust API — ดี | runtime ต้องเรียกผ่าน C ABI (`extern "C" qb_*`) |
| งานฝั่ง BASIC emitter | สูง (เขียน evaluate$ ใหม่เป็น Rust syntax + borrow/ownership) | สูง (เขียน evaluate$ ใหม่เป็น QIR) แต่ format ง่ายสุด | สูง + ต้องมี backend tool |
| ความเสี่ยงเฉพาะ | ผู้ใช้ไม่มี rustc; rustc version drift | ความเร็ว; AV flag exe ที่ต่อ payload | ต้องมี linker สำหรับ AOT; ขนาด Cranelift |
| ใช้ซ้ำกับทางเลือกอื่น | ต่ำ | **IR เดียวกันเป็น input ของ C** | ต้องมี IR อยู่ดี |

ข้อสรุป: B ให้ "ไม่มี C/C++ และไม่มี toolchain" ได้เร็วที่สุด และเป็นฐานของ C. A ตกเพราะย้าย dependency จาก g++ ไป rustc โดยไม่ได้ประโยชน์ด้าน control flow.

## 4. QIR

### 4.1 Format ของ spike (v0, Direct: `crates/engine/exec/src/lib.rs`)

ข้อความ 1 คำสั่ง/บรรทัด, header `QIR 0`, stack machine, string literal เป็น hex (ไม่มีปัญหา escape/CRLF/byte 0–255):

```
QIR 0
SHARED TOTAL&              ' DIM SHARED (ตัวแปร global)
LINE 7                     ' จุดเริ่ม statement: บรรทัดสำหรับ error, จุด RESUME, จุด poll event
PUSH I 1 | PUSH L 40000 | PUSH F 1.5 | PUSH D 0.1 | PUSH S 48656C6C6F
LOAD X! | STORE X! | REF X!   ' ชื่อมี suffix เสมอ (STORE แปลงชนิดตาม suffix, ~ชื่อ = temp ไม่มีชนิด)
OP ADD|SUB|MUL|DIV|IDIV|MOD|POW|AND|OR|XOR|EQ|NE|LT|LE|GT|GE
UN NEG|NOT
FN LEFT$ 2                 ' builtin + จำนวน argument
PRINT | PRINTZONE | PRINTNL
LABEL MAIN.REPORT | JMP x | JZ x | GOSUB x | RETURN
FORTEST                    ' (value, limit, step) -> -1/0 ตามเครื่องหมาย step
SUB GREET 1 WHO$ ... ENDSUB | CALL GREET 1
ONERROR MAIN.HANDLER | ONERROR 0 | RESUME 0|NEXT|label
END
```

ตัวอย่างจริงจาก `spike.bas` บรรทัด 8–10 (`FOR i% = 1 TO 5 STEP 2` / single-line IF / `NEXT`):

```
LINE 8
PUSH I 1
STORE I%
PUSH I 5
STORE ~F1.L
PUSH I 2
STORE ~F1.S
LABEL ~F1.T
LOAD I%
LOAD ~F1.L
LOAD ~F1.S
FORTEST
JZ ~F1.X
LINE 9
LOAD I%
PUSH I 3
OP EQ
JZ ~IF2.ELSE
PUSH S 7468726565
PRINT
PRINTNL
JMP ~IF2.END
LABEL ~IF2.ELSE
LOAD I%
PRINT
LABEL ~IF2.END
LINE 10
LOAD I%
LOAD ~F1.S
OP ADD
STORE I%
JMP ~F1.T
LABEL ~F1.X
```

Loader resolve label เป็น index และ intern ชื่อตัวแปรเป็น slot id ตอนโหลด (ไม่ hash ชื่อตอนรัน)

### 4.2 สิ่งที่ v1 ต้องเปลี่ยน (Estimate)

- **Typed, register-based**: compiler รู้ชนิดทุก expression อยู่แล้ว (`evaluate$` คืน `typ`) → emit `ADD.I32 r3, r1, r2` แทนการเช็กชนิดตอนรัน; ลด dispatch ~3–5 เท่าและ map ตรงไป Cranelift
- Binary encoding (หรือ text + cache) และ section แยก: constants, string pool, symbol table (สำหรับ `$DEBUG`/vwatch), DATA, ข้อมูล UDT layout
- Version/feature header เพื่อให้ runner ปฏิเสธ IR ที่ใหม่กว่า

## 5. การ lower control flow

| Construct | QIR (B) | หมายเหตุ/สถานะ |
| --- | --- | --- |
| line number / `label:` | `LABEL <scope>.<name>` (line number → `.L<n>`) | spike ✔; label ใน SUB เป็น local ตาม QB |
| `GOTO` | `JMP` | spike ✔ |
| `IF ... THEN ... ELSE` บรรทัดเดียว | `cond; JZ else; then; JMP end; LABEL else; else; LABEL end` | spike ✔ (statement ใน THEN ไม่มี `LINE` เพื่อไม่ให้ RESUME NEXT กระโดดเข้า ELSE) |
| block `IF/ELSEIF/ELSE/END IF` | ต่อ part: `JMP end; LABEL part_k; cond; JZ part_k+1` | spike ✔ |
| `SELECT CASE` | selector ลง temp `~Sn.V` ครั้งเดียว, แต่ละ CASE = compare (`IS`, `a TO b`, list) รวมด้วย `OP OR` + `JZ`; `EVERYCASE` ไม่กระโดดออกหลัง match, flag `~Sn.M` สำหรับ `CASE ELSE` | ✔ (รอบ 2) |
| `FOR/NEXT` | limit/step ประเมินครั้งเดียวลง temp, `FORTEST` เลือกทิศตามเครื่องหมาย step, `NEXT` = add + `JMP` | spike ✔ (ตรงกับ `fornext_step_negative` ของ C++) |
| `DO/LOOP [WHILE/UNTIL]`, `WHILE/WEND` | `LABEL T` + test หัว (`JZ/JNZ X`) หรือท้าย (`JNZ/JZ T`); `_CONTINUE` → label `C` | ✔ (รอบ 2) |
| `EXIT FOR/DO/SUB/FUNCTION` | `JMP` ไป label ปลาย block | `EXIT SUB` ✔ |
| `GOSUB`/`RETURN` | runner เก็บ stack `(return pc, frame depth)`; `RETURN` ที่ไม่ตรง frame → error 3 | spike ✔ |
| `RETURN label` | pop แล้ว `JMP label` | ยังไม่ทำ |
| `ON n GOTO/GOSUB` | selector ลง temp แล้วชุด `EQ k` + `JZ` (นอกช่วง = ผ่านไป); `GOSUB` ตามด้วย `JMP end` | ✔ (รอบ 2) |
| `SUB`/`FUNCTION` | frame ใหม่, BYREF = `REF` cell เดิม, ค่าคงที่/expression = cell ใหม่; FUNCTION คืนค่าผ่าน slot ชื่อฟังก์ชัน; recursion ได้ (locals คืนเมื่อออกจาก frame) | ✔ ทั้ง SUB/FUNCTION/recursion/`STATIC` (รอบ 2; STATIC locals อยู่ใน heap ของ runner) |
| `ON ERROR GOTO`/`RESUME`/`RESUME NEXT`/`RESUME label` | error → บันทึก `(pc ของ LINE ปัจจุบัน, line)` แล้ว `pc = handler`; `RESUME` = pc เดิม, `NEXT` = `LINE` ถัดไป | ✔ รวม error ใน SUB: runner **ไม่ unwind frame**, รัน handler ด้วยมุมมองตัวแปรระดับ module (`view = 0`) แล้ว `RESUME` กลับ frame เดิม (ตาม QBasic); `ERROR n` ✔ |
| `ERR`, `ERL`, `ERROR n`, `_ERRORLINE` | runner state | `ERR` ✔ |
| `ON TIMER/KEY/STRIG GOSUB` + `TIMER ON/OFF/STOP` | thread ของ timer ตั้ง flag; runner poll flag ที่ทุก `LINE` (เท่ากับ `qbevent`/`evnt`) แล้วทำ implicit `GOSUB`; ไม่ปล่อย event ซ้อนระหว่าง handler/ระหว่าง error handling | ยังไม่ทำ; จุด poll มีแล้ว (`LINE`) |
| `END`/`SYSTEM`/`STOP` | `END` op + shutdown ของ runtime (`END` ใน console ของ C++ พิมพ์ "Press any key" — ต้องทำตาม) | ✔ (ยังไม่พิมพ์ข้อความ "Press any key") |
| `CHAIN`/`RUN`/`COMMON` | runner โหลด QIR ใหม่ (จาก exe อื่นหรือไฟล์) ส่ง COMMON ผ่านหน่วยความจำ | ยังไม่ทำ |
| `CLEAR` | reset cell/stack/gosub/error state (ตาม `sub_clear` ใน `qbx.cpp:1137-1160`) | ยังไม่ทำ |

## 6. ข้อมูล: strings, arrays, UDT → qb-runtime

| ชนิด | รูปแบบใน runner | qb-runtime |
| --- | --- | --- |
| ตัวเลข `_BYTE`..`_INTEGER64`, `_UNSIGNED`, `SINGLE`, `DOUBLE`, `_FLOAT`, `_OFFSET` | value มี tag; SINGLE ถือ double precision จนกว่าจะ store/print (spike พบว่า `y# = 1 / 3` ของ C++ path เก็บ 53 bit) | rounding `qb_runtime::round`, `STR$`/PRINT `qb_runtime::numfmt` (spike ใช้อยู่), `_FLOAT` ใช้ F80 ของ `round.rs` |
| `STRING` ความยาวแปรผัน | handle ไปยัง string ใน heap ของ qb-runtime (ต้องมีที่อยู่คงที่สำหรับ `SADD`/`VARPTR`) | R3 ต้องให้ Rust API + C ABI ที่ไม่ผูกกับ `qbs*` ของ C++ |
| `STRING * n` / `FIELD` | buffer ยาวคงที่ใน cell/UDT | R3/R4 |
| array | descriptor เดียวกับที่ compiler คำนวณอยู่แล้ว (base, element size, จำนวนมิติ, lbound/ubound, flags `$DYNAMIC`/`$STATIC`); op `DIMA`, `REDIM [_PRESERVE]`, `ERASE`, `LOADE/STOREE <n index>`, error 9 เมื่อเกินขอบ | layout ใน cmem/heap ของ qb-runtime |
| UDT | byte buffer ขนาดตามที่ `udts.bas` คำนวณ (packed แบบ QB64); `LOADF/STOREF <offset> <type>` | ต้องตรงกับไฟล์ `GET/PUT` และ `_MEM` (R4 `qb_runtime::mem` มีแล้วใน branch R4) |
| `DATA/READ/RESTORE` | DATA section ของ QIR + pointer | RESTORE label แก้ L-03 ได้ในตัว |
| `VARPTR/VARSEG/PEEK/POKE/DEF SEG` | emulate cmem 1 MB ใน runner | ย้าย logic `varseg_dblock_check` จาก `qbx.cpp:1090-1113` |

## 7. Packaging + bootstrap

- spike: `out.exe = qb-exec.exe + QIR + u64 LE length + "QBNEXIR0"`; runner อ่านไฟล์ตัวเองตอนเริ่ม (`crates/engine/exec/src/main.rs`). ใช้ได้ทุก OS (Linux/macOS `chmod +x` จาก compiler)
- ระยะยาว: ใส่ QIR ใน PE resource / ELF section (ลดโอกาส anti-virus มองว่าเป็น packer) และ sign runner ตอน release
- runner ที่แจกอยู่ที่ `crates/engine/exec/bin/<windows|linux|macos>/qb-exec[.exe]` (gitignored เหมือน `libqb_runtime.a`); CMake target `qb-exec` (ALL และ dependency ของ `qb-selfhost`) เรียก `crates/engine/exec/os/<os>/setup_build.*` — Windows build ด้วย target `x86_64-pc-windows-gnu` เหมือน qb-runtime, import แค่ DLL ของระบบ (KERNEL32, UCRT `api-ms-win-crt-*`, ntdll, USERENV, WS2_32, bcryptprimitives); ยังไม่ได้ใส่ใน release bundle
- **Self-host:** compiler (`compiler/qbnex.bas`) ก็คือโปรแกรม BASIC → เมื่อ QIR ครอบคลุมภาษาที่ compiler ใช้, `qb.exe` = `qb-exec` (+JIT) + `qbnex.qir` และ snapshot bootstrap = `internal/source/qbnex.qir` ไฟล์เดียว แทน C++ 818 ไฟล์ (แก้ปัญหา refresh snapshot ของ L-00 ในเชิงโครงสร้าง แต่ต้องแก้ fixpoint ของ L-00 ก่อน)

## 8. Migration steps

| Step | งาน | Exit gate |
| --- | --- | --- |
| R11.0 (เสร็จ: spike นี้) | QIR v0, `qb-exec`, `--ir` emitter แยก (tokenizer + parser เล็กของตัวเอง) | `cargo test -p qb-exec`: 18 unit + 3 e2e, e2e differential เทียบ C++ path |
| R11.1 | QIR v1 spec (typed, register) + emitter **ใน pipeline จริง**: main pass ของ `qbnex.bas` มีชั้น `Emit*` แทน `PRINT #12, ...` ตรงๆ; `evaluate$` คืน operand QIR เมื่อ `IrEmitMode` (ใช้ symbol table, type, UDT, `$INCLUDE`, `$IF`, CONST, DEFxxx เดิมทั้งหมด) แล้วลบ parser ของ spike | corpus `tests/conformance/` ที่ใช้แค่ core statements ผ่านทั้งสอง backend |
| R11.2 (บางส่วน: CMake + scripts ✔ บน Windows) | Build/packaging: runner ใน CMake + setup scripts + release bundle; flag `--ir` ยัง opt-in | CI build runner ทั้ง 3 OS |
| R11.3 | ครอบคลุมภาษา: arrays, UDT, FUNCTION, SELECT, DO, DATA, file I/O, events, CHAIN/COMMON, `_MEM`, metacommands; builtin 405 regid ผ่านตาราง dispatch ไปยัง qb-runtime | conformance ทั้งหมดผ่าน **differential** (C++ vs QIR) + smoke tests |
| R11.4 | `DECLARE LIBRARY` / `DECLARE DYNAMIC LIBRARY`: dynamic library = dlopen + call thunk ตาม signature; static C header = deprecate หรือคง C++ path เป็น opt-in | ตัดสินใจ (ต้องการ owner) |
| R11.5 | Performance: typed register VM, superinstructions, inline cache ของ builtin | loop benchmark ≤ 10x ของ C++ |
| R11.6 | Cranelift JIT tier ใน `qb-exec` (ไม่ต้องใช้ linker) สำหรับ SUB/loop ที่ร้อน และสำหรับรัน compiler เอง | front end ของ compiler กับ `qbnex.bas` ≤ 2x ของ native ปัจจุบัน |
| R11.7 | Self-host ผ่าน QIR, QIR เป็น default, ลบ `qbx.cpp`, `internal/source` C++ snapshot, `makeline_*`, การดาวน์โหลด MinGW, ส่วน C++ emitter | build ไม่มี C/C++ compiler; conformance + smoke ผ่าน |

## 9. Effort estimate (Estimate, คนเดียวเต็มเวลา)

| Step | สัปดาห์ |
| --- | --- |
| R11.1 emitter ใน pipeline จริง (ส่วนใหญ่คือ `evaluate$`/`semantics.bas` + ~960 `PRINT #n`) | 8–12 |
| R11.2 build/packaging | 1–2 |
| R11.3 ครอบคลุมภาษา + builtin dispatch (ขึ้นกับว่า R3–R9 export Rust API ครบแค่ไหน) | 8–12 |
| R11.4 FFI | 2–4 |
| R11.5 VM performance | 3–4 |
| R11.6 Cranelift JIT | 6–8 |
| R11.7 self-host + ลบ C++ | 3–4 |
| **รวม** | **31–46** |

## 10. ความเสี่ยง

| ความเสี่ยง | ผล | วิธีลด |
| --- | --- | --- |
| Interpreter ช้า (Measured ~7 ns/op แบบ untyped) | เกม/โปรแกรมคำนวณช้ากว่า C++ หลายสิบเท่า; compiler ที่ interpret ช้าเกินใช้ | R11.5 typed VM, R11.6 JIT; คง C++ path จน gate ผ่าน |
| `DECLARE LIBRARY` กับ header C | ฟีเจอร์ที่ต้องมี C compiler หายไป | R11.4: dynamic library ผ่าน thunk; static header = deviation ที่ต้องอนุมัติ |
| Emitter สองชุดคู่กันช่วง migration | behaviour drift | differential test (spike ทำแล้ว: `e2e.rs` เทียบ output สอง backend) ขยายเป็นทุก case ใน `tests/conformance/` |
| `evaluate$` คืนข้อความ C++ ฝังลึก | refactor ใหญ่, อาจชน L-00 (compiler รุ่นใหม่ self-host ไม่ได้) | ทำบน stage0 ที่ build ได้; แก้ L-00 ก่อน R11.7 |
| ความหมายตัวเลขละเอียด (precision ของ SINGLE ที่ C++ คำนวณแบบ long double/float, integer overflow ที่ C++ wrap) | output ต่างทีละ digit | differential corpus แบบสุ่ม (แบบ `diff_*` ของ R2) บน expression ที่ generate |
| Exe ต่อ payload ถูก anti-virus จับ | ผู้ใช้รันไม่ได้ | PE resource section + code signing |
| Runtime ยังอยู่ใน C++ (`libqb.cpp`) ระหว่าง R3–R9 | runner Rust เรียก C++ ไม่ได้ถ้าไม่ลิงก์ libqb | runner ใช้เฉพาะ subsystem ที่ port แล้ว; R11.3 รอ R3–R9 |
| Thread model (QBMAIN, TIMERTHREAD, GLUT/winit main thread) | event/หน้าจอผิดเวลา | runner รันโปรแกรมบน thread แยกเหมือน `QBMAIN`; poll event ที่ `LINE` (R8 กำลังย้ายหน้าต่างไป winit) |

## 11. qb-runtime C-ABI ที่ backend ใหม่ต้องใช้ (สำหรับ R3–R9)

runner ลิงก์ `qb-runtime` เป็น rlib จึงเรียก Rust API ได้ตรง แต่ **JIT tier (R11.6) เรียก runtime ผ่าน `extern "C"` เท่านั้น** และ C++ path ยังใช้อยู่ช่วง migration ดังนั้นทุก builtin ต้องมีทั้งสองแบบ ข้อตกลง:

1. ทุก builtin = `pub fn` ใน module ของ qb-runtime (ชนิด Rust ธรรมดา) **และ** `#[no_mangle] extern "C" qb_<subsystem>_<name>` ที่รับแค่ scalar / `ptr+len` / handle — ห้ามมีชนิด C++ (`qbs*`, `img_struct*`) ใน signature
2. ห้ามพึ่ง global ของ `qbx.cpp`/`libqb.cpp` (`new_error`, `error_goto_line`, `return_point`, `qbevent`, `cmem`, `write_page`): error ต้องรายงานผ่านค่าคืน (`int32` error code) หรือ hook `qb_set_error_hook` ที่ runner/libqb ตั้ง
3. state ระดับโปรแกรม (หน้าจอปัจจุบัน, ไฟล์ที่เปิด, RND seed) อยู่ใน qb-runtime และ reset ได้ด้วยฟังก์ชันเดียว (`CLEAR`/`RUN`)
4. เรียกได้จาก thread ที่รันโปรแกรม (ไม่ใช่ main thread) — งานที่ต้องอยู่ main thread ให้ qb-runtime ส่งต่อเอง

ที่ backend ต้องใช้แน่นอน (ชื่อที่มีแล้วบน `rust-port`/branch R4–R9 ตอนที่ตรวจ; ชื่อ *ตัวเอียง* = ยังไม่มี ต้องเพิ่ม):

| Subsystem | ใช้แล้วใน spike | ต้องมีสำหรับ R11.3 |
| --- | --- | --- |
| R1/R2 numeric | `qb_qbr_f64` (rounding ตอน store), `str_i64`/`str_f32`/`str_f64` (`qb_str_*`) | `qb_qbr_f80`, `qb_fistp_*`, `qb_val`, `qb_f80_to_f64`, `qb_rnd`, `qb_randomize`, `qb_timer`, `qb_abi_version` |
| R3 strings | (spike ใช้ `Vec<u8>` ชั่วคราว) | *`qb_str_new/free/concat/left/right/mid/mid_set/instr/ucase/lcase/ltrim/rtrim/space/string/chr/asc/compare/len`*, *`qb_print_using`* |
| R4 files + `_MEM` | — | `qb_mem*`, `qb_new_mem_lock`, `qb_free_mem_lock` (มีใน R4); *`qb_file_open/close/print/input/line_input/get/put/eof/lof/loc/seek/freefile/kill/name/mkdir/files`* |
| R5 graphics | — | `qb_gfx_pset/line/circle/paint/color/palette/view/window/point/pmap/boxfill` (มีใน R5); *`qb_gfx_screen/cls/locate/print_text`* |
| R6 audio | — | `qb_snd_beep/sound/play/open/close/play_file/stop/vol/...`, `qb_synth_*` (มีใน R6) |
| R7 fonts/images | — | `qb_font_load/free/width/render_utf32`, `qb_image_decode` (มีใน R7) |
| R8 window/input | — | `qb_window_*`, `qb_keyhit_pop/push`, `qb_keyheld*`, `qb_input_lock/unlock` (มีใน R8); *`qb_inkey`*, *`qb_event_poll`* (สำหรับ poll ที่ `LINE`) |
| R9 sys | — | `qb_get_ticks`, `qb_day_ms`, `qb_time_init`, `qb_tcp_*`, `qb_clipboard_*`, `qb_screen*` (มีใน R9); *`qb_command`*, *`qb_environ`*, *`qb_shell`*, *`qb_sleep`*, *`qb_date/time`* |
| runner-owned (ใหม่) | GOSUB stack, ON ERROR state, PRINT column | *`qb_set_error_hook`*, *`qb_console_write`* (PRINT + column tracking ร่วมกับ `LOCATE`/`POS`), *`qb_event_set/take`* สำหรับ ON TIMER/KEY/STRIG |

## 12. Spike (สิ่งที่ส่งมอบในรอบนี้)

| ไฟล์ | หน้าที่ |
| --- | --- |
| `crates/engine/exec/src/lib.rs` | loader (resolve label/slot) + interpreter ของ QIR v0, ใช้ `qb_runtime::numfmt` และ `qb_runtime::round` |
| `crates/engine/exec/src/main.rs` | runner: อ่าน payload ท้ายไฟล์ตัวเอง หรือ `qb-exec file.qir` |
| `crates/engine/exec/tests/vm.rs` | 18 unit tests (เขียนก่อน, เห็น fail กับ stub ก่อน implement) |
| `crates/engine/exec/tests/e2e.rs` + `fixtures/` | `.bas` → `qb.exe --ir` → exe → output; `spike.bas` เทียบกับ C++ path (differential), `onerror.bas` (L-01 ใน C++), บรรทัดที่ไม่รองรับต้องแจ้งเลขบรรทัด |
| `compiler/utilities/ir_emit.bas`, `ir_state.bas` | emitter ใน BASIC: tokenizer, recursive-descent expression, PRINT/LET/IF/FOR/GOTO/GOSUB/SUB/CALL/DIM/ON ERROR/RESUME/END/SYSTEM |
| `compiler/utilities/cli.bas`, `compiler/qbnex.bas`, `compiler/includes/bootstrap.bas` | flag `--ir` (`-ir`, `--emit-ir`), include 2 บรรทัด, hook 1 บรรทัดหลังอ่าน command line — C++ path เป็น default เหมือนเดิม |

วิธีรัน:

```bash
cmake --build --preset windows-mingw-selfhost
```

```bash
cargo test -p qb-exec
```

```bash
cargo build --release -p qb-exec && cp target/release/qb-exec.exe crates/engine/exec/bin/windows/ && ./qb.exe prog.bas --ir -o prog.exe
```

### 12.1 รอบ 2: ขยาย coverage (วัดด้วย conformance corpus)

conformance runner รับ `--qb-arg <flag>` แล้ว จึงรัน corpus เดิมผ่าน IR backend ได้:

```bash
cargo run --release -p qbnex-conformance -- --qb-arg --ir
```

| | ผ่าน | xfail | fail |
| --- | --- | --- | --- |
| C++ backend (default) | 26 | 7 | 0 |
| QIR รอบ 1 | 1 | 7 | 25 |
| **QIR รอบ 2** | **15** | 7 | 11 |

เพิ่มในรอบ 2 (TDD: `tests/vm_lang.rs` 19 tests, `tests/vm_data.rs` 7 tests เขียนก่อนและเห็น fail ก่อน implement):
DO/LOOP, WHILE/WEND, EXIT FOR/DO/SUB/FUNCTION, `_CONTINUE`, SELECT CASE [EVERYCASE], ON n GOTO/GOSUB, `IF x THEN <line>`, FUNCTION + recursion, `SUB ... STATIC`, array หลายมิติ (DIM/REDIM [_PRESERVE]/LBOUND/UBOUND/auto-DIM 0..10/error 9), SWAP, BYREF ของ array element, `MID$` statement, ERROR n, error ใน SUB → handler ใน main, DEFINT/DEFLNG/DEFSNG/DEFDBL/DEFSTR, OPTION BASE, `&H/&O/&B`, EQV/IMP, DATA/READ/RESTORE [label], RND/RANDOMIZE [USING] (state จาก `qb_runtime::rnd`), VAL (`qb_runtime::val`), MKI$/MKL$/MKS$/MKD$/CVx, MKSMBF$/MKDMBF$/CVSMBF/CVDMBF (`qb_runtime::mbf`), INSTR, LTRIM$/RTRIM$/_TRIM$, SPACE$, STRING$, HEX$/OCT$, ABS/SGN/SQR/LOG/EXP/SIN/COS/TAN/ATN/INT/FIX/CINT/CLNG/CSNG/CDBL/_ROUND

case ที่ C++ path ทำไม่ได้ (L-01 `ON ERROR` ใน main, L-03 `RESTORE label`, L-04 relational ใน PRINT) รันถูกต้องใน QIR — เก็บเป็น `tests/fixtures/legacy_defects/*.bas` + `.out` (ตรวจด้วยมือเทียบ QBasic) ใน e2e

ที่ยัง fail (11): `TYPE` (UDT: `udt`, `binary_random`), file I/O (`sequential`), `PRINT USING` (`print_using`, `rnd_sequence`, `val_parse`), `_INTEGER64`/`_FLOAT`/`&&` (`rounding_limits`, `str_format`, `print_format`, `mkcv`), `STRING * n` (`fixed_length`) — ทั้งหมดรอ R3/R4 (string engine, file I/O, PRINT USING) และชนิดตัวเลข 64/80 bit ใน runner

ข้อจำกัดของ spike (ตั้งใจ): parser ของ emitter แยกจาก pipeline จริง (ไม่มี `$INCLUDE`, CONST, UDT); ตัวเลขเป็น value แบบ tag เช็กตอนรัน; `END` ไม่พิมพ์ "Press any key"; e2e ใช้ runner แบบ debug build; array ที่ DIM ใน SUB ไม่ถูกคืนหน่วยความจำเมื่อออกจาก SUB
