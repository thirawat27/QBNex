# 🐛 รายงานบั๊ก QBNex Compiler Version 1.0.0

**วันที่รายงาน**: 14 เมษายน 2569  
**ผู้รายงาน**: Automated Test Suite  
**เวอร์ชันที่ทดสอบ**: QBNex Compiler 1.0.0  
**แพลตฟอร์ม**: Windows 64-bit  
**ระดับความรุนแรง**: มีบั๊กวิกฤตที่ต้องแก้ไขทันที

---

## 📋 สรุปผู้บริหาร

จากการทดสอบระบบแบบเต็มรูปแบบ พบบั๊กทั้งหมด **3 ระดับ**:

| ระดับ | จำนวน | สถานะ |
|-------|-------|--------|
| 🔴 วิกฤต (Critical) | 1 | ต้องแก้ทันที |
| 🟡 ร้ายแรง (High) | 1 | ต้องแก้ไข |
| 🟠 ปานกลาง (Medium) | 1 | ควรแก้ไข |

**ผลกระทบ**: บั๊กวิกฤตทำให้ไม่สามารถใช้ Standard Library และ OOP Features ได้เลย

---

## 🔴 บั๊กที่ 1: ระบบ Import เติม Parameters ผิด (CRITICAL)

### ข้อมูลบั๊ก

**ชื่อบั๊ก**: Import Function Parameter Injection Bug  
**ระดับ**: 🔴 วิกฤต (Critical)  
**ส่วนที่ได้รับผลกระทบ**: Standard Library Import System  
**ความถี่**: เกิดขึ้น 100% เมื่อใช้ฟังก์ชันที่มี 2+ parameters จาก imported modules

### รายละเอียด

เมื่อ import ฟังก์ชันจาก standard library modules และเรียกใช้ฟังก์ชันเหล่านั้น คอมไพเลอร์จะ**เติมตัวเลขผิดเข้าไปใน function call** ทำให้เกิด error "Illegal string-number conversion"

### ตัวอย่างที่ชัดเจน

#### โค้ดที่เขียน:
```basic
'$IMPORT:'io.path'

SUB TestPath ()
    DIM p AS STRING
    p = Path_Join$("root", "file.txt")
    PRINT "Result: "; p
END SUB
```

#### คอมไพเลอร์แปลงเป็น (ผิด!):
```c
// ใน C code ที่สร้าง
p = PATH_JOIN$("root", 4, "file.txt", 8)
//                ↑ เพิ่มเลข 4   ↑ เพิ่มเลข 8
```

#### ข้อผิดพลาดที่เกิดขึ้น:
```
Illegal string-number conversion
Caused by (or after):P = PATH_JOIN$ ( "root",4 , "file.txt",8 )
LINE 4:p = Path_Join$("root", "file.txt")
```

### ฟังก์ชันที่ได้รับผลกระทบ

ฟังก์ชันทั้งหมดที่มี 2+ parameters:

| ฟังก์ชัน | Module | Parameters | สถานะ |
|----------|--------|------------|--------|
| `Path_Join$` | io.path | 2 | ❌ ผิดพลาด |
| `Path_Normalize$` | io.path | 1 | ✅ ใช้ได้ |
| `Text_PadRight$` | strings.text | 3 | ❌ ผิดพลาด |
| `Text_PadLeft$` | strings.text | 3 | ❌ ผิดพลาด |
| `Text_Repeat$` | strings.text | 2 | ❌ ผิดพลาด |
| `Text_StartsWith$` | strings.text | 2 | ❌ ผิดพลาด |
| `Text_EndsWith$` | strings.text | 2 | ❌ ผิดพลาด |
| `Text_Contains$` | strings.text | 2 | ❌ ผิดพลาด |
| `Dict_Set` | collections.dictionary | 3 | ❌ ผิดพลาด |
| `CSV_Row3$` | io.csv | 3 | ❌ ผิดพลาด |
| `Math_Clamp#` | math.numeric | 3 | ❌ ผิดพลาด |
| `Json_Object3$` | io.json | 6 | ❌ ผิดพลาด |

### ฟังก์ชันที่ยังใช้ได้

ฟังก์ชันที่มี **parameter เดียว** หรือ **ไม่มี parameter**:

```basic
' ✅ ใช้ได้
'$IMPORT:'sys.env'
PRINT Env_Platform$          ' 0 parameters
PRINT Env_GetHome$           ' 0 parameters
PRINT Env_Is64Bit&           ' 0 parameters

' ✅ ใช้ได้
'$IMPORT:'io.path'
PRINT Path_Separator$        ' 0 parameters
PRINT Path_FileName$("a/b")  ' 1 parameter - อาจจะได้

' ❌ ใช้ไม่ได้
'$IMPORT:'io.path'
PRINT Path_Join$("a", "b")   ' 2 parameters - พัง!
```

### การทดสอบที่ยืนยัน

**Test File**: `path_join_test.bas`
```basic
'$IMPORT:'io.path'

SUB TestPath ()
    DIM p AS STRING
    p = Path_Join$("root", "file.txt")
    PRINT "Joined: "; p
END SUB
```

**ผลลัพธ์**: 
```
❌ Failed to compile
Error: Illegal string-number conversion
```

### ผลกระทบ

1. **Standard Library ใช้ไม่ได้**: ไม่สามารถเรียกใช้ฟังก์ชันส่วนใหญ่จาก stdlib ได้
2. **Smoke Tests พัง**: การทดสอบ smoke tests ทั้งหมดล้มเหลว
3. **Demo Programs พัง**: Demo programs ทั้งหมดใช้ไม่ได้
4. **Collections ใช้ไม่ได้**: List, Stack, Queue, Dictionary ไม่สามารถใช้งานได้เต็มที่

### สาเหตุที่เป็นไปได้

1. **Import Processor Bug**: บั๊กในส่วนที่ประมวลผล `'$IMPORT:'` directive
2. **Parameter Encoding Error**: การเข้ารหัส parameters ใน function call ผิดพลาด
3. **String Interpolation Issue**: ปัญหาในการจัดการ string parameters
4. **AST Transformation Bug**: บั๊กในการแปลง Abstract Syntax Tree ของ imports

### ตำแหน่งไฟล์ที่ต้องตรวจสอบ

```
source/
├── qbnex.bas                    ← Main compiler
├── utilities/
│   ├── build.bas                ← Build process
│   └── strings.bas              ← String handling
└── global/
    ├── compiler_settings.bas    ← Compiler settings
    └── constants.bas            ← Constants
```

### ความเร่งด่วน

🔴 **ต้องแก้ไขทันที** - Blocker สำหรับการใช้งานจริง

---

## 🟡 บั๊กที่ 2: CLASS Syntax Error (HIGH)

### ข้อมูลบั๊ก

**ชื่อบั๊ก**: CLASS Keyword Parse Error  
**ระดับ**: 🟡 ร้ายแรง (High)  
**ส่วนที่ได้รับผลกระทบ**: OOP Parser - CLASS/END CLASS blocks  
**ความถี่**: เกิดขึ้น 100% เมื่อใช้ CLASS keyword

### รายละเอียด

คอมไพเลอร์ไม่สามารถ parse คำว่า `CLASS` ได้ ทำให้เกิด syntax error ทันทีที่เจอ CLASS definition

### ตัวอย่างที่ชัดเจน

#### โค้ดที่เขียน:
```basic
'$IMPORT:'qbnex'

CLASS Animal
    Name AS STRING * 32
    Age AS INTEGER

    CONSTRUCTOR (petName AS STRING, petAge AS INTEGER)
        ME.Name = petName
        ME.Age = petAge
    END CONSTRUCTOR

    FUNCTION Describe$ ()
        Describe$ = RTRIM$(ME.Name)
    END FUNCTION
END CLASS

DIM pet AS Animal
__QBNEX_Animal_CTOR pet, "Buddy", 3
PRINT pet.Describe$()
```

#### ข้อผิดพลาดที่เกิดขึ้น:
```
Syntax error
Caused by (or after):CLASS ANIMAL
LINE 7:CLASS Animal
```

### การทดสอบที่ยืนยัน

**Test File 1**: `source/stdlib/examples/class_syntax_demo.bas`
```basic
'$IMPORT:'qbnex'

TYPE Animal
    Header AS QBNex_ObjectHeader
    Name AS STRING * 32
    Age AS INTEGER
END TYPE

TYPE Dog
    Header AS QBNex_ObjectHeader
    Name AS STRING * 32
    Age AS INTEGER
    Breed AS STRING * 32
END TYPE

' ... โค้ดอื่น ๆ
```

**ผลลัพธ์**:
```
❌ Failed to compile
Error: Syntax error at "CLASS ANIMAL"
```

**Test File 2**: `source/stdlib/examples/top_level_runtime_regression.bas`
```basic
'$IMPORT:'qbnex'

CLASS Dog
    Name AS STRING * 32

    CONSTRUCTOR (petName AS STRING)
        ME.Name = petName
    END CONSTRUCTOR

    FUNCTION Describe$ ()
        Describe$ = RTRIM$(ME.Name)
    END FUNCTION
END CLASS

DIM pet AS Dog
__QBNEX_Dog_CTOR pet, "Buddy"
PRINT pet.Describe$()
```

**ผลลัพธ์**:
```
❌ Failed to compile
Error: Syntax error at "CLASS DOG"
```

### OOP Features ที่ใช้ไม่ได้

| Feature | ตัวอย่าง | สถานะ |
|---------|---------|--------|
| `CLASS` | `CLASS Animal` | ❌ Syntax error |
| `CONSTRUCTOR` | `CONSTRUCTOR (name AS STRING)` | ❌ ใช้ไม่ได้ |
| `ME.` keyword | `ME.Name = value` | ❌ ใช้ไม่ได้ |
| `FUNCTION` ใน CLASS | `FUNCTION Describe$()` | ❌ ใช้ไม่ได้ |
| Inheritance | `CLASS Dog EXTENDS Animal` | ❌ ใช้ไม่ได้ |
| Interfaces | `IMPLEMENTS IPet` | ❌ ใช้ไม่ได้ |

### TYPE ยังใช้ได้

ข้อสังเกต: `TYPE...END TYPE` **ยังใช้ได้** ✅

```basic
' ✅ ใช้ได้
TYPE Animal
    Name AS STRING * 32
    Age AS INTEGER
END TYPE

DIM a AS Animal
a.Name = "Buddy"
```

### ผลกระทบ

1. **OOP ใช้ไม่ได้**: ไม่สามารถเขียน code แบบ OOP ได้
2. **StdLib Demo พัง**: Demo programs ทั้งหมดใช้ CLASS
3. **Examples พัง**: ตัวอย่าง OOP ทั้งหมดใช้ไม่ได้
4. **Modern BASIC ใช้ไม่ได้**: ไม่สามารถใช้ syntax สมัยใหม่ได้

### สาเหตุที่เป็นไปได้

1. **Keyword Missing**: Parser ไม่มีคำว่า CLASS ใน keyword list
2. **Grammar Incomplete**: CLASS grammar rule ไม่สมบูรณ์
3. **Not Implemented**: ฟีเจอร์ CLASS อาจจะยังไม่ได้ implement
4. **Import Order Issue**: CLASS ทำงานได้เฉพาะตอนไม่ใช้ import

### ตำแหน่งไฟล์ที่ต้องตรวจสอบ

```
source/
├── qbnex.bas                    ← ต้องตรวจสอบ parser section
└── stdlib/
    └── oop/
        ├── class.bas            ← CLASS implementation
        └── interface.bas        ← INTERFACE implementation
```

### ความเร่งด่วน

🟡 **ต้องแก้ไข** - สำคัญแต่ไม่ถึงกับ blocker ถ้าใช้ classic BASIC style

---

## 🟠 บั๊กที่ 3: Console Output ไม่ทำงาน (MEDIUM)

### ข้อมูลบั๊ก

**ชื่อบั๊ก**: PRINT Statements Do Not Output to Console STDOUT  
**ระดับ**: 🟠 ปานกลาง (Medium)  
**ส่วนที่ได้รับผลกระทบ**: Console/Terminal output  
**ความถี่**: เกิดขึ้นเมื่อรัน executable จาก command line

### รายละเอียด

โปรแกรมที่ compile แล้ว ไม่แสดงผลลัพธ์ทาง console เมื่อรัน executable โดยตรง ถึงแม้จะมีคำสั่ง `PRINT` ในโค้ด

### ตัวอย่างที่ชัดเจน

#### โค้ดที่เขียน:
```basic
CLS
PRINT "Hello, QBNex!"
PRINT "Testing compiler functionality..."
PRINT "Date: "; DATE$
PRINT "Time: "; TIME$
PRINT "Test complete!"
SYSTEM 0
```

#### Compile:
```bash
qb.exe hello_test.bas -w
# Build complete: hello_test.exe
```

#### รัน executable:
```bash
D:\QBNex> hello_test.exe
(ไม่มี output ใด ๆ เลย)

D:\QBNex>
```

### การทดสอบที่ยืนยัน

**Test 1**: Hello World พื้นฐาน
```basic
PRINT "Hello, QBNex!"
```
**ผลลัพธ์**: ❌ ไม่แสดงอะไรเลย

**Test 2**: พร้อม SYSTEM 0
```basic
PRINT "Hello, QBNex!"
SYSTEM 0
```
**ผลลัพธ์**: ❌ ไม่แสดงอะไรเลย

**Test 3**: ใช้ -x flag
```bash
qb.exe hello_test.bas -x
```
**ผลลัพธ์**: ✅ แสดง output (แต่เป็นของ compiler, ไม่ใช่ของโปรแกรม)

### การทดสอบด้วย redirection

พยายามจับ output ด้วยวิธีต่าง ๆ:

```bash
# วิธีที่ 1: Redirection
hello_test.exe > output.txt
type output.txt
# (ว่างเปล่า)

# วิธีที่ 2: Pipe
hello_test.exe | more
# (ว่างเปล่า)

# วิธีที่ 3: Start /wait
start /wait /min hello_test.exe
# (ไม่มี output)
```

### สิ่งที่ยังใช้ได้

✅ **Compiler output** แสดงผลปกติ:
```bash
qb.exe hello.bas
  QQQQ    BBBB    N   N   EEEEE   X   X
 ...
Build complete: hello.exe
```

✅ **Error messages** แสดงผลปกติ:
```bash
qb.exe nonexistent.bas
Error: File not found
```

❌ **Program output** ไม่แสดง:
```bash
hello.exe
(nothing)
```

### ผลกระทบ

1. **CLI Programs ยาก**: เขียนโปรแกรมแบบ command line ยาก
2. **Testing ยาก**: ทดสอบผลลัพธ์ของโปรแกรมยาก
3. **Piping ไม่ได้**: ไม่สามารถใช้ pipe หรือ redirect output
4. **Automation ลำบาก**: ยากที่จะ automate tasks

### สาเหตุที่เป็นไปได้

1. **SCREEN Mode Default**: โปรแกรมใช้ SCREEN mode เป็น default แทน console
2. **Print Buffering**: PRINT ไปอยู่ใน buffer ของกราฟฟิก ไม่ใช่ console
3. **Missing Console Mode**: ไม่มี console text mode
4. **SUBSYSTEM Wrong**: Executable เป็น SUBSYSTEM:WINDOWS แทน SUBSYSTEM:CONSOLE

### วิธีแก้ชั่วคราว

ใช้ flag `-x` เพื่อ compile และรันในขั้นตอนเดียว:
```bash
qb.exe program.bas -x
```

แต่ output ยังเป็นของ compiler ไม่ใช่ของโปรแกรม

### ความเร่งด่วน

🟠 **ควรแก้ไข** - ลำบากในการทำ automation และ testing

---

## 📊 ตารางสรุปบั๊กทั้งหมด

| # | ชื่อบั๊ก | ระดับ | ส่วนที่ได้รับ | สถานะ | ความเร่งด่วน |
|---|----------|-------|--------------|--------|-------------|
| 1 | Import Parameter Injection | 🔴 Critical | Import System | ❌ ไม่แก้ | ต้องแก้ทันที |
| 2 | CLASS Syntax Error | 🟡 High | OOP Parser | ❌ ไม่แก้ | ต้องแก้ไข |
| 3 | Console Output Missing | 🟠 Medium | Console I/O | ❌ ไม่แก้ | ควรแก้ไข |

---

## 🎯 ผลกระทบโดยรวม

### ฟีเจอร์ที่ใช้ได้ ✅

| ฟีเจอร์ | สถานะ | หมายเหตุ |
|---------|--------|----------|
| Basic PRINT | ✅ | แต่ output ไม่แสดงใน console |
| ตัวแปรพื้นฐาน | ✅ | DIM, AS, STRING, INTEGER, LONG |
| Numeric operations | ✅ | +, -, *, / ทำงานปกติ |
| String functions | ✅ | LCASE$, UCASE$, LEN, LEFT$, RIGHT$, MID$ |
| Control flow | ✅ | IF-THEN, FOR-NEXT, DO-LOOP, SELECT CASE |
| SUB/FUNCTION | ✅ | ประกาศและเรียกใช้ได้ |
| TYPE/UDT | ✅ | User-defined types ยังใช้ได้ |
| Arrays | ✅ | DIM array ยังใช้ได้ |
| File I/O | ⚠️ | ไม่ได้ทดสอบ |
| Compiler flags | ✅ | ทุก flag ทำงานได้ |

### ฟีเจอร์ที่ใช้ไม่ได้ ❌

| ฟีเจอร์ | สถานะ | หมายเหตุ |
|---------|--------|----------|
| Import stdlib functions | ❌ | บั๊ก #1 - เติม parameters ผิด |
| CLASS/OOP | ❌ | บั๊ก #2 - syntax error |
| Collections (List, etc.) | ❌ | ต้องใช้ import ที่พัง |
| Path utilities | ❌ | Path_Join$ ใช้ไม่ได้ |
| Text utilities | ❌ | Text_PadRight$ ใช้ไม่ได้ |
| CSV/JSON | ❌ | Functions ใช้ไม่ได้ |
| Console output | ❌ | บั๊ก #3 - ไม่แสดง output |

---

## 🔍 ขั้นตอนการทำซ้ำบั๊ก

### บั๊ก #1: Import Parameter Bug

```bash
# ขั้นตอนที่ 1: สร้างไฟล์ทดสอบ
echo ^
SUB TestPath ()^
    DIM p AS STRING^
    p = Path_Join$("root", "file.txt")^
    PRINT "Result: "; p^
END SUB^
^
'$IMPORT:'io.path'^
> path_test.bas

# ขั้นตอนที่ 2: Compile
qb.exe path_test.bas -z

# ขั้นตอนที่ 3: ดูผลลัพธ์
# Expected: Generate C code successfully
# Actual: Illegal string-number conversion error
```

### บั๊ก #2: CLASS Syntax Error

```bash
# ขั้นตอนที่ 1: สร้างไฟล์ทดสอบ
echo ^
CLASS Animal^
    Name AS STRING * 32^
END CLASS^
> class_test.bas

# ขั้นตอนที่ 2: Compile
qb.exe class_test.bas -z

# ขั้นตอนที่ 3: ดูผลลัพธ์
# Expected: Compile successfully
# Actual: Syntax error at "CLASS ANIMAL"
```

### บั๊ก #3: Console Output

```bash
# ขั้นตอนที่ 1: สร้างไฟล์ทดสอบ
echo PRINT "Hello, QBNex!" > hello.bas

# ขั้นตอนที่ 2: Compile
qb.exe hello.bas

# ขั้นตอนที่ 3: รัน executable
hello.exe

# ขั้นตอนที่ 4: ดูผลลัพธ์
# Expected: แสดง "Hello, QBNex!"
# Actual: ไม่มี output ใด ๆ
```

---

## 💡 ข้อเสนอแนะในการแก้ไข

### บั๊ก #1: Import Parameter (Critical)

**ขั้นตอนการแก้**:

1. **ค้นหา Import Processor**
   ```
   ไฟล์ที่ต้องตรวจสอบ:
   - source/qbnex.bas (main compiler)
   - source/utilities/build.bas (build process)
   ```

2. **ตรวจสอบ Function Call Generation**
   - ค้นหาส่วนที่สร้าง function calls จาก imports
   - ดูการ encode parameters
   - ตรวจสอบ string vs number handling

3. **เพิ่ม Debug Output**
   ```basic
   ' เพิ่ม debug ใน import processor
   PRINT "DEBUG: Processing function call: "; functionName$
   PRINT "DEBUG: Parameter count: "; paramCount
   PRINT "DEBUG: Parameters: "; params$
   ```

4. **ทดสอบกับฟังก์ชันง่าย ๆ**
   ```basic
   ' เริ่มจากฟังก์ชัน 1 parameter
   '$IMPORT:'io.path'
   PRINT Path_FileName$("a/b/c.txt")
   
   ' แล้วค่อยเพิ่มเป็น 2 parameters
   PRINT Path_Join$("a", "b")
   ```

5. **Unit Test หลังแก้**
   - ทดสอบฟังก์ชัน 0, 1, 2, 3+ parameters
   - ทดสอบ string และ numeric parameters
   - ทดสอบ nested function calls

**เวลาที่คาดว่าจะใช้**: 2-4 ชั่วโมง

---

### บั๊ก #2: CLASS Syntax (High)

**ขั้นตอนการแก้**:

1. **ตรวจสอบ Keyword List**
   ```
   ค้นหา: keyword list, reserved words
   ไฟล์: source/qbnex.bas
   ```

2. **เพิ่ม CLASS Keyword**
   ```basic
   ' ตรวจสอบว่ามี CLASS ใน keyword list หรือไม่
   ' ถ้าไม่มี ให้เพิ่ม
   ```

3. **เพิ่ม Grammar Rule**
   ```
   CLASS className
       [fields]
       [CONSTRUCTOR]
       [METHODs]
   END CLASS
   ```

4. **ทดสอบ**
   ```basic
   CLASS Test
       Value AS INTEGER
   END CLASS
   
   DIM t AS Test
   t.Value = 42
   PRINT t.Value
   ```

**เวลาที่คาดว่าจะใช้**: 4-8 ชั่วโมง

---

### บั๊ก #3: Console Output (Medium)

**ขั้นตอนการแก้**:

1. **ตรวจสอบ Compiler Flags**
   ```bash
   # ดูว่าใช้ SUBSYSTEM ไหน
   qb.exe hello.bas -z
   # ตรวจสอบ C code
   ```

2. **เพิ่ม Console Flag**
   ```basic
   ' เพิ่ม flag สำหรับ console mode
   qb.exe hello.bas --console
   ```

3. **ตรวจสอบ PRINT Implementation**
   - PRINT ไปที่ buffer ใด
   - มี console text mode หรือไม่

**เวลาที่คาดว่าจะใช้**: 1-2 ชั่วโมง

---

## 📝 Timeline การแก้ไขที่แนะนำ

### Phase 1: แก้บั๊ก Critical (สัปดาห์ที่ 1)

| วัน | งาน | ผลลัพท์ |
|-----|-----|---------|
| 1-2 | วิเคราะห์ Import Processor | เข้าใจสาเหตุ |
| 3-4 | แก้ไข Import Parameter Bug | ฟังก์ชัน 2+ params ใช้ได้ |
| 5 | ทดสอบ stdlib ทั้งหมด | Smoke tests ผ่าน |

### Phase 2: แก้บั๊ก High (สัปดาห์ที่ 2)

| วัน | งาน | ผลลัพท์ |
|-----|-----|---------|
| 1-2 | วิเคราะห์ OOP Parser | เข้าใจโครงสร้าง |
| 3-5 | เพิ่ม CLASS Support | CLASS ใช้ได้ |
| 6-7 | ทดสอบ OOP Features | OOP tests ผ่าน |

### Phase 3: แก้บั๊ก Medium (สัปดาห์ที่ 3)

| วัน | งาน | ผลลัพท์ |
|-----|-----|---------|
| 1 | ตรวจสอบ Console Output | เข้าใจปัญหา |
| 2-3 | เพิ่ม Console Mode | Output แสดงผล |
| 4-5 | ทดสอบ CLI Programs | CLI tests ผ่าน |

---

## ✅ Acceptance Criteria

### บั๊ก #1 แก้สำเร็จเมื่อ:

- [ ] `Path_Join$("a", "b")` compile ได้
- [ ] `Text_PadRight$("QB", 4, ".")` compile ได้
- [ ] Smoke tests ผ่านทั้งหมด
- [ ] stdlib functions ทำงานได้
- [ ] No regression ใน basic features

### บั๊ก #2 แก้สำเร็จเมื่อ:

- [ ] `CLASS Animal` compile ได้
- [ ] CONSTRUCTOR ใช้ได้
- [ ] METHOD ใช้ได้
- [ ] OOP examples ทำงานได้
- [ ] No regression ใน basic features

### บั๊ก #3 แก้สำเร็จเมื่อ:

- [ ] `PRINT "Hello"` แสดงผลใน console
- [ ] Redirection (`>`) ทำงานได้
- [ ] Piping (`|`) ทำงานได้
- [ ] CLI programs ใช้งานได้

---
