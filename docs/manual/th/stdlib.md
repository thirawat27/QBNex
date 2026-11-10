# ไลบรารีมาตรฐาน

[สารบัญคู่มือ](../README.md) · [English](../en/stdlib.md)

ไลบรารีมาตรฐานเป็นโค้ด BASIC อยู่ใน `compiler/stdlib` และ import ทีละโมดูล

<!-- docs-test: no-ir ext.* modules are not available with --ir -->
```basic
'$IMPORT:'json'
'$IMPORT:'ext.regex'
```

วางบรรทัด import ตรงไหนของไฟล์หลักก็ได้ (ดู[ส่วนขยายของภาษา](language.md#โมดูลไลบรารีมาตรฐาน-importname))
ชื่อ procedure มี suffix บอกชนิดติดมาด้วย (`Http_Get&`, `Regex_Replace$`)
จะเรียกแบบมีหรือไม่มี suffix ก็ได้ในที่ที่ QBasic ยอม language server จะแสดง
บรรทัดประกาศและชื่อโมดูลของ procedure ในไลบรารีเมื่อเอาเมาส์ไปชี้

สถานะ: ทุกโมดูลในหน้านี้ **ใช้ได้แล้ว** กับ back end ค่าเริ่มต้น (C++)
เมื่อใช้ `--ir` โมดูล `ext.*` ยังใช้ไม่ได้ (runner ของ QIR ยังไม่ export
ฟังก์ชัน native ของมัน) และใน[รุ่นเว็บ](web.md) `ext.http` จะแจ้งว่า
"not available"

## ภาพรวม

| โมดูล | ทำอะไร |
| --- | --- |
| `json` | JSON ตาม RFC 8259: parse, เข้าถึงข้อมูล, สร้าง, แปลงกลับเป็นข้อความ |
| `url` | แยกส่วน URL, query string, การ encode, การ resolve ตาม RFC 3986 |
| `ext.regex` | regular expression |
| `ext.hash` | CRC32, MD5, SHA-1/256/512, Base64, hex |
| `ext.zip` | อ่าน แตก และสร้างไฟล์ zip |
| `ext.http` | HTTP/HTTPS client (TLS เขียนด้วย Rust ล้วน) |
| `ext.utf8` | string UTF-8 แบบนับเป็นตัวอักษร, ตัวพิมพ์ใหญ่เล็ก, normalization, แปลง CP437 |
| `ext.collections` | list และ dictionary ของ string และตัวเลข (ใช้ handle) |
| `ext.datetime` | วันเวลาในรูปมิลลิวินาทีแบบ Unix: ISO 8601, จัดรูปแบบ, parse, แยกส่วน |
| `ext.core` | ตัวช่วยที่ใช้ร่วมกัน: `Ext_LastError$` |
| `collections.list`, `.dictionary`, `.queue`, `.stack`, `.set` | collection เขียนด้วย BASIC (อิง TYPE) ตัวอื่นนอกจาก `collections.list` ให้ import ผ่าน `qbnex` (ตอนนี้ import แยกเดี่ยวแล้วคอมไพล์ไม่ผ่าน) |
| `strings.text`, `strings.strbuilder` | ตัวช่วยจัดการข้อความ, string builder |
| `io.path`, `io.csv`, `io.json` | path, แถว CSV, ตัวสร้าง JSON ขนาดเล็ก |
| `math.numeric` | `Math_Min#`, `Math_Max#`, `Math_Clamp#`, `Math_Lerp#`, องศา/เรเดียน |
| `sys.env`, `sys.args`, `sys.datetime` | แพลตฟอร์ม, environment, argument, วันที่ |
| `error.result` | ค่า Result ที่มีรหัส error และบริบท |
| `oop.class`, `oop.interface` | registry ตอนรันที่ CLASS ใช้ |
| `qbnex` | ทุกอย่างข้างบน ยกเว้น `ext.*` ใน import เดียว |

โมดูล `ext.*` เรียกไลบรารี Rust `crates/qb-ext` (Rust ล้วน ไม่ใช้ OpenSSL)
เมื่อเกิด error ฟังก์ชันจะคืน 0 หรือ string ว่าง และ `Ext_LastError$`
บอกสาเหตุ

## json

<!-- docs-test: output -->
```basic
'$IMPORT:'json'
Json_Init
q$ = CHR$(34)                                 ' QBasic strings have no escapes
root% = JSON_Parse%("{" + q$ + "name" + q$ + ": " + q$ + "QBNex" + q$ + ", " + q$ + "tags" + q$ + ": [" + q$ + "basic" + q$ + ", 3]}")
IF root% = 0 THEN PRINT JSON_LastError$; " at byte"; JSON_ErrorPosition&: END
PRINT JsonGetString$(JsonObjectGet%(root%, "name"))
tags% = JsonObjectGet%(root%, "tags")
FOR i% = 0 TO JsonArrayLength%(tags%) - 1
    PRINT JSON_Stringify$(JsonArrayGet%(tags%, i%))
NEXT
o% = JsonCreateObject%
JsonObjectSet o%, "ok", JsonCreateBoolean%(-1)
PRINT JSON_Stringify$(o%)                      ' JSON_StringifyPretty$ indents
```

<!-- expected output -->
```text
QBNex
"basic"
3
{"ok":true}
```

ค่าทุกค่าเป็น handle (0 = ไม่มี) การ parse รองรับ object, array, escape ของ
string ทุกแบบ (`\uXXXX` และ surrogate pair กลายเป็น UTF-8), ตัวเลขตาม
grammar อย่างเคร่งครัด, ซ้อนได้ลึกสุด 500 ชั้น การเข้าถึง: `JsonGetType%`,
`JsonObjectGet%`, `JsonObjectKey$`, `JsonObjectCount%`, `JsonObjectHas%`,
`JsonObjectRemove`, `JsonArrayGet%` (เริ่มที่ 0), `JsonArrayLength%`,
`JsonGetString$`, `JsonGetNumber#` การสร้าง: `JsonCreate*`, `JsonArrayPush`,
`JsonObjectSet` ตัวเลขที่ parse มาเมื่อแปลงกลับจะได้ข้อความเดิมทุกตัวอักษร

## url

<!-- docs-test: output -->
```basic
'$IMPORT:'url'
DIM u AS UrlParts
UrlParse "https://user@Example.com:8080/a/b.txt?q=1&s=a+b#top", u
PRINT RTRIM$(u.hostname)
PRINT u.port
PRINT RTRIM$(u.pathname)
PRINT UrlGetQueryParam$(u, "s")          ' "a b"
UrlSetQueryParam u, "q", "two words"
PRINT UrlFormat$(u)
PRINT UrlEncodeComponent$("a b/c")
PRINT UrlDecode$("a%20b")
DIM r AS UrlParts
UrlResolve "http://a/b/c/d;p?q", "../g", r: PRINT UrlFormat$(r)   ' http://a/b/g
```

<!-- expected output -->
```text
example.com
 8080
/a/b.txt
a b
https://user@example.com:8080/a/b.txt?q=two%20words&s=a+b#top
a%20b%2Fc
a b
http://a/b/g
```

มี `UrlNormalizePath$`, `UrlBasename$`, `UrlExtname$`, `UrlDirname$`,
`UrlBuildQueryString$`, `UrlPathToFile$`, `UrlFileToPath$` ด้วย

## ext.regex

<!-- docs-test: output -->
<!-- docs-test: no-ir ext.* modules are not available with --ir -->
```basic
'$IMPORT:'ext.regex'
PRINT Regex_IsMatch&("^\d{3}-\d{4}$", "555-1234")           ' -1
p& = Regex_Find&("(\w+)@(\w+)\.com", "mail bob@example.com", 1)
PRINT p&
PRINT Regex_Match$
PRINT Regex_Group$(1)
PRINT Regex_GroupCount&
PRINT Regex_Replace$("(\d+)", "a1b22", "<$1>")               ' a<1>b<22>
n& = Regex_Split&("\s*,\s*", "a , b,c")
FOR i& = 0 TO n& - 1: PRINT Regex_Part$(i&): NEXT
```

<!-- expected output -->
```text
-1
 6
bob@example.com
bob
 3
a<1>b<22>
a
b
c
```

ใช้ syntax ของ crate `regex` ใน Rust (ไม่มี backreference และ look-around
ใช้ `(?i)` เพื่อไม่สนตัวพิมพ์ใหญ่เล็ก) ตำแหน่งนับเป็นไบต์เริ่มที่ 1

## ext.hash

<!-- docs-test: output -->
<!-- docs-test: no-ir ext.* modules are not available with --ir -->
```basic
'$IMPORT:'ext.hash'
PRINT Hash_SHA256$("abc")
PRINT Hash_MD5$("abc")
PRINT Hash_SHA1$("abc")
PRINT Hash_CRC32$("123456789")
b$ = Base64_Encode$("bytes")
PRINT b$
PRINT Base64_Decode$(b$)
PRINT Hex_Encode$("AZ")
PRINT Hex_Decode$("41 42")
```

<!-- expected output -->
```text
ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
900150983cd24fb0d6963f7d28e17f72
a9993e364706816aba3e25717850c26c9cd0d89d
cbf43926
Ynl0ZXM=
bytes
415a
AB
```

digest เป็น hex ตัวพิมพ์เล็ก ส่วน `Hash_CRC32Value~&` คืนค่าเป็นตัวเลข

## ext.zip

<!-- docs-test: no-ir ext.* modules are not available with --ir -->
```basic
'$IMPORT:'ext.zip'
h& = Zip_Create&("out.zip")
r& = Zip_AddData&(h&, "hello.txt", "hello")
r& = Zip_AddFile&(h&, "docs/readme.md", "README.md")
r& = Zip_Close&(h&)
n& = Zip_List&("out.zip")
FOR i& = 0 TO n& - 1: PRINT Zip_EntryName$(i&), Zip_EntrySize&&(i&): NEXT
PRINT Zip_ReadEntry$("out.zip", "hello.txt")
r& = Zip_Extract&("out.zip", "outdir")     ' ไม่ยอมแตกไฟล์ที่ชื่อมี ../
```

## ext.http

<!-- docs-test: no-ir ext.* modules are not available with --ir -->
```basic
'$IMPORT:'ext.http'
st& = Http_Get&("https://example.com/", 5000)          ' status code, 0 = ล้มเหลว
IF st& = 0 THEN PRINT Ext_LastError$ ELSE PRINT st&, LEN(Http_Body$)
PRINT Http_Header$("content-type")
st& = Http_Post&(url$, "application/json", body$, 5000)
st& = Http_Request&("PUT", url$, "X-Token: abc" + CHR$(13) + CHR$(10), body$, 5000)
```

HTTPS ใช้ rustls กับ root certificate ของ Mozilla และตาม redirect ให้เอง
body เป็น string ที่เก็บไบต์ได้ทุกค่า

## ext.utf8

<!-- docs-test: no-ir ext.* modules are not available with --ir -->
```basic
'$IMPORT:'ext.utf8'
s$ = "สวัสดี"
PRINT LEN(s$), Utf8_Len&(s$)                ' จำนวนไบต์, จำนวนตัวอักษร
PRINT Utf8_Mid$(s$, 2, 3), Utf8_Left$(s$, 2), Utf8_Right$(s$, 1)
PRINT Utf8_Upper$("straße"), Utf8_Normalize$(t$, "NFC")
PRINT Utf8_CodePoint&(s$, 1), Utf8_Chr$(3647), Utf8_IsValid&(s$)
u$ = Cp437_ToUtf8$(old$): back$ = Utf8_ToCp437$(u$)
```

ไบต์ที่ไม่ใช่ UTF-8 ที่ถูกต้องนับเป็นหนึ่งตัวอักษรต่อไบต์และถูกเก็บไว้
`Cp437_ToUtf8Graphics$` แปลงรหัสควบคุม 1-31 เป็นสัญลักษณ์ของ CP437

## ext.collections

<!-- docs-test: output -->
<!-- docs-test: no-ir ext.* modules are not available with --ir -->
```basic
'$IMPORT:'ext.collections'
l& = XList_New&
XList_Add l&, "pear": XList_AddNum l&, 2.5: XList_Insert l&, 0, "fig"
PRINT XList_Count&(l&)
PRINT XList_Get$(l&, 0)
PRINT XList_GetNum#(l&, 2)
XList_Sort l&, 0, -1                         ' เรียงจากน้อยไปมาก ไม่สนตัวพิมพ์
PRINT XList_Join$(l&, ",")
d& = XDict_New&
XDict_Set d&, "name", "Ann": XDict_SetNum d&, "age", 42
PRINT XDict_Get$(d&, "name")
PRINT XDict_GetNum#(d&, "age")
PRINT XDict_Has&(d&, "x")
FOR i& = 0 TO XDict_Count&(d&) - 1: PRINT XDict_Key$(d&, i&): NEXT
XColl_Free l&: XColl_Free d&
```

<!-- expected output -->
```text
 3
fig
 2.50
2.5,fig,pear
Ann
 42.0
 0
name
age
```

list เริ่มนับที่ 0 dictionary เก็บลำดับตามที่ใส่ `XList_Type&` /
`XDict_Type&` บอกว่าเป็น string หรือตัวเลข

## ext.datetime

<!-- docs-test: no-ir ext.* modules are not available with --ir -->
```basic
'$IMPORT:'ext.datetime'
t# = DT_Now#                                  ' มิลลิวินาทีแบบ Unix (UTC)
PRINT DT_ISO$(t#), DT_ISOLocal$(t#)
PRINT DT_Format$(t#, "%A %d %B %Y %H:%M:%S")
t2# = DT_Parse#("2024-03-05T07:08:09Z", "")   ' ISO 8601 หรือระบุ pattern เอง
PRINT DT_Year&(t2#), DT_Weekday&(t2#), DT_IsoWeek&(t2#)
PRINT DT_DiffDays#(DT_AddDays#(t2#, 10), t2#)
```

`DT_Make#(y, mo, d, h, mi, s, ms)` สร้างเวลาแบบ UTC ส่วน `DT_MakeLocal#`
สร้างตามเวลาท้องถิ่น `DT_IsValid&` ตรวจว่าเป็นค่า "invalid" ที่การ parse
คืนมาหรือไม่

## โมดูลที่เขียนด้วย BASIC ล้วน

<!-- docs-test: run -->
```basic
'$IMPORT:'qbnex'
DIM names AS QBNex_List
List_Init names
List_Add names, "basic"
PRINT List_Join$(names, ", ")
List_Free names

PRINT Path_Join$("dir", "file.txt"), Path_Extension$("a/b.bas")
PRINT Env_Platform$, Args_Count&
```

โมดูล BASIC กลุ่มนี้ (`qbnex`, `sys.*`, `json`, `url`, `collections.*`, `io.*` ...) ใช้ได้ทั้งกับ back end ค่าเริ่มต้นและ `--ir`
