# ส่วนขยายของภาษา

[สารบัญคู่มือ](../README.md) · [English](../en/language.md)

QBNex รับโปรแกรม QBasic / QuickBASIC 4.5 และภาษาถิ่น QB64 (keyword ที่ขึ้นต้น
ด้วย `_`, metacommand `$`, `_UNSIGNED`, `_INTEGER64`, `_FLOAT`, `_MEM`,
`DECLARE LIBRARY` ...) หน้านี้รวบรวมสิ่งที่ QBNex เพิ่มเข้ามา แต่ละหัวข้อมี
สถานะกำกับ (ดู[ป้ายสถานะ](../README.md#status-labels--ป้ายสถานะ))

ตรวจกับคอมไพเลอร์ปัจจุบันแล้ว: หัวข้อที่เป็น **ใช้ได้แล้ว** คอมไพล์ได้จริง
วันนี้ ส่วนหัวข้อ **วางแผนไว้** คอมไพเลอร์ยังไม่รับ แม้ editor จะไฮไลต์ให้แล้วก็ตาม

## ใช้ได้แล้ว

### ไฟล์ source แบบ CP437 หรือ UTF-8

ไฟล์ที่บันทึกจาก QBasic (ไบต์ CP437 เช่นตัวอักษรตีกรอบใน string)
คอมไพล์ได้ตามที่เป็นอยู่และไบต์ไม่ถูกเปลี่ยน ไฟล์ UTF-8 (มีหรือไม่มี BOM)
ก็ใช้ได้

### โมดูลไลบรารีมาตรฐาน: `'$IMPORT:'name'`

```basic
'$IMPORT:'json'
'$IMPORT:'ext.http'
```

วางบรรทัดนี้ตรงไหนของไฟล์หลักก็ได้ ส่วนประกาศของแต่ละโมดูลจะถูกวางไว้
ก่อนโปรแกรม และ procedure ของโมดูลจะถูกวางไว้หลังโปรแกรม ดังนั้น
`DIM SHARED`, TYPE และ SUB ของโมดูลใช้ได้ไม่ว่าบรรทัด import จะอยู่ตรงไหน
ดู[ไลบรารีมาตรฐาน](stdlib.md)

### `_CONTINUE`

ข้ามไปทำรอบถัดไปของ loop `FOR`, `DO` หรือ `WHILE` ชั้นในสุด

```basic
FOR i = 1 TO 5
    IF i MOD 2 = 0 THEN _CONTINUE
    PRINT i
NEXT
```

### CLASS (แปลงเป็นโค้ดธรรมดาก่อนคอมไพล์ ต้องใช้คู่กับ `'$IMPORT:'qbnex'`)

```basic
'$IMPORT:'qbnex'
CLASS Dog
    Name AS STRING * 32
    CONSTRUCTOR (petName AS STRING)
        ME.Name = petName
    END CONSTRUCTOR
END CLASS
```

`CLASS ... END CLASS` ที่มี field, `CONSTRUCTOR`, สมาชิกแบบ
`METHOD`/`SUB`/`FUNCTION`, `EXTENDS` และ `IMPLEMENTS` จะถูกเขียนใหม่เป็น TYPE
กับ procedure ก่อนคอมไพล์ ส่วน CLASS ตัวจริงใน QIR (virtual dispatch,
`NEW`/`DELETE`, property) อยู่ในสถานะวางแผนไว้

### `DECLARE LIBRARY`, `_FLOAT`, `_MEM`

ใช้ได้เหมือน QB64 ใน QIR back end ฟังก์ชันภายนอกถูกเรียกผ่าน
`crates/qb-ffi` (calling convention ของ Windows x64 และ Linux x86-64/AArch64)
ส่วน `_FLOAT` เป็นเลขทศนิยม 80-bit ที่คำนวณด้วยซอฟต์แวร์ได้ผลตรงเป๊ะ

## วางแผนไว้ (port-plan หัวข้อ 9.4)

ออกแบบไว้แล้ว และเครื่องมือของ editor (formatter, language server,
การไฮไลต์) รู้จักคำเหล่านี้แล้ว แต่คอมไพเลอร์ยังไม่รับ คำใหม่ทุกคำขึ้นต้น
ด้วย `_` หรือใช้เฉพาะในรูปคำสั่งใหม่ โปรแกรมเดิมจึงยังคอมไพล์ได้เหมือนเดิม

### การควบคุมลำดับการทำงาน

```basic
x = _IIF(score >= 50, "pass", "fail")      ' คำนวณเฉพาะค่าที่ถูกเลือก
IF a <> 0 _ANDALSO b / a > 2 THEN ...       ' AND / OR แบบ short-circuit (_ORELSE)

FOR EACH item IN names$()
    PRINT item
NEXT
```

และ `EXIT FOR`/`_CONTINUE` ที่ระบุได้ว่าจะออกหรือข้ามรอบของ loop ไหน

### จัดการ error: TRY / CATCH / FINALLY

```basic
TRY
    OPEN "data.txt" FOR INPUT AS #1
    LINE INPUT #1, first$
CATCH e
    PRINT "error"; e; _ERRORMESSAGE$
FINALLY
    CLOSE
END TRY

_THROW 5, "bad argument"
```

สร้างบนกลไก `ON ERROR` เดิม `ON ERROR GOTO` ยังใช้ได้ตามปกติ
(ตัว `_ERRORMESSAGE$` ใช้ได้แล้ววันนี้)

### ชนิดข้อมูล

```basic
ENUM Direction
    North
    East
    South
    West
END ENUM

DIM primes() AS LONG = {2, 3, 5, 7}
```

รวมถึง string ความยาวไม่คงที่และ array ภายใน TYPE และ CONST ได้ทุกชนิด

### Procedure

พารามิเตอร์ `OPTIONAL` พร้อมค่าเริ่มต้น
(`SUB box (w AS LONG, OPTIONAL h AS LONG = 10)`), overload ตามชนิดแบบ
เลือกเปิด, function pointer (`@SubName`, `CALL ptr(...)`) และ recursion
ลึกๆ โดยไม่ stack overflow (frame อยู่บน heap ของ VM)

### String

```basic
name$ = "Ann"
PRINT $"Hello {name$}, you are {age%} years old"   ' string interpolation

n& = _SPLIT("a,b,c", ",", parts$())
PRINT _JOIN$(parts$(), " | ")
PRINT _REPLACE$("a-b-c", "-", "+"), _STARTSWITH("QBNex", "QB"), _ENDSWITH(f$, ".bas")
PRINT _FORMAT$(1234.5, "#,##0.00")
PRINT _ULEN(s$), _UMID$(s$, 2, 3), _UCHR$(3647), _UASC("ก")      ' UTF-8 นับเป็นตัวอักษร
```

ระหว่างนี้ใช้โมดูล [`ext.utf8`](stdlib.md#extutf8) และ
[`ext.regex`](stdlib.md#extregex) แทนได้

### Collection ในตัวภาษา

`DIM l AS _LIST`, `DIM d AS _DICT` ใช้คู่กับ `FOR EACH` ตอนนี้ใช้
[`ext.collections`](stdlib.md#extcollections) (แบบ handle) หรือโมดูล
`collections.*` แทน

### ไลบรารีในตัว

`_REGEX`, `_RANDOMSEED`, JSON, HTTP(S), hash, zip, UUID และวันที่แบบ ISO
เป็น built-in ตอนนี้ทุกอย่างยกเว้น UUID ใช้ได้แล้วผ่านโมดูล `ext.*`

### syntax อื่นที่เคยอยู่ใน README เดิม

`x += 1` (รวม `-=`, `*=`, `/=`), บรรทัด comment แบบ `# comment`,
`IMPORT module` ที่ไม่ใช้รูป `'$IMPORT:` และ `FUNCTION name (...) AS STRING`
เคยถูกเขียนไว้ใน README รุ่นก่อน แต่คอมไพเลอร์ปัจจุบันยังไม่รับ จึงให้ถือว่า
อยู่ในสถานะวางแผนไว้เช่นกัน
