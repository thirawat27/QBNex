# ส่วนขยายของภาษา

[สารบัญคู่มือ](../README.md) · [English](../en/language.md)

QBNex รับโปรแกรม QBasic / QuickBASIC 4.5 และภาษาถิ่น QB64 (keyword ที่ขึ้นต้น
ด้วย `_`, metacommand `$`, `_UNSIGNED`, `_INTEGER64`, `_FLOAT`, `_MEM`,
`DECLARE LIBRARY` ...) หน้านี้รวบรวมสิ่งที่ QBNex เพิ่มเข้ามา แต่ละหัวข้อมี
สถานะกำกับ (ดู[ป้ายสถานะ](../README.md#status-labels--ป้ายสถานะ))
โปรแกรมเดิมไม่ได้รับผลกระทบ เพราะคำใหม่ขึ้นต้นด้วย `_` หรือถูกตีความเฉพาะ
ในจุดที่ QBasic เดิมจะแจ้ง syntax error

ตัวอย่างในหน้านี้ถูกคอมไพล์และรันโดย `tests/docs/check_docs.ps1`
(ผลลัพธ์ที่ควรได้แสดงไว้ใต้แต่ละตัวอย่าง)

## ใช้ได้แล้ว

### `_IIF`: IF แบบบรรทัดเดียวในนิพจน์

<!-- docs-test: output -->
```basic
x = 7
PRINT _IIF(x > 5, "big", "small"); _IIF(x < 5, 1, 2)
```

<!-- expected output -->
```text
big 2
```

คำนวณเฉพาะค่าที่ถูกเลือก ทั้งสองค่าต้องเป็น string ทั้งคู่ หรือเป็นตัวเลขทั้งคู่

### `_ANDALSO`, `_ORELSE`: ตรรกะแบบ short-circuit

<!-- docs-test: output -->
```basic
DIM a(1 TO 3) AS INTEGER
i = 4
IF i <= 3 _ANDALSO a(i) = 0 THEN PRINT "in range" ELSE PRINT "index"; i; "skipped"
IF i > 3 _ORELSE a(i) = 0 THEN PRINT "no subscript error"
```

<!-- expected output -->
```text
index 4 skipped
no subscript error
```

ด้านขวาจะถูกคำนวณเมื่อจำเป็นเท่านั้น ผลลัพธ์เป็น -1 หรือ 0

### String interpolation

<!-- docs-test: output -->
```basic
name$ = "Ada"
n% = 3
PRINT $"Hello {name$}, {n%} new messages, {{braces}}"
```

<!-- expected output -->
```text
Hello Ada, 3 new messages, {braces}
```

ตัวเลขถูกเขียนแบบ `LTRIM$(STR$(x))` (ฟังก์ชัน `_TOSTR$`) ส่วน `{{` และ
`}}` คือวงเล็บปีกกาตามตัวอักษร นิพจน์ใน `{}` มี string literal ได้
เช่น `$"Hi {name$ + "!"}"`

### `FOR EACH`

<!-- docs-test: output -->
```basic
DIM scores(1 TO 4) AS INTEGER
FOR i = 1 TO 4: scores(i) = i * 10: NEXT
FOR EACH s IN scores()
    total = total + s
NEXT s
PRINT total
```

<!-- expected output -->
```text
 100
```

วนทุกสมาชิกจาก LBOUND ถึง UBOUND array หลายมิติวนโดยให้ index แรกอยู่วงนอกสุด
ตัวแปรของ loop เป็นสำเนาของสมาชิก

### `ENUM`

<!-- docs-test: output -->
```basic
ENUM Fruit
    Apple
    Banana = 5
    Cherry, Grape
END ENUM
PRINT Apple; Banana; Cherry; Grape
```

<!-- expected output -->
```text
 0  5  6  7
```

สมาชิกกลายเป็น `CONST` จำนวนเต็ม นับขึ้นจาก 0 หรือจาก `= ค่า` ตัวล่าสุด
`ENUM` แบบไม่มีชื่อก็ใช้ได้

### `TRY` / `CATCH` / `FINALLY`

<!-- docs-test: output -->
```basic
TRY
    ERROR 53
    PRINT "not reached"
CATCH e
    PRINT "caught error"; e
FINALLY
    PRINT "cleanup"
END TRY
```

<!-- expected output -->
```text
caught error 53
cleanup
```

สร้างบน `ON ERROR`: `CATCH e` เก็บหมายเลข error ไว้ใน `e` ถ้าไม่มี
`CATCH` error จะถูกยกขึ้นอีกครั้งหลัง `FINALLY` TRY ใช้ในตัว SUB และในไฟล์
`$INCLUDE` ได้ และ keyword ของ TRY อยู่บรรทัดเดียวกับคำสั่งอื่นได้
(`TRY: ERROR 6: CATCH e: PRINT e: END TRY`) การหารจำนวนเต็มด้วยศูนย์
(`\` และ `MOD`) เป็น error 11 ที่ดักได้ `ON ERROR GOTO` ยังใช้ได้ตามเดิม:
handler ที่ตั้งไว้ก่อน `TRY` กลับมามีผลเมื่อจบส่วน body รวมถึงเมื่อออกจาก body
ด้วย `EXIT FOR`, `EXIT DO`, `EXIT WHILE`, `_CONTINUE`, `EXIT SUB`,
`EXIT FUNCTION` หรือ `GOTO` ไปยัง label ที่อยู่นอก body

### พารามิเตอร์ `OPTIONAL`

<!-- docs-test: output -->
```basic
DECLARE SUB Greet (who$, OPTIONAL greeting$ = "Hello", OPTIONAL times% = 1)
Greet "Ada"
Greet "Bob", "Hi", 2
PRINT Scale(3); Scale(3, 10)
SYSTEM

SUB Greet (who$, OPTIONAL greeting$ = "Hello", OPTIONAL times% = 1)
    FOR i% = 1 TO times%
        PRINT greeting$; ", "; who$
    NEXT
END SUB

FUNCTION Scale& (v AS LONG, OPTIONAL factor AS LONG = 2)
    Scale& = v * factor
END FUNCTION
```

<!-- expected output -->
```text
Hello, Ada
Hi, Bob
Hi, Bob
 6  30
```

ละ argument ท้ายๆ ได้ ค่าเริ่มต้นเป็นนิพจน์อะไรก็ได้ที่ใช้ได้ในจุดที่เรียก
procedure

### `_CONTINUE`

<!-- docs-test: output -->
```basic
FOR i = 1 TO 5
    IF i MOD 2 = 0 THEN _CONTINUE
    PRINT i;
NEXT
```

<!-- expected output -->
```text
 1  3  5
```

ข้ามไปรอบถัดไปของ loop `FOR`, `DO` หรือ `WHILE` ชั้นในสุด

### `$ASSERTS` และ `_ASSERT`

<!-- docs-test: output -->
```basic
$ASSERTS
_ASSERT 1 + 1 = 2, "math works"
PRINT "asserts passed"
```

<!-- expected output -->
```text
asserts passed
```

### ไฟล์ source แบบ CP437 หรือ UTF-8

ไฟล์ที่บันทึกจาก QBasic (ไบต์ CP437 เช่นตัวอักษรตีกรอบใน string)
คอมไพล์ได้ตามที่เป็นอยู่และไบต์ไม่ถูกเปลี่ยน ไฟล์ UTF-8 (มีหรือไม่มี BOM)
ก็ใช้ได้

### โมดูลไลบรารีมาตรฐาน: `'$IMPORT:'name'`

<!-- docs-test: output -->
```basic
'$IMPORT:'url'
PRINT Url_Encode$("a b&c")
```

<!-- expected output -->
```text
a%20b%26c
```

วางบรรทัดนี้ตรงไหนของไฟล์หลักก็ได้ ส่วนประกาศของแต่ละโมดูลจะถูกวางไว้
ก่อนโปรแกรม และ procedure ของโมดูลจะถูกวางไว้หลังโปรแกรม ดังนั้น
`DIM SHARED`, TYPE และ SUB ของโมดูลใช้ได้ไม่ว่าบรรทัด import จะอยู่ตรงไหน
ดู[ไลบรารีมาตรฐาน](stdlib.md)

### การเรียก method บนค่าของ TYPE

<!-- docs-test: output -->
```basic
TYPE Point
    x AS SINGLE
    y AS SINGLE
END TYPE

DIM p AS Point
p.Move 3, 4                    ' Point_Move p, 3, 4
PRINT p.x; p.y; p.Length       ' Point_Length(p)
PRINT p.Label$("p")            ' Point_Label$(p, "p")
SYSTEM

SUB Point_Move (pt AS Point, dx AS SINGLE, dy AS SINGLE)
    pt.x = pt.x + dx: pt.y = pt.y + dy
END SUB

FUNCTION Point_Length! (pt AS Point)
    Point_Length! = SQR(pt.x * pt.x + pt.y * pt.y)
END FUNCTION

FUNCTION Point_Label$ (pt AS Point, label AS STRING)
    Point_Label$ = label + "(" + LTRIM$(STR$(pt.x)) + ", " + LTRIM$(STR$(pt.y)) + ")"
END FUNCTION
```

```text
 3  4  5
p(3, 4)
```

เมื่อตัวแปร พารามิเตอร์ สมาชิกของ array หรือ field มีชนิดเป็น TYPE `T` ที่ไม่มี
field ชื่อนั้น `x.Name(args)` จะเรียก `T_Name(x, args)` ถ้าไม่มีวงเล็บ คำสั่ง
`x.Add "a"` จะเรียก `T_Add x, "a"` และ `x.Clear` เรียก `T_Clear x` ส่วนในนิพจน์
`x.Count` เรียก `T_Count(x)` field จริงมาก่อนเสมอ โปรแกรมเดิมจึงไม่เปลี่ยน และ
method ของ CLASS มาก่อน procedure ชื่อเดียวกัน TYPE ของไลบรารี `List`, `Dict`,
`Set`, `Url` และ `Result` ใช้งานแบบนี้ (`names.Add "Ann"`,
`PRINT names.Join$(", ")`) ดู[ไลบรารีมาตรฐาน](stdlib.md)

### CLASS (แปลงเป็นโค้ดธรรมดาก่อนคอมไพล์ ใช้คู่กับ `'$IMPORT:'oop.class'`)

```basic
'$IMPORT:'oop.class'
CLASS Dog
    Name AS STRING * 32
    CONSTRUCTOR (petName AS STRING)
        ME.Name = petName
    END CONSTRUCTOR
END CLASS
```

`CLASS ... END CLASS` ที่มี field, `CONSTRUCTOR`, สมาชิกแบบ
`METHOD`/`SUB`/`FUNCTION`, `EXTENDS` และ `IMPLEMENTS` จะถูกเขียนใหม่เป็น TYPE
กับ procedure ก่อนคอมไพล์ class แม่ต้องประกาศก่อน class ที่ `EXTENDS` มัน
`oop.class` (และ `oop.interface`) เก็บ runtime ที่โค้ดที่แปลงแล้วใช้ ส่วน
`'$IMPORT:'qbnex'` ก็ import ให้ด้วย

### `DECLARE LIBRARY`, `_FLOAT`, `_MEM`

ใช้ได้เหมือน QB64 ฟังก์ชันภายนอกถูกเรียกผ่าน
`crates/engine/ffi` (calling convention แบบ Win64, System V x86-64 และ AArch64
รวมถึงแบบ AArch64 ของ Apple จึงใช้ได้บน Windows, Linux และ macOS)
ส่วน `_FLOAT` เป็นเลขทศนิยม 80-bit ที่คำนวณด้วยซอฟต์แวร์ได้ผลตรงเป๊ะ

## วางแผนไว้

ออกแบบแล้วแต่ยังไม่อยู่ในคอมไพเลอร์ (เครื่องมือ editor รู้จักบางคำแล้ว)

<!-- docs-test: compile-error -->
```basic
_THROW 5, "bad argument"
```

<!-- docs-test: compile-error -->
```basic
DIM primes() AS LONG = {2, 3, 5, 7}
```

<!-- docs-test: compile-error -->
```basic
n& = _SPLIT("a,b,c", ",", parts$())
PRINT _JOIN$(parts$(), " | "), _REPLACE$("a-b", "-", "+"), _ULEN("ก")
```

<!-- docs-test: compile-error -->
```basic
x = 1
x += 2
```

- `_THROW code[, message$]` (ตอนนี้ใช้ `ERROR code`)
- array literal และ array ภายใน TYPE (string ความยาวไม่คงที่ใน TYPE ใช้ได้แล้ว:
  `nm AS STRING`)
- ตัวช่วย string `_SPLIT`, `_JOIN$`, `_REPLACE$`, `_STARTSWITH`,
  `_ENDSWITH`, `_FORMAT$` และตัวช่วย UTF-8 `_ULEN`, `_UMID$`, `_UCHR$`,
  `_UASC` (ตอนนี้ใช้โมดูล [`strings`](stdlib.md#strings),
  [`utf8`](stdlib.md#utf8) และ [`regex`](stdlib.md#regex) แทน)
- `_LIST` / `_DICT` ในตัวภาษา (ตอนนี้ใช้โมดูล [`collections`](stdlib.md#collections))
- `_REGEX`, `_RANDOMSEED` และ UUID แบบ built-in
- CLASS ใน QIR: virtual dispatch, `NEW`/`DELETE`, property, function
  pointer (`@SubName`), overload ตามชนิด
- `x += 1` (รวม `-=`, `*=`, `/=`), บรรทัด comment แบบ `# comment`,
  `IMPORT module` ที่ไม่ใช้รูป `'$IMPORT:` และ `FUNCTION name (...) AS type`
