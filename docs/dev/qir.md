# QIR — spec

The text format starts with the line `QIR 1`: `1` is the format version that the
runner checks, not part of the name.

> สถานะ: **normative** สำหรับ VM และ Cranelift JIT ใน `crates/engine/exec`/`crates/engine/jit` และ emitter ใน `compiler/utilities/ir_*.bas`
> โค้ดอ้างอิง: `crates/compiler/ir` (ชนิดข้อมูล, parser, printer, validator, ตาราง builtin). ถ้าเอกสารนี้กับ crate ขัดกัน **crate ชนะ** และต้องแก้เอกสาร

## 1. ภาพรวม

- ข้อความ ASCII, 1 รายการต่อบรรทัด, token คั่นด้วย space, บรรทัดว่าง/ขึ้นต้นด้วย `;` = comment
- บรรทัดแรกที่ไม่ใช่ comment ต้องเป็น `QIR 1`. Executor ต้องปฏิเสธ version ที่ใหม่กว่าที่ตัวเองรู้จัก
- โปรแกรม = **module** (types, globals, DATA) + **functions**. `func main` คือ main module (บังคับมี, ไม่มี param, ไม่คืนค่า)
- ภายใน function: **register** `%n` แบบ typed (ชนิดคงที่ตลอด function), **variable** `$name` = storage ที่มี address
- ชนิดของทุก op อยู่ใน opcode (`add.i32`) → executor ไม่ต้องเช็กชนิดตอนรัน; `qb_ir::validate` รับประกันความสอดคล้องของชนิด
- ทุก control flow เป็น jump ไป label ภายใน function เดียวกัน (ยกเว้น ON ERROR/RESUME ดู §7)

Canonical text = output ของ `qb_ir::print` (parse กลับได้ module เท่าเดิม). Parser ยอมรับ `global`/`data`/`datalabel`/`meta` ที่ใดก็ได้ (รวมถึงในตัว function — emitter เขียนตามลำดับที่เจอใน source) และ `param`/`local`/`static` ที่ใดก็ได้ใน function

## 2. ชนิด

| ชนิด register | QB | ขนาดใน memory |
| --- | --- | --- |
| `i8` `u8` | `_BYTE`, `_UNSIGNED _BYTE` | 1 |
| `i16` `u16` | `INTEGER`, `_UNSIGNED INTEGER` | 2 |
| `i32` `u32` | `LONG`, `_UNSIGNED LONG` | 4 |
| `i64` `u64` | `_INTEGER64`, `_UNSIGNED _INTEGER64` | 8 |
| `f32` | `SINGLE` | 4 |
| `f64` | `DOUBLE` | 8 |
| `f80` | `_FLOAT` (x87 80-bit; D12: ต้องตรงทุก bit) | 32 (เหมือน QB64) |
| `str` | `STRING` ความยาวแปรผัน — handle; executor เป็นเจ้าของ bytes | 8 |
| `ptr` | address ของ storage (`_OFFSET` ใช้ `i64`/`u64`) | 8 |

ชนิด storage (ใช้ได้กับ variable/field/element เท่านั้น ไม่ใช่ register):
- `fstr:N` = `STRING * N` (N byte, เติม space) — `load.str`/`store.str` อ่าน/เขียนเป็น `str` (store ตัด/เติม space)
- `udt:NAME` = UDT ที่ประกาศด้วย `type`

`_BIT` ไม่อยู่ใน v1 (compiler แจ้ง error)

## 3. Module

```
QIR 1
meta console only            ; key + ข้อความที่เหลือ (ดู §3.1)
type POINT 8                  ; UDT: ชื่อ, ขนาด byte (packed ตาม udts.bas)
field X i32 0                 ; ชื่อ, ชนิด storage, offset
field Y i32 4
endtype
global $TOTAL& i32            ; ตัวแปรระดับ module (DIM SHARED และตัวแปรใน main ทั้งหมด)
global $A%() array i16        ; array = descriptor, ยังไม่ DIM
datalabel MYDATA              ; RESTORE MYDATA ชี้ไปที่ DATA ตัวถัดไป
data x3132                    ; DATA 1 รายการ = ข้อความดิบ (hex) ตามที่เขียนใน source หลังตัด quote
func main
...
endfunc
func FACT -> i32              ; FUNCTION คืนค่า i32
param $N& i32 byval           ; param ตามลำดับ argument
param $A& i32 byref           ; byref: caller ส่ง ptr
param $V%() array i16         ; array param: caller ส่ง ptr ของ descriptor (aref)
local $T# f64                 ; local: สร้างใหม่ทุกครั้งที่เข้า, คืนเมื่อ ret
static $C& i32                ; static: คงค่าข้ามการเรียก
...
endfunc
```

- ตัวแปรทุกตัวเริ่มที่ 0 / string ว่าง / fstr เป็น space ทั้งหมด / UDT เป็น 0 (field fstr เป็น 0 byte เหมือน QB64)
- ชื่อ variable ขึ้นต้น `$` และไม่มี space; ชื่อ label/func/type เป็น token ที่ไม่ขึ้นต้นด้วย `%`/`$`
- การค้นชื่อ variable ใน function: param/local/static ก่อน แล้วจึง global
- ตัวแปรใน main module เป็น `global` ทั้งหมด (compiler ห้าม SUB เห็นตัวที่ไม่ SHARED เอง) → handler ของ ON ERROR ที่รันขณะอยู่ใน SUB ใช้ตัวแปรของ main ได้ตรงๆ

### 3.1 meta ที่รู้จัก

| key | ความหมาย |
| --- | --- |
| `console only` / `console` | `$CONSOLE:ONLY` / `$CONSOLE` |
| `source <hex>` | ชื่อไฟล์ต้นฉบับ (สำหรับข้อความ error) |
| `screenhide` | `$SCREENHIDE` (no window: `_DISPLAY` leaves `_AUTODISPLAY` on) |
| `endkey 0` | `END` ไม่พิมพ์ "Press any key to continue" |

executor ต้องข้าม meta ที่ไม่รู้จัก

## 4. Instructions

รูปแบบ: `opcode[.T[.T2]] operands...`; ถ้า op ให้ค่า register ปลายทางมาก่อน. `T` = ชนิด register. ทุก operand ที่เป็นค่าคือ register (ไม่มี immediate ยกเว้น `const`, offset/ขนาด byte และจำนวนมิติ)

### 4.1 ค่า

| op | ความหมาย |
| --- | --- |
| `const.T %d lit` | int: ฐานสิบ (ต้องอยู่ในช่วงของ T), float: ฐานสิบ / `inf` / `-inf` / `nan` / bit ตรง `0x` (f32 8 hex, f64 16 hex, f80 20 hex = sign+exp 4 + mantissa 16), str: `x<hex bytes>` (`x` = ว่าง), ptr: `0` เท่านั้น |
| `mov.T %d %s` | copy (str: copy ค่า) |

ทศนิยมของ f32 แปลงตรงเป็น f32 (correctly rounded); ของ f80 แปลงตรงเป็น 80-bit

### 4.2 คณิต (T ต้องเป็นตัวเลข)

| op | ชนิด | ความหมาย |
| --- | --- | --- |
| `add` `sub` `mul` `.T %d %a %b` | int, float | int: wrap แบบ two's complement (เหมือน C++ path) |
| `div.T` | float เท่านั้น | IEEE (หารศูนย์ได้ inf/nan; QB error 11 ของ `/` emitter เช็กเองถ้าต้องการ) |
| `idiv.T` `mod.T` | int เท่านั้น | ตัดทศนิยมเข้าหา 0 / เศษมีเครื่องหมายตามตัวตั้ง; ตัวหาร 0 → **error 11** |
| `pow.T` | float เท่านั้น | `pow()` ของ C |
| `and` `or` `xor` `shl` `shr` | int เท่านั้น | bitwise; `shr` ของ signed = arithmetic, unsigned = logical; shift count = ค่าของ `%b` mod ความกว้าง |
| `neg.T %d %a` | int, float | |
| `not.T %d %a` | int | bitwise NOT |

EQV/IMP: emitter แตกเป็น `xor`+`not` / `not`+`or`

### 4.3 เปรียบเทียบ

`eq ne lt le gt ge .T %d %a %b` — T เป็นตัวเลขหรือ `str`; ผลเป็น **`i32`** ค่า -1 (จริง) หรือ 0. `str` เทียบ byte ต่อ byte แบบ unsigned แล้วเทียบความยาว (QB). float: NaN เท่ากับ C (เทียบแล้วเป็นเท็จ ยกเว้น `ne`)

### 4.4 แปลงชนิด

| op | ความหมาย |
| --- | --- |
| `cvt.TO.FROM %d %s` | int→int: cast แบบ C (ตัด/ขยาย sign หรือ zero); int→float, float→float: ปัดใกล้สุด (ties-to-even); **float→int: ปัดครึ่งไปคู่ (`CINT`/`qbr`)** ค่าเกินช่วงของ i64 ได้ค่าที่ไม่กำหนด (ตาม C++ path) |
| `trunc.TO.FROM %d %s` | เหมือน `cvt` แต่ float→int ตัดเข้าหา 0 (cast ของ C) |

string ↔ ตัวเลขทำผ่าน builtin (`STR$.*`, `VAL`)

### 4.5 String

| op | ความหมาย |
| --- | --- |
| `concat %d %a %b` | ต่อ string |

ที่เหลือ (`LEN`, `MID$`, ...) เป็น builtin (§6)

### 4.6 Variable และ memory

| op | ความหมาย |
| --- | --- |
| `load.T %d $v` / `store.T $v %s` | อ่าน/เขียนตัวแปร scalar (รวม param byref — executor dereference ให้). T = ชนิดของตัวแปร; `fstr:N` ใช้ T=`str` |
| `addr %d $v` | address ของตัวแปร (ไม่ใช่ array) → `ptr` (ใช้ส่ง BYREF, เข้าถึง UDT) |
| `aref %d $a` | address ของ descriptor ของ array → ส่งให้ `param ... array` |
| `aaddr %d $a %i1 .. %in` | address ของ element; index เป็น int ชนิดใดก็ได้. ไม่ได้ DIM หรือจำนวน index ไม่ตรง หรือเกินขอบ → **error 9** |
| `ld.T %d %p off` / `st.T %p off %s` | อ่าน/เขียนชนิด T ที่ `p + off` (off = byte, immediate) |
| `ldfs %d %p off len` / `stfs %p off len %s` | fixed string `len` byte ↔ `str` (store ตัด/เติม space) |
| `padd %d %p %o` | `p + o` byte |
| `copy %dst %src size` | copy `size` byte (กำหนดค่า UDT ทั้งก้อน) |

ข้อกำหนด memory:
- storage ของ `str` เป็น handle ขนาด 8 byte ที่ executor จัดการ: `st.str` แทนที่ค่าเดิม (executor คืน string เดิม), `ld.str` ได้สำเนาค่า. UDT ใน v1 มีเฉพาะ field ที่ไม่ใช่ `str` (QB64 1.x) — `copy` จึงไม่ต้องรู้จัก handle
- `ptr` ชี้ได้เฉพาะ storage ที่ยังมีชีวิต: local ตายเมื่อ function return, element ตายเมื่อ `redim`/`erase`. executor ไม่ต้องตรวจ (เหมือน C++)
- layout ของ element: เรียงต่อกันด้วยขนาด element, **index แรกเปลี่ยนเร็วสุด** (เหมือน QB64)

### 4.7 Array

| op | ความหมาย |
| --- | --- |
| `dim $a n %lb1 %ub1 .. %lbn %ubn` | สร้าง n มิติ; ถ้า DIM อยู่แล้ว → **error 10**; `ub < lb` → error 9 |
| `redim $a n ...` | ทิ้งค่าเดิมแล้วสร้างใหม่ (ค่าเริ่ม 0/ว่าง) |
| `redimp $a n ...` | `REDIM _PRESERVE`: คง byte ส่วนต้นเท่าที่ขนาดใหม่รับได้ (เหมือน QB64), ส่วนที่เพิ่มเป็น 0 |
| `erase $a` | คืนหน่วยความจำ, array กลับเป็นไม่ได้ DIM |
| `lbound %d $a %k` / `ubound` | ขอบของมิติที่ `k` (เริ่ม 1) เป็น `i32`; k ผิดหรือไม่ได้ DIM → error 9 |

`local` array คืนเมื่อ function return; `static` / `global` คงอยู่. Auto-DIM (`A(5)` โดยไม่ DIM) emitter เป็นผู้ emit `dim` 0..10 เอง

### 4.8 Control flow

| op | ความหมาย |
| --- | --- |
| `label L` | ตำแหน่ง (namespace ต่อ function) |
| `jmp L` / `jz %c L` / `jnz %c L` | `%c` เป็น int ชนิดใดก็ได้; jz = กระโดดเมื่อเป็น 0 |
| `gosub L` | push (ตำแหน่งถัดไป) บน GOSUB stack **ของ frame ปัจจุบัน** แล้ว jmp |
| `gret` | `RETURN`: pop; stack ว่าง → **error 3** |
| `gretl L` | `RETURN L`: pop แล้ว jmp L |
| `call [%d] F %a..` | เรียก function. byval arg = ค่า, byref / array arg = `ptr`. มี `%d` เมื่อ F มี `-> T` เท่านั้น |
| `ret [%r]` | กลับ (`ret %r` เมื่อ function คืนค่า). ถ้าจบ main ด้วย `ret` = จบโปรแกรมแบบ `end` |
| `end` | END: flush + จบ (ถ้า console ไม่ใช่ `console only` และไม่มี `meta endkey 0` → พิมพ์ "Press any key to continue" แล้วรอ) |
| `stop` | STOP (เหมือน `end` ใน v1) |
| `system [%c]` | SYSTEM: จบทันที exit code = `%c` หรือ 0 |

### 4.9 Statement boundary

`line N` = เริ่ม statement ของบรรทัด N ใน source (บรรทัดหลัง `$INCLUDE` ใช้หมายเลขของ compiler). ใช้สำหรับ:
1. หมายเลขบรรทัดใน error (`ERL`, ข้อความ "Unhandled error")
2. จุด `RESUME` (กลับมาทำ statement นี้ซ้ำ) และ `RESUME NEXT` (ไป `line` ถัดไป)
3. จุด poll event (ON TIMER ฯลฯ, v1.x)

statement ในส่วน THEN/ELSE ของ IF บรรทัดเดียวไม่มี `line` ของตัวเอง

### 4.10 DATA

| op | ความหมาย |
| --- | --- |
| `read.T %d` | อ่าน DATA ถัดไปแปลงเป็นชนิด T (ตัวเลข: แบบ `VAL` แต่ข้อความที่ไม่ใช่ตัวเลข → **error 13**; หมด DATA → **error 4**) |
| `restore` / `restore L` | ตั้ง pointer ไป DATA แรก / ตำแหน่งของ `datalabel L` |

## 5. Function และ frame

- frame ใหม่ทุก `call`: locals ใหม่ (0/ว่าง), registers ใหม่, GOSUB stack ใหม่
- byref param: `load/store $p` ไปที่ storage ของ caller; `addr $p` ได้ ptr เดิมของ caller (ส่งต่อได้)
- recursion ไม่จำกัดนอกจาก memory (stack overflow → error 7 "Out of memory")
- string ที่คืนจาก function เป็นค่าใหม่

## 6. Builtin (`rt`)

`rt [%d] NAME %a..` เรียก runtime. ตาราง **normative** คือ `qb_ir::builtins::TABLE` (ชื่อ, ชนิด argument, ชนิดผล). Executor ต้องปฏิเสธชื่อที่ไม่รู้จักตอนโหลด. ข้อตกลง:

- ชื่อ = ชื่อ QBasic ตัวพิมพ์ใหญ่; `.T` ท้ายชื่อเลือก overload ตามชนิด (`STR$.f32`), `.n` = รูปที่มี argument เพิ่ม (`MID$.n` = มีความยาว)
- `ch` (i32) = หมายเลขไฟล์, `0` = หน้าจอ/console
- PRINT: `PRINTS ch s`, `PRINTN.T ch v` (ตัวเลขแบบ QB: มี space นำถ้าไม่ติดลบ และ space ท้าย), `PRINTZONE ch` (`,`), `PRINTNL ch`, `PRINTTAB/PRINTSPC ch n`. `PRINT` ที่ไม่มี `;`/`,` ท้าย = emitter ใส่ `PRINTNL`
- WRITE: `WRITES` (มี quote), `WRITEN.T` (ไม่มี space), `WRITESEP` (`,`), ท้ายด้วย `PRINTNL`
- PRINT USING: `USINGBEGIN ch fmt`, แล้ว `USINGS`/`USINGN.T` ต่อค่า, `USINGEND` (พิมพ์ literal ที่เหลือของ format); newline แยกด้วย `PRINTNL`
- builtin รายงาน error ผ่านกลไกเดียวกับ `error` (§7) — เช่น `MID$` start < 1 → error 5
- การเพิ่มชื่อใหม่ = เปลี่ยนแบบ compatible; เปลี่ยน signature ของชื่อเดิม = ห้าม (เพิ่มชื่อใหม่แทน)

## 7. Error

| op | ความหมาย |
| --- | --- |
| `onerror L` / `onerror 0` | ตั้ง/ยกเลิก handler. **L เป็น label ใน `main` เสมอ** (กฎของ QB) แม้ op อยู่ใน SUB |
| `ontimer %s %x L` | ON TIMER: `%s` i32 = slot (0 = `ON TIMER(x)`, อื่นๆ จาก `_FREETIMER`), `%x` f64 = วินาที; L เป็น label ใน `main`; handler รันแบบ GOSUB ระดับ module ที่ safe point, RETURN แล้ว slot ยิงได้อีก |
| `onplay %n L` | ON PLAY: `%n` i32 = จำนวนโน้ต (1..32); L เป็น label ใน `main`; handler รันแบบ GOSUB ระดับ module เมื่อคิวเพลงลดจาก n โน้ตเหลือน้อยกว่า n ขณะ `PLAY ON` (`rt PLAYEVENT` 1 ON, 2 OFF, 3 STOP) |
| `error %c` | ERROR n |
| `resume` / `resume next` / `resume L` | ใช้ได้ใน handler เท่านั้น (ไม่ใช่ → error 20) |

กลไก (เท่ากับ QBasic และ spike v0):
1. error (จาก op, builtin หรือ `error`) ขณะอยู่ใน frame F ที่ statement `line N`: ถ้าไม่มี handler หรือกำลังอยู่ใน handler → จบโปรแกรมด้วย "Unhandled error #code on line N" (exit 1)
2. ถ้ามี handler: บันทึก (F, ตำแหน่ง `line` ของ statement, N), ตั้ง ERR/ERL, แล้วรันโค้ดของ `main` ที่ label L **โดยไม่ unwind frame** (handler เห็น global ตามปกติ; frame ของ SUB ค้างอยู่)
3. `resume` กลับ F ที่ `line` เดิม; `resume next` กลับ F ที่ `line` ถัดไปของ F; `resume L` unwind กลับ frame ของ main แล้ว jmp L
4. handler จบด้วย `end`/`system` ได้ตามปกติ
5. `ERL` = หมายเลขบรรทัดแบบ line-number label ตัวล่าสุดที่ทำงานก่อน error (compiler emit `rt LASTLINE n` ที่ label ตัวเลข; ไม่มีเลย = 0, เหมือน `last_line` ของ C++); `_ERRORLINE` = บรรทัด source จาก `line N`

JIT: ฟังก์ชันที่อาจเกิด error ใน handler ภายนอกต้องสามารถ "หยุดที่ `line` แล้วกลับมาต่อ" — วิธีที่ง่ายที่สุดคือ JIT เฉพาะ function ที่ไม่มี op ที่เกิด error ได้ หรือ fallback ไป VM ตอนเกิด error (deopt ที่ `line`)

## 8. Packaging

exe = `qb-exec` + ข้อความ QIR + u64 LE ความยาว + `QBNEXIR0` (เหมือน v0). Executor ดู header `QIR <n>` เพื่อเลือก loader (v0 spike ยังรันได้จนกว่าจะลบ)

## 9. ตัวอย่าง

```basic
FUNCTION Fact& (n&)
    IF n& <= 1 THEN Fact& = 1 ELSE Fact& = n& * Fact&(n& - 1)
END FUNCTION
PRINT Fact&(10)
```

```
QIR 1
func main
line 4
const.i32 %0 10
call %1 FACT %0     ; byval เพราะ argument เป็น expression (emitter สร้าง temp แล้วส่ง addr ถ้า param เป็น byref)
const.i32 %2 0
cvt.i64.i32 %3 %1
rt PRINTN.i64 %2 %3
rt PRINTNL %2
end
endfunc
func FACT -> i32
param $N& i32 byref
local $FACT& i32
...
load.i32 %9 $FACT&
ret %9
endfunc
```

## 10. สิ่งที่จะเพิ่มใน v1.x (compatible)

events (`ON TIMER/KEY` + `line` poll), `CHAIN`/`COMMON`, `_MEM` ops, `DECLARE LIBRARY` call thunk (R11d), binary encoding + cache. ทุกอย่างเพิ่มเป็น op/builtin/meta ใหม่ ไม่เปลี่ยนความหมายของของเดิม
