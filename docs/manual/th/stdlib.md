# ไลบรารีมาตรฐาน

[สารบัญคู่มือ](../README.md) · [English](../en/stdlib.md)

ไลบรารีมาตรฐานเป็นโค้ด BASIC อยู่ใน `compiler/stdlib` และ import ทีละโมดูลด้วยชื่อสั้นๆ

```basic
'$IMPORT:'collections'
'$IMPORT:'regex'
```

วางบรรทัด import ตรงไหนของไฟล์หลักก็ได้ (ดู[ส่วนขยายของภาษา](language.md#โมดูลไลบรารีมาตรฐาน-importname))
แต่ละโมดูลถูกโหลดเพียงครั้งเดียว จะ import ในลำดับใดก็ได้ และแม้โมดูลอื่นจะ
import โมดูลนั้นซ้ำก็ไม่เป็นไร `'$IMPORT:'qbnex'` import ทุกโมดูลในครั้งเดียว

<!-- docs-test: output -->
```basic
'$IMPORT:'qbnex'
DIM words AS List
words.Split "pear fig apple", " "
words.Sort
PRINT words.Join$(" "); " "; Str_PadLeft$("7", 3, "0"); " "; Path_Ext$("game.bas")
```

<!-- expected output -->
```text
apple fig pear 007 .bas
```

procedure ตั้งชื่อแบบ `Module_Name` และมี suffix บอกชนิดติดมาด้วย
(`Str_Replace$`, `Regex_Test&`) จะเรียกแบบมีหรือไม่มี suffix ก็ได้ในที่ที่ QBasic ยอม
TYPE ของไลบรารี `List`, `Dict`, `Set`, `Url` และ `Result` ใช้ผ่าน
[การเรียก method](language.md#การเรียก-method-บนค่าของ-type):
`names.Add "Ann"` คือ `List_Add names, "Ann"` และ `names.Count` คือ
`List_Count&(names)` เขียนแบบไหนก็ได้ language server จะแสดงบรรทัดประกาศและ
ชื่อโมดูลของ procedure ในไลบรารีเมื่อเอาเมาส์ไปชี้

สถานะ: ทุกโมดูลในหน้านี้ **ใช้ได้แล้ว** โมดูล native (`regex`, `hash`, `zip`,
`http`, `utf8`, `datetime`) เรียกไลบรารี Rust `crates/runtime/ext` (Rust ล้วน
ไม่ใช้ OpenSSL) ซึ่งถูกลิงก์เข้าไปในโปรแกรม native ทุกตัว โมดูลกลุ่มนี้ใช้ไม่ได้ใน
[รุ่นเว็บ](web.md) ส่วนโมดูลที่เขียนด้วย BASIC ใช้ได้ทั้งในโปรแกรม native และในรุ่นเว็บ

## ภาพรวม

| โมดูล | ทำอะไร |
| --- | --- |
| [`core`](#core) | `LastError$`: สาเหตุที่การเรียกไลบรารีครั้งล่าสุดล้มเหลว (ใช้ร่วมกันทุกโมดูล) |
| [`collections`](#collections) | `List`, `Dict` และ `Set` ของ string |
| [`strings`](#strings) | ตัวช่วยจัดการข้อความ `Str_*`: มีอยู่ใน, ขึ้นต้น/ลงท้ายด้วย, แทนที่, ตัดช่องว่าง, เติมให้ครบ, ทำซ้ำ |
| [`json`](#json) | JSON ตาม RFC 8259: parse, อ่านด้วย path, สร้าง, แปลงกลับเป็นข้อความ |
| [`url`](#url) | TYPE `Url` (แยกส่วน, query, ประกอบกลับ), encode/decode, การ resolve ตาม RFC 3986 |
| [`path`](#path) | path ของไฟล์: ต่อ path, ชื่อไฟล์, โฟลเดอร์, นามสกุล, ตรวจว่ามีอยู่ |
| [`csv`](#csv) | แปลงแถว CSV หนึ่งแถว (RFC 4180) กับ `List` ไปมา |
| [`math`](#math) | `Math_Min#`, `Math_Max#`, `Math_Clamp#`, `Math_Lerp#`, `Math_Pi#`, แปลงองศา/เรเดียน |
| [`sys`](#sys) | แพลตฟอร์ม, environment variable, โฟลเดอร์ home, argument ของ command line |
| [`result`](#result) | TYPE `Result`: ค่าหรือ error ที่ส่งกลับจาก SUB ที่เขียนเอง |
| [`regex`](#regex) | regular expression (native) |
| [`hash`](#hash) | MD5, SHA-1/256/512, CRC32, Base64, hex (native) |
| [`zip`](#zip) | อ่าน แตก และสร้างไฟล์ zip (native) |
| [`http`](#http) | HTTP/HTTPS client (native) |
| [`utf8`](#utf8) | ข้อความ UTF-8 แบบนับเป็นตัวอักษร, ตัวพิมพ์ใหญ่เล็ก, normalization, CP437 (native) |
| [`datetime`](#datetime) | วันเวลาในรูปมิลลิวินาทีนับจากปี 1970: ISO 8601, จัดรูปแบบ, parse, แยกส่วน (native) |
| `qbnex` | ทุกโมดูลใน import เดียว |

`ext` เป็นส่วนเชื่อมต่อที่โมดูล native ใช้ร่วมกัน และ `oop.class` /
`oop.interface` เป็น runtime ที่ [CLASS](language.md) ใช้ โปรแกรมไม่ต้องเรียกโมดูลเหล่านี้โดยตรง

## core

ทุกโมดูลแจ้ง error แบบเดียวกัน: การเรียกที่ล้มเหลวจะคืน 0 หรือ string ว่าง
และตั้ง `LastError$` เป็นสาเหตุ ส่วนการเรียกที่สำเร็จจะตั้งกลับเป็น "" ทุกโมดูล import `core`

<!-- docs-test: output -->
```basic
'$IMPORT:'url'
DIM u AS Url
u.Parse "http://example.com:99999/"
PRINT u.port; "["; LastError$; "]"
u.Parse "http://example.com:80/"
PRINT u.port; "["; LastError$; "]"
```

<!-- expected output -->
```text
 0 [invalid port: 99999]
 80 []
```

## collections

<!-- docs-test: output -->
```basic
'$IMPORT:'collections'
DIM fruit AS List
fruit.Add "pear": fruit.Add "fig"
fruit.Insert 0, "apple"
PRINT fruit.Count; fruit.Get$(0); " "; fruit.Get$(-1)
fruit.Sort
PRINT fruit.Join$(", ")
PRINT fruit.IndexOf("fig"); fruit.Has("kiwi")
last$ = fruit.Pop$
PRINT last$; fruit.Count
fruit.Split "a,b,c"
PRINT fruit.Join$(" | ")

DIM ages AS Dict
ages.Set "Ann", "42": ages.Set "Bob", "7"
PRINT ages.Get$("Ann"); " "; ages.Get$("Zed", "?")
PRINT ages.Has("Bob"); ages.Has("Zed")
FOR i = 0 TO ages.Count - 1
    PRINT ages.Key$(i); "="; ages.Value$(i)
NEXT

DIM seen AS Set
seen.Add "x": seen.Add "y": seen.Add "x"
PRINT seen.Count; seen.Join$
```

<!-- expected output -->
```text
 3 apple fig
apple, fig, pear
 1  0
pear 2
a | b | c
42 ?
-1  0
Ann=42
Bob=7
 2 x, y
```

ตัวแปร `List`, `Dict` หรือ `Set` ที่ประกาศใหม่ใช้ได้ทันที (ไม่ต้อง init)
ทุกรายการเป็น string: เก็บตัวเลขด้วย `STR$` และอ่านกลับด้วย `VAL` index
เริ่มที่ 0 และ index ติดลบนับจากท้าย (-1 = รายการสุดท้าย) สำเนา (`b = a`)
คือ collection เดียวกัน เหมือนใน Python `Free` คืนหน่วยความจำก่อนเวลา
ถ้าไม่เรียก หน่วยความจำจะอยู่จนจบโปรแกรม list ใช้เป็น field ใน TYPE ที่เขียนเอง
เป็นสมาชิกของ array และเป็นพารามิเตอร์ของ SUB ได้

- **List**: `Add item$`, `Insert index, item$`, `Count`, `Get$(index)` (ได้ ""
  เมื่อ index อยู่นอก list), `Set index, item$`, `Pop$` (เอารายการสุดท้ายออกและ
  คืนค่านั้น `Pop$(0)` คือรายการแรก), `RemoveAt index`, `Remove item$` (รายการแรก
  ที่เท่ากัน), `IndexOf(item$)` (-1 เมื่อไม่มี), `Has(item$)`, `Join$(sep$)`
  (ค่าเริ่มต้น `", "`), `Split text$, sep$` (แทนที่รายการเดิมทั้งหมด ค่าเริ่มต้น
  `","` ถ้าเป็น "" จะแยกทีละตัวอักษร), `Sort` (เรียงตามไบต์ `Sort -1` เรียงจากมาก
  ไปน้อย), `Reverse`, `Clear`, `Free` ทำ stack ได้ด้วย `Add` + `Pop$` ทำ queue
  ด้วย `Add` + `Pop$(0)` และต่อ string ทีละส่วนด้วย `Add` + `Join$("")`
- **Dict**: `Set key$, value$` (key ใหม่ต่อท้าย key เก็บลำดับตามที่ใส่),
  `Get$(key$, fallback$)`, `Has(key$)`, `Remove key$`, `Count`, `Key$(index)`,
  `Value$(index)`, `Clear`, `Free`
- **Set**: `Add item$` (เพิ่มเฉพาะเมื่อยังไม่มี), `Has(item$)`, `Remove item$`,
  `Count`, `Join$(sep$)` (ตามลำดับที่ใส่), `Clear`, `Free`

## strings

<!-- docs-test: output -->
```basic
'$IMPORT:'strings'
t$ = Str_Trim$("   hello world  ")
PRINT "["; t$; "]"
PRINT Str_Has(t$, "lo w"); Str_StartsWith(t$, "he"); Str_EndsWith(t$, "x")
PRINT Str_Replace$("a-b-c", "-", "+")
PRINT Str_PadLeft$("7", 3, "0"); " "; Str_PadRight$("ab", 5, "."); "|"
PRINT Str_Repeat$("ab", 3)
```

<!-- expected output -->
```text
[hello world]
-1 -1  0
a+b+c
007 ab...|
ababab
```

`Str_Has&` ("" อยู่ในทุกข้อความ), `Str_StartsWith&`, `Str_EndsWith&`,
`Str_Replace$` (แทนที่ทุกจุดที่ตรง), `Str_Trim$` (ตัดช่องว่าง tab และขึ้นบรรทัดใหม่
ออกทั้งสองด้าน), `Str_PadLeft$` / `Str_PadRight$` (ค่าเริ่มต้นเติมด้วยช่องว่าง),
`Str_Repeat$` การแยกและต่อข้อความใช้ `List.Split` และ `List.Join$`
([collections](#collections)) ส่วนการจัดการข้อความทีละตัวอักษรอยู่ใน [utf8](#utf8)

## json

<!-- docs-test: output -->
```basic
'$IMPORT:'json'
q$ = CHR$(34)                                 ' QBasic strings have no escapes
text$ = "{'name': 'QBNex', 'tags': ['basic', 3], 'user': {'age': 42, 'admin': true}}"
FOR i = 1 TO LEN(text$)                       ' turn every ' into "
    IF MID$(text$, i, 1) = "'" THEN MID$(text$, i, 1) = q$
NEXT
root = Json_Parse(text$)
IF root = 0 THEN PRINT LastError$; " at byte"; Json_ErrorPos&: END
PRINT Json_Str$(root, "name")
PRINT Json_Num#(root, "user.age"); Json_Bool%(root, "user.admin")
PRINT Json_Type$(root, "tags"); Json_Count(root, "tags")
PRINT Json_Str$(root, "tags[0]"); " "; Json_Str$(root, "tags.1")
PRINT Json_Key$(root, 2); Json_Has%(root, "user.email")
o = Json_Object
Json_SetStr o, "name", "Ann"
Json_SetNum o, "age", 42
a = Json_Array
Json_PushStr a, "x": Json_PushBool a, 0
Json_Set o, "list", a
PRINT Json_Stringify$(o)
Json_Free o: Json_Free root
```

<!-- expected output -->
```text
QBNex
 42 -1
array 2
basic 3
user 0
{"name":"Ann","age":42,"list":["x",false]}
```

ค่าทุกค่าเป็น handle (INTEGER, 0 = ไม่มี) อ่านค่าด้วย path ที่ประกอบด้วยชื่อสมาชิก
และ index ที่เริ่มจาก 0 เช่น `"user.tags.0"` หรือ `"items[2].id"` (`""` คือค่านั้นเอง)

- **Parse**: `Json_Parse%(text$)` คืน root หรือคืน 0 พร้อม `LastError$` และ
  `Json_ErrorPos&` (ตำแหน่งไบต์เริ่มที่ 1) string เป็นไบต์: escape `\uXXXX`
  (รวม surrogate pair) กลายเป็น UTF-8 ตัวเลขตาม grammar อย่างเคร่งครัด ซ้อนได้ลึกสุด
  500 ชั้น และการ parse ที่ล้มเหลวจะไม่เหลือค่าค้างไว้
- **อ่าน**: `Json_Get%(v, path$)` (handle), `Json_Str$` (string ได้ข้อความของมัน
  ตัวเลขหรือ `true`/`false` ได้ข้อความ JSON ส่วน null หรือไม่มีค่าได้ ""),
  `Json_Num#`, `Json_Bool%`, `Json_Type$` (`"object"`, `"array"`, `"string"`,
  `"number"`, `"boolean"`, `"null"` และ "" เมื่อไม่มีค่า), `Json_Has%`,
  `Json_Count%`, `Json_Key$(v, index[, path$])`
- **สร้าง**: `Json_Object%`, `Json_Array%`, `Json_Set` (ค่าที่เป็น object หรือ
  array), `Json_SetStr`, `Json_SetNum`, `Json_SetBool`, `Json_SetNull`,
  `Json_Remove`, `Json_Push`, `Json_PushStr`, `Json_PushNum`, `Json_PushBool`
  object เก็บลำดับสมาชิกไว้ การตั้งค่า key ที่มีอยู่แล้วจะแทนค่าเดิม และการเพิ่มค่าที่
  อยู่ใน array หรือ object อื่นอยู่แล้วจะเพิ่มเป็นสำเนา
- **เขียน**: `Json_Stringify$(v)` ส่วน `Json_Stringify$(v, 2)` ย่อหน้าทีละ 2 ช่อง
  ตัวเลขที่ parse มาเมื่อแปลงกลับจะได้ข้อความเดิมทุกตัวอักษร
- **หน่วยความจำ**: มีค่าอยู่พร้อมกันได้สูงสุด 32767 ค่า `Json_Free v` คืนค่านั้น
  พร้อมทุกค่าที่อยู่ข้างใน (ถอดออกจาก parent ก่อน) และ `Json_FreeAll` คืนทุกค่า
  เมื่อ pool เต็ม `Json_Parse%` คืน 0 และ `LastError$` เป็น "out of JSON value space"

## url

<!-- docs-test: output -->
```basic
'$IMPORT:'url'
DIM u AS Url
u.Parse "https://user@Example.com:8080/a/b.txt?q=1&s=a+b#top"
PRINT u.hostname; u.port; u.pathname; " "; u.hash
PRINT u.Query$("s")
u.SetQuery "q", "two words"
PRINT u.Format$
PRINT Url_Encode$("a b/c"); " "; Url_Decode$("a%20b")
PRINT Url_Resolve$("http://a/b/c/d;p?q", "../g")
PRINT Url_FromPath$("C:\My Files\a.txt")
```

<!-- expected output -->
```text
example.com 8080 /a/b.txt #top
a b
https://user@example.com:8080/a/b.txt?q=two%20words&s=a+b#top
a%20b%2Fc a b
http://a/b/g
file:///C:/My%20Files/a.txt
```

field ของ `Url` ตั้งชื่อตาม `URL` ของ JavaScript: `href`, `protocol`
(`"https:"`), `username`, `password`, `hostname`, `port` (LONG), `pathname`,
`search`, `hash`, `host`, `origin` ทุก field เป็น string ความยาวแปรผัน
ส่วนประกอบยาวเท่าใดก็เก็บได้ครบ method: `Parse text$`, `Query$(key$)` (ถอดรหัส
ให้แล้ว `+` คือช่องว่าง), `SetQuery key$, value$` (แทนค่า key เดิมหรือต่อท้าย
และอัปเดต `search` กับ `href`), `Format$` port ที่ไม่ใช่ตัวเลข 0 ถึง 65535
ถือเป็น error: `port` เป็น 0 และ `LastError$` บอกสาเหตุ

`Url_Encode$` encode ทุกตัวยกเว้นตัวอักษร unreserved ตาม RFC 3986 (เหมือน
`encodeURIComponent` ของ JavaScript), `Url_Decode$` ถอด `%XX`,
`Url_Resolve$(base$, ref$)` resolve reference (RFC 3986 หัวข้อ 5.2 เหมือน
`urljoin` ของ Python) และ `Url_FromPath$` / `Url_ToPath$` แปลงระหว่าง path
ของไฟล์กับ URL แบบ `file:///`

## path

<!-- docs-test: output -->
```basic
'$IMPORT:'path'
f$ = Path_Join$("docs", "manual", "stdlib.md")   ' docs\manual\stdlib.md on Windows
PRINT Path_Base$(f$); " "; Path_Stem$(f$); " "; Path_Ext$(f$)
PRINT Path_Dir$(f$) = Path_Join$("docs", "manual")
PRINT f$ = "docs" + Path_Sep$ + "manual" + Path_Sep$ + "stdlib.md"
PRINT Path_Exists("no-such-file.txt")
```

<!-- expected output -->
```text
stdlib.md stdlib .md
-1
-1
 0
```

รับได้ทั้ง `/` และ `\` ผลลัพธ์ใช้ตัวคั่นของเครื่อง (`Path_Sep$`: `\` บน Windows
และ `/` บนระบบอื่น) `Path_Join$` รับ 2 ถึง 4 ส่วน นอกจากนี้มี `Path_Normalize$`
(ใช้ตัวคั่นของเครื่อง ไม่มีตัวคั่นซ้อน) และ `Path_Exists&` (-1 เมื่อเป็นไฟล์หรือโฟลเดอร์)

## csv

<!-- docs-test: output -->
```basic
'$IMPORT:'csv'
DIM row AS List
row.Add "plain": row.Add "a,b": row.Add "say " + CHR$(34) + "hi" + CHR$(34)
line$ = Csv_Row$(row)
PRINT line$
Csv_Parse row, line$
PRINT row.Count; row.Get$(2)
Csv_Parse row, "x;y;z", ";"
PRINT row.Join$("/")
```

<!-- expected output -->
```text
plain,"a,b","say ""hi"""
 3 say "hi"
x/y/z
```

`Csv_Row$(row[, sep$])` เขียนรายการใน `List` เป็นหนึ่งบรรทัด (ช่องที่มีตัวคั่น
เครื่องหมายคำพูด หรือขึ้นบรรทัดใหม่จะถูกครอบด้วยเครื่องหมายคำพูด),
`Csv_Parse row, text$[, sep$]` แทนรายการเดิมด้วย field ของบรรทัดนั้น และ
`Csv_Quote$` ครอบช่องเดียว การ import `csv` จะ import `collections` ให้ด้วย

## math

<!-- docs-test: output -->
```basic
'$IMPORT:'math'
PRINT Math_Min#(3, 8); Math_Max#(3, 8); Math_Clamp#(15, 0, 10)
PRINT Math_Lerp#(0, 10, .25); Math_Degrees#(Math_Pi#); Math_Radians#(180) = Math_Pi#
```

<!-- expected output -->
```text
 3  8  10
 2.5  180 -1
```

`Math_Lerp#(a, b, t)` คือ `a + (b - a) * t` ส่วน `Math_Radians#` และ
`Math_Degrees#` แปลงมุม ฟังก์ชันอื่นมีในตัวภาษาอยู่แล้ว (`ABS`, `SQR`, `INT`, ...)

## sys

<!-- docs-test: run -->
```basic
'$IMPORT:'sys'
PRINT Sys_Platform$; Sys_Is64Bit
PRINT Sys_Env$("EDITOR", "notepad")
PRINT Sys_HasEnv("PATH"), Sys_Home$
PRINT Sys_ArgCount; "arguments after "; Sys_Arg$(0)
FOR i = 1 TO Sys_ArgCount: PRINT Sys_Arg$(i): NEXT
```

`Sys_Platform$` เป็น `"windows"`, `"linux"` หรือ `"macos"`,
`Sys_Env$(name$[, fallback$])` อ่าน environment variable, `Sys_HasEnv&`
บอกว่าตั้งค่าไว้หรือไม่, `Sys_Home$` คือโฟลเดอร์ home ของผู้ใช้, `Sys_Is64Bit&`
เป็น -1 ในโปรแกรม 64-bit, `Sys_ArgCount&` นับ argument ที่ตามหลังชื่อโปรแกรม และ
`Sys_Arg$(index[, fallback$])` คืน argument ตัวนั้น (0 = ตัวโปรแกรม)

## result

<!-- docs-test: output -->
```basic
'$IMPORT:'result'
DIM r AS Result
LoadConfig r, "missing.ini"
IF r.Failed THEN r.Context "starting": PRINT r.Message; r.Code ELSE PRINT r.Value
LoadConfig r, ""
IF r.Failed THEN PRINT r.Message ELSE PRINT r.Value
SYSTEM

SUB LoadConfig (r AS Result, file AS STRING)
    IF file = "" THEN r.Ok "defaults": EXIT SUB
    IF _FILEEXISTS(file) = 0 THEN r.Fail file + " is missing", 53: EXIT SUB
    r.Ok "loaded " + file
END SUB
```

<!-- expected output -->
```text
starting: missing.ini is missing 53
defaults
```

`Result` ใช้ส่งค่ากลับจาก SUB ที่เขียนเองแทนการยก `ERROR` อ่าน field ได้โดยตรง:
`Failed` (-1 หรือ 0), `Value`, `Message`, `Code` และใช้ method ตั้งค่า:
`Ok [value$]` (สำเร็จ ล้าง error ก่อนหน้า), `Fail message$[, code]` (รหัสเริ่มต้น
คือ 1) และ `Context where$` (เติม `"where: "` หน้าข้อความ error เพื่อบอกว่าเกิดที่ไหน)

## regex

<!-- docs-test: output -->
```basic
'$IMPORT:'regex'
PRINT Regex_Test("^\d{3}-\d{4}$", "555-1234")
m$ = Regex_Find$("(\w+)@(\w+)\.com", "mail bob@example.com")
PRINT m$; Regex_Pos; Regex_Group$(1); " "; Regex_Group$(2)
PRINT Regex_Replace$("(\d+)", "a1b22", "<$1>")
PRINT Regex_Replace$("\d", "a1b22", "#", -1)
DIM parts AS List
Regex_Split "\s*,\s*", "a , b,c", parts
PRINT parts.Join$("|")
Regex_FindAll "\d+", "1 22 333", parts
PRINT parts.Count; parts.Get$(-1)
```

<!-- expected output -->
```text
-1
bob@example.com 6 bob example
a<1>b<22>
a#b22
a|b|c
 3 333
```

ใช้ syntax ของ crate `regex` ใน Rust: `\d \w \s`, class, `(group)`,
`(?<name>...)`, `(?i)` เพื่อไม่สนตัวพิมพ์ใหญ่เล็ก ไม่มี backreference และ look-around
ตำแหน่งนับเป็นไบต์เริ่มที่ 1 เหมือน `INSTR`

- `Regex_Test&(pattern$, text$)`: -1 เมื่อข้อความมีส่วนที่ตรง
- `Regex_Find$(pattern$, text$[, start])`: ส่วนที่ตรงส่วนแรกที่ไบต์ `start` หรือหลังจากนั้น
  ("" เมื่อไม่มี) จากนั้น `Regex_Pos&` คือตำแหน่งของมัน (0 = ไม่ตรง) และ
  `Regex_Group$(n)` คือ group ของมัน (0 = ทั้งส่วนที่ตรง)
- `Regex_Replace$(pattern$, text$, replacement$[, firstOnly])`: แทนทุกจุดที่ตรง
  (เฉพาะจุดแรกเมื่อให้ -1) ใน replacement ใช้ `$1`, `${name}` และ `$$` ได้
- `Regex_Split pattern$, text$, parts` และ `Regex_FindAll pattern$, text$, found`
  ใส่ผลลงใน `List`

pattern ที่ผิดจะได้ 0 หรือ "" และ `LastError$` บอกสาเหตุ

## hash

<!-- docs-test: output -->
```basic
'$IMPORT:'hash'
PRINT Hash_Sha256$("abc")
PRINT Hash_Md5$("abc")
PRINT Hash_Sha1$("abc")
PRINT LEFT$(Hash_Sha512$("abc"), 16)
PRINT LCASE$(HEX$(Hash_Crc32~&("123456789")))
b$ = Base64_Encode$("bytes?>")
PRINT b$; " "; Base64_Encode$("bytes?>", -1); " "; Base64_Decode$(b$)
PRINT Hex_Encode$("AZ"); " "; Hex_Decode$("41 42")
```

<!-- expected output -->
```text
ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
900150983cd24fb0d6963f7d28e17f72
a9993e364706816aba3e25717850c26c9cd0d89d
ddaf35a193617aba
cbf43926
Ynl0ZXM/Pg== Ynl0ZXM_Pg bytes?>
415a AB
```

digest เป็น hex ตัวพิมพ์เล็กของไบต์ใน string `Hash_Crc32~&` คืน CRC32 เป็นตัวเลข
(แบบที่ zip และ PNG ใช้) `Base64_Encode$(s$, -1)` ใช้ตัวอักษรแบบ URL-safe
และไม่เติม padding ส่วน `Base64_Decode$` อ่านได้ทั้งสองแบบและข้ามการขึ้นบรรทัดใหม่
(ข้อความที่ไม่ถูกต้องได้ "" และ `LastError$`) `Hex_Decode$` ข้ามช่องว่าง

## zip

<!-- docs-test: output -->
```basic
'$IMPORT:'zip'
z& = Zip_Create&("out.zip")
Zip_Write z&, "hello.txt", "hello"
Zip_AddFolder z&, "docs/"
Zip_Write z&, "docs/note.txt", "a note"
Zip_Close z&
DIM names AS List
Zip_List "out.zip", names
PRINT names.Join$(", ")
PRINT Zip_Read$("out.zip", "docs/note.txt")
Zip_Extract "out.zip", "outdir"
PRINT _FILEEXISTS("outdir/docs/note.txt")
x$ = Zip_Read$("out.zip", "missing.txt")
PRINT LastError$
```

<!-- expected output -->
```text
hello.txt, docs/, docs/note.txt
a note
-1
missing.txt: specified file not found in archive
```

ชื่อสมาชิกใช้ `/` คั่นโฟลเดอร์ `Zip_Create&` คืน handle (0 เมื่อเกิด error)
`Zip_Write` เพิ่มสมาชิกจาก string, `Zip_AddFile z&, member$, file$` เพิ่มจากไฟล์
และ `Zip_AddFolder` เพิ่มโฟลเดอร์ว่าง `Zip_Close` เขียนไฟล์ zip ให้เสร็จ
`Zip_List` ใส่ชื่อสมาชิกลงใน `List`, `Zip_Read$` คืนสมาชิกหนึ่งตัว และ
`Zip_Extract` เขียนทุกสมาชิกลงในโฟลเดอร์ (ไม่ยอมแตกชื่ออย่าง `../x`) เมื่อเกิด error
`LastError$` จะบอกสาเหตุ

## http

```basic
'$IMPORT:'http'
page$ = Http_Get$("https://example.com/", 5000)           ' 5 s timeout
IF Http_Status <> 200 THEN PRINT "failed:"; Http_Status; LastError$ ELSE PRINT LEN(page$); "bytes"
PRINT Http_Header$("content-type")
reply$ = Http_Post$("https://example.com/api", "{}")      ' sent as application/json
reply$ = Http_Request$("PUT", "https://example.com/api", "data", "Authorization: Bearer x")
```

การเรียกแต่ละครั้งคืน body ของคำตอบ (เก็บไบต์ได้ทุกค่า) `Http_Status&` คือ
status (200, 404, ...) หรือ -1 เมื่อไม่มีคำตอบกลับมา (URL ผิด, ถูกปฏิเสธ, DNS,
TLS, หมดเวลา: ดู `LastError$`) `Http_Header$(name$)` คืน header หนึ่งตัวของ
คำตอบล่าสุด (ไม่สนตัวพิมพ์ใหญ่เล็ก) และ `Http_Headers$` คืนทั้งหมด
`Http_Post$(url$, body$[, contentType$[, timeoutMs]])` และ
`Http_Request$(method$, url$[, body$[, headers$[, timeoutMs]]])` (headers เป็น
บรรทัด `"Name: value"`) ครบชุด timeout เป็น 0 หมายถึง 30 วินาทีสำหรับทั้ง request
HTTPS ใช้ rustls กับ root certificate ของ Mozilla และตาม redirect ให้เอง

## utf8

<!-- docs-test: output -->
```basic
'$IMPORT:'utf8'
s$ = "สวัสดี"
PRINT LEN(s$); Utf8_Len(s$)                     ' bytes, characters
w$ = "naïve café"
PRINT Utf8_Mid$(w$, 3, 3); " "; Utf8_Left$(w$, 5); " "; Utf8_Right$(w$, 4)
PRINT Utf8_Upper$("straße"); " "; Utf8_Lower$("ÀB")
PRINT Utf8_Asc(s$); Utf8_Chr$(3647); Utf8_IsValid(s$); Utf8_IsValid(CHR$(255))
PRINT Utf8_Normalize$("e" + Utf8_Chr$(769)) = Utf8_Chr$(233)
PRINT Utf8_FromCp437$(CHR$(201) + CHR$(205) + CHR$(187)); " "; Utf8_ToCp437$("é") = CHR$(130)
```

<!-- expected output -->
```text
 18  6
ïve naïve café
STRASSE àb
 3626 ฿-1  0
-1
╔═╗ -1
```

string ของ BASIC เป็นไบต์ ฟังก์ชันกลุ่มนี้มองเป็นข้อความ UTF-8 และนับทีละตัวอักษร
ไบต์ที่ไม่ใช่ UTF-8 ที่ถูกต้องนับเป็นหนึ่งตัวอักษรต่อไบต์และถูกเก็บไว้
`Utf8_Mid$(s$, start[, count])` ทำงานเหมือน `MID$`, `Utf8_Upper$` /
`Utf8_Lower$` ใช้การแปลงตัวพิมพ์ของ Unicode เต็มรูปแบบ,
`Utf8_Normalize$(s$[, form$])` ให้ `"NFC"` (ค่าเริ่มต้น), `"NFD"`, `"NFKC"` หรือ
`"NFKD"`, `Utf8_Asc&(s$[, index])` และ `Utf8_Chr$` ทำงานเหมือน `ASC` และ `CHR$`
แต่ใช้ code point, `Utf8_FromCp437$` แปลงข้อความ DOS เป็น UTF-8
(`Utf8_FromCp437$(s$, -1)` แปลงไบต์ 1-31 และ 127 เป็นสัญลักษณ์ของ PC ด้วย) และ
`Utf8_ToCp437$` แปลงกลับ (ได้ `?` เมื่อ CP437 ไม่มีตัวอักษรนั้น)

## datetime

<!-- docs-test: output -->
```basic
'$IMPORT:'datetime'
t# = Date_Parse#("2024-03-05T07:08:09Z")
PRINT Date_Iso$(t#)
PRINT Date_Format$(t#, "%A %d %B %Y, %H:%M")
PRINT Date_Year(t#); Date_Month(t#); Date_Day(t#); Date_Weekday(t#); Date_IsoWeek(t#)
PRINT Date_Iso$(Date_AddDays#(t#, 10))
PRINT Date_DiffDays#(Date_Make#(2024, 12, 25), Date_Make#(2024, 3, 5))
PRINT Date_Format$(Date_Parse#("05/03/2024", "%d/%m/%Y"), "%F")
bad# = Date_Parse#("not a date")
PRINT Date_IsValid(bad#); Date_IsValid(Date_Now#)
```

<!-- expected output -->
```text
2024-03-05T07:08:09.000Z
Tuesday 05 March 2024, 07:08
 2024  3  5  2  10
2024-03-15T07:08:09.000Z
 295
2024-03-05
 0 -1
```

เวลาหนึ่งจุดเป็น DOUBLE: มิลลิวินาทีนับจาก 1970-01-01 00:00:00 UTC เหมือน
`Date.now()` ของ JavaScript ทุกฟังก์ชันทำงานแบบ UTC ถ้าต้องการเขตเวลาของเครื่อง
ให้ส่ง -1 เป็น argument สุดท้าย (`isLocal`) เช่น `Date_Iso$(t#, -1)`
(`"2024-03-05T14:08:09.000+07:00"` ในเขตเวลา UTC+7)

- `Date_Now#`, `Date_Make#(y, month, day[, h, mi, s, ms[, isLocal]])`,
  `Date_Parse#(text$[, pattern$[, isLocal]])` (ISO 8601 / RFC 3339 เมื่อ pattern เป็น "")
- `Date_Iso$`, `Date_Format$(t#[, pattern$[, isLocal]])` (ค่าเริ่มต้น
  `"%Y-%m-%d %H:%M:%S"`)
- แยกส่วน: `Date_Year&`, `Date_Month&` (1-12), `Date_Day&`, `Date_Hour&`,
  `Date_Minute&`, `Date_Second&`, `Date_Millisecond&`, `Date_Weekday&`
  (0 = วันอาทิตย์), `Date_YearDay&`, `Date_IsoWeek&`
- `Date_AddDays#(t#, days)`, `Date_DiffDays#(a#, b#)`, `Date_Offset&(t#)`
  (จำนวนวินาทีที่เขตเวลาของเครื่องเร็วกว่า UTC)
- ข้อมูลที่ไม่ถูกต้องได้ NaN (`Date_IsValid&(t#)` = 0) และตั้ง `LastError$`

รหัสจัดรูปแบบ: `%Y` ปี, `%m` เดือน 01-12, `%d` วัน, `%H` ชั่วโมง 00-23, `%I`
01-12, `%M` นาที, `%S` วินาที, `%3f` มิลลิวินาที, `%p` AM/PM, `%A`/`%a`
ชื่อวัน, `%B`/`%b` ชื่อเดือน, `%j` วันที่ของปี, `%V` สัปดาห์แบบ ISO, `%z`
`+0700`, `%:z` `+07:00`, `%s` วินาทีแบบ Unix, `%F` = `%Y-%m-%d`, `%T` =
`%H:%M:%S`, `%%` เครื่องหมายเปอร์เซ็นต์
