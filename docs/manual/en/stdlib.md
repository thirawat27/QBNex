# Standard library

[Manual index](../README.md) · [ภาษาไทย](../th/stdlib.md)

The standard library is BASIC source under `compiler/stdlib`, imported per
module:

```basic
'$IMPORT:'json'
'$IMPORT:'ext.regex'
```

The import line can be anywhere in the main file (see
[language](language.md#standard-library-modules-importname)). Procedure
names carry their type suffix (`Http_Get&`, `Regex_Replace$`); you may call
them with or without it where QBasic allows. The language server shows the
declaration and the module of every library procedure on hover.

Status: all modules on this page are **Available**. In the
[web build](web.md) `ext.http` reports "not available".

## Overview

| Module | What |
| --- | --- |
| `json` | RFC 8259 JSON: parse, access, build, serialize |
| `url` | URL parsing, query strings, encoding, RFC 3986 resolution |
| `ext.regex` | Regular expressions |
| `ext.hash` | CRC32, MD5, SHA-1/256/512, Base64, hex |
| `ext.zip` | Read, extract and create zip archives |
| `ext.http` | HTTP/HTTPS client (pure-Rust TLS) |
| `ext.utf8` | UTF-8 strings by character, case, normalization, CP437 conversion |
| `ext.collections` | Lists and dictionaries of strings and numbers (handles) |
| `ext.datetime` | Dates as Unix milliseconds: ISO 8601, formatting, parsing, parts |
| `ext.core` | Shared helpers: `Ext_LastError$` |
| `collections.list`, `.dictionary`, `.queue`, `.stack`, `.set` | Collections in BASIC (TYPE based); import `qbnex` for all but `collections.list` (on their own they fail to compile for now) |
| `strings.text`, `strings.strbuilder` | Text helpers, string builder |
| `io.path`, `io.csv`, `io.json` | Paths, CSV rows, small JSON builders |
| `math.numeric` | `Math_Min#`, `Math_Max#`, `Math_Clamp#`, `Math_Lerp#`, degrees/radians |
| `sys.env`, `sys.args`, `sys.datetime` | Platform, environment, arguments, dates |
| `error.result` | Result values with error codes and context |
| `oop.class`, `oop.interface` | Runtime registry used by CLASS |
| `qbnex` | Everything above except `ext.*` in one import |

The `ext.*` modules call the Rust library `crates/runtime/ext` (pure Rust, no
OpenSSL). Errors: functions return 0 / an empty string and
`Ext_LastError$` says why.

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

Values are handles (0 = none). Parsing: objects, arrays, all string escapes
(`\uXXXX` and surrogate pairs become UTF-8), strict numbers, depth limit 500.
Access: `JsonGetType%`, `JsonObjectGet%`, `JsonObjectKey$`,
`JsonObjectCount%`, `JsonObjectHas%`, `JsonObjectRemove`, `JsonArrayGet%`
(0-based), `JsonArrayLength%`, `JsonGetString$`, `JsonGetNumber#`.
Building: `JsonCreate*`, `JsonArrayPush`, `JsonObjectSet`. Parse ->
stringify gives the same text back for parsed numbers.

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

Also `UrlNormalizePath$`, `UrlBasename$`, `UrlExtname$`, `UrlDirname$`,
`UrlBuildQueryString$`, `UrlPathToFile$`, `UrlFileToPath$`.

## ext.regex

<!-- docs-test: output -->
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

Rust `regex` syntax (no backreferences or look-around; `(?i)` for case
folding). Positions are 1-based byte positions.

## ext.hash

<!-- docs-test: output -->
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

Digests are lower-case hex. `Hash_CRC32Value~&` returns the number.

## ext.zip

```basic
'$IMPORT:'ext.zip'
h& = Zip_Create&("out.zip")
r& = Zip_AddData&(h&, "hello.txt", "hello")
r& = Zip_AddFile&(h&, "docs/readme.md", "README.md")
r& = Zip_Close&(h&)
n& = Zip_List&("out.zip")
FOR i& = 0 TO n& - 1: PRINT Zip_EntryName$(i&), Zip_EntrySize&&(i&): NEXT
PRINT Zip_ReadEntry$("out.zip", "hello.txt")
r& = Zip_Extract&("out.zip", "outdir")     ' refuses ../ names
```

## ext.http

```basic
'$IMPORT:'ext.http'
st& = Http_Get&("https://example.com/", 5000)          ' status code, 0 = failed
IF st& = 0 THEN PRINT Ext_LastError$ ELSE PRINT st&, LEN(Http_Body$)
PRINT Http_Header$("content-type")
st& = Http_Post&(url$, "application/json", body$, 5000)
st& = Http_Request&("PUT", url$, "X-Token: abc" + CHR$(13) + CHR$(10), body$, 5000)
```

HTTPS uses rustls with the Mozilla root certificates; redirects are
followed. Bodies are binary-safe strings.

## ext.utf8

```basic
'$IMPORT:'ext.utf8'
s$ = "สวัสดี"
PRINT LEN(s$), Utf8_Len&(s$)                ' bytes, characters
PRINT Utf8_Mid$(s$, 2, 3), Utf8_Left$(s$, 2), Utf8_Right$(s$, 1)
PRINT Utf8_Upper$("straße"), Utf8_Normalize$(t$, "NFC")
PRINT Utf8_CodePoint&(s$, 1), Utf8_Chr$(3647), Utf8_IsValid&(s$)
u$ = Cp437_ToUtf8$(old$): back$ = Utf8_ToCp437$(u$)
```

Invalid bytes count as one character each and are kept.
`Cp437_ToUtf8Graphics$` maps control codes 1-31 to their CP437 symbols.

## ext.collections

<!-- docs-test: output -->
```basic
'$IMPORT:'ext.collections'
l& = XList_New&
XList_Add l&, "pear": XList_AddNum l&, 2.5: XList_Insert l&, 0, "fig"
PRINT XList_Count&(l&)
PRINT XList_Get$(l&, 0)
PRINT XList_GetNum#(l&, 2)
XList_Sort l&, 0, -1                         ' ascending, ignore case
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
 2.5
2.5,fig,pear
Ann
 42
 0
name
age
```

Lists are 0-based; dictionaries keep insertion order. `XList_Type&` /
`XDict_Type&` tell string from number.

## ext.datetime

```basic
'$IMPORT:'ext.datetime'
t# = DT_Now#                                  ' Unix milliseconds (UTC)
PRINT DT_ISO$(t#), DT_ISOLocal$(t#)
PRINT DT_Format$(t#, "%A %d %B %Y %H:%M:%S")
t2# = DT_Parse#("2024-03-05T07:08:09Z", "")   ' ISO 8601, or give a pattern
PRINT DT_Year&(t2#), DT_Weekday&(t2#), DT_IsoWeek&(t2#)
PRINT DT_DiffDays#(DT_AddDays#(t2#, 10), t2#)
```

`DT_Make#(y, mo, d, h, mi, s, ms)` builds a UTC time, `DT_MakeLocal#` a
local one. `DT_IsValid&` checks for the "invalid" value parsing returns.

## BASIC-only modules

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

These BASIC modules (`qbnex`, `sys.*`, `json`, `url`, `collections.*`, `io.*`, ...) work in native programs and in the web build.
