# Standard library

[Manual index](../README.md) · [ภาษาไทย](../th/stdlib.md)

The standard library is BASIC source under `compiler/stdlib`, imported per
module by its short name:

```basic
'$IMPORT:'collections'
'$IMPORT:'regex'
```

The import line can be anywhere in the main file (see
[language](language.md#standard-library-modules-importname)). Each module is
loaded once, whatever the order, even when another module imports it too.
`'$IMPORT:'qbnex'` imports every module at once:

<!-- docs-test: output -->
```basic
'$IMPORT:'qbnex'
DIM words AS List
words.Split "pear fig apple", " "
words.Sort
PRINT words.Join(" "); " "; Str_PadLeft("7", 3, "0"); " "; Path_Ext("game.bas")
```

<!-- expected output -->
```text
apple fig pear 007 .bas
```

Procedures are named `Module_Name` (`Str_Replace`, `Regex_Test`). They are
declared with a type suffix (`Str_Replace$`), but a call does not need it, so
this page writes them without: `Str_Trim(s$)`, `names.Get(0)`, `LastError`.
The suffix is still allowed. A variable that takes a string result needs a
string type, from `DIM name AS STRING` or a `$` suffix (`name$ = names.Get(0)`).
The library types `List`, `Dict`, `Set`, `Url` and `Result` are
used with [method calls](language.md#method-calls-on-type-values):
`names.Add "Ann"` is `List_Add names, "Ann"` and `names.Count` is
`List_Count(names)`. Both forms work. The language server shows the
declaration and the module of every library procedure on hover.

Status: all modules on this page are **Available**. The native modules
(`regex`, `hash`, `zip`, `http`, `utf8`, `datetime`) call the Rust library
`crates/runtime/ext` (pure Rust, no OpenSSL), which is linked into every
native program; they are not available in the [web build](web.md). The BASIC
modules work in native programs and in the web build.

## Overview

| Module | What |
| --- | --- |
| [`core`](#core) | `LastError`: why the last library call failed (every module) |
| [`collections`](#collections) | `List`, `Dict` and `Set` of strings |
| [`strings`](#strings) | `Str_*` text helpers: contains, starts/ends with, replace, trim, pad, repeat |
| [`json`](#json) | RFC 8259 JSON: parse, read by path, build, stringify |
| [`url`](#url) | `Url` TYPE (parse, query, format), encode/decode, RFC 3986 resolution |
| [`path`](#path) | File paths: join, base name, folder, extension, exists |
| [`csv`](#csv) | One CSV row (RFC 4180) to and from a `List` |
| [`math`](#math) | `Math_Min`, `Math_Max`, `Math_Clamp`, `Math_Lerp`, `Math_Pi`, degrees/radians |
| [`sys`](#sys) | Platform, environment variables, home folder, command-line arguments |
| [`result`](#result) | `Result` TYPE: a value or an error from your own SUBs |
| [`regex`](#regex) | Regular expressions (native) |
| [`hash`](#hash) | MD5, SHA-1/256/512, CRC32, Base64, hex (native) |
| [`zip`](#zip) | Read, extract and create zip archives (native) |
| [`http`](#http) | HTTP/HTTPS client (native) |
| [`utf8`](#utf8) | UTF-8 text by character, case, normalization, CP437 (native) |
| [`datetime`](#datetime) | Dates as milliseconds since 1970: ISO 8601, formatting, parsing, parts (native) |
| `qbnex` | Every module in one import |

`ext` is the plumbing the native modules share, and `oop.class` /
`oop.interface` are the runtime used by [CLASS](language.md); programs do not
call them directly.

## core

Every module reports errors the same way: a call that fails returns 0 or an
empty string and sets `LastError` to the reason; a call that works sets it
back to "". Every module imports `core`.

<!-- docs-test: output -->
```basic
'$IMPORT:'url'
DIM u AS Url
u.Parse "http://example.com:99999/"
PRINT u.port; "["; LastError; "]"
u.Parse "http://example.com:80/"
PRINT u.port; "["; LastError; "]"
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
PRINT fruit.Count; fruit.Get(0); " "; fruit.Get(-1)
fruit.Sort
PRINT fruit.Join(", ")
PRINT fruit.IndexOf("fig"); fruit.Has("kiwi")
last$ = fruit.Pop
PRINT last$; fruit.Count
fruit.Split "a,b,c"
PRINT fruit.Join(" | ")

DIM ages AS Dict
ages.Set "Ann", "42": ages.Set "Bob", "7"
PRINT ages.Get("Ann"); " "; ages.Get("Zed", "?")
PRINT ages.Has("Bob"); ages.Has("Zed")
FOR i = 0 TO ages.Count - 1
    PRINT ages.Key(i); "="; ages.Value(i)
NEXT

DIM seen AS Set
seen.Add "x": seen.Add "y": seen.Add "x"
PRINT seen.Count; seen.Join
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

A new `List`, `Dict` or `Set` variable is ready to use (no init). Items are
strings: store numbers with `STR$` and read them with `VAL`. Indexes start at
0 and a negative index counts from the end (-1 = the last item). A copy
(`b = a`) is the same collection, as in Python. `Free` gives the memory back
early; otherwise it lasts until the program ends. Lists work as fields of
your own TYPEs, as array elements and as SUB parameters.

- **List**: `Add item$`, `Insert index, item$`, `Count`, `Get$(index)` (""
  outside the list), `Set index, item$`, `Pop$` (removes and returns the
  last item; `Pop$(0)` the first), `RemoveAt index`, `Remove item$` (the
  first equal item), `IndexOf(item$)` (-1 when missing), `Has(item$)`,
  `Join$(sep$)` (default `", "`), `Split text$, sep$` (replaces the items;
  default `","`; "" splits into characters), `Sort` (by bytes; `Sort -1`
  descending), `Reverse`, `Clear`, `Free`. A stack is `Add` + `Pop$`, a queue
  `Add` + `Pop$(0)`, a string builder `Add` + `Join$("")`.
- **Dict**: `Set key$, value$` (a new key goes at the end; keys keep their
  order), `Get$(key$, fallback$)`, `Has(key$)`, `Remove key$`, `Count`,
  `Key$(index)`, `Value$(index)`, `Clear`, `Free`.
- **Set**: `Add item$` (only when it is not there yet), `Has(item$)`,
  `Remove item$`, `Count`, `Join$(sep$)` (in the order added), `Clear`,
  `Free`.

## strings

<!-- docs-test: output -->
```basic
'$IMPORT:'strings'
t$ = Str_Trim("   hello world  ")
PRINT "["; t$; "]"
PRINT Str_Has(t$, "lo w"); Str_StartsWith(t$, "he"); Str_EndsWith(t$, "x")
PRINT Str_Replace("a-b-c", "-", "+")
PRINT Str_PadLeft("7", 3, "0"); " "; Str_PadRight("ab", 5, "."); "|"
PRINT Str_Repeat("ab", 3)
```

<!-- expected output -->
```text
[hello world]
-1 -1  0
a+b+c
007 ab...|
ababab
```

`Str_Has` ("" is in every text), `Str_StartsWith`, `Str_EndsWith`,
`Str_Replace` (every match), `Str_Trim` (spaces, tabs and line breaks off
both ends), `Str_PadLeft` / `Str_PadRight` (default fill: a space),
`Str_Repeat`. Splitting and joining are `List.Split` and `List.Join`
([collections](#collections)); text by character is [utf8](#utf8).

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
IF root = 0 THEN PRINT LastError; " at byte"; Json_ErrorPos: END
PRINT Json_Str(root, "name")
PRINT Json_Num(root, "user.age"); Json_Bool(root, "user.admin")
PRINT Json_Type(root, "tags"); Json_Count(root, "tags")
PRINT Json_Str(root, "tags[0]"); " "; Json_Str(root, "tags.1")
PRINT Json_Key(root, 2); Json_Has(root, "user.email")
o = Json_Object
Json_SetStr o, "name", "Ann"
Json_SetNum o, "age", 42
a = Json_Array
Json_PushStr a, "x": Json_PushBool a, 0
Json_Set o, "list", a
PRINT Json_Stringify(o)
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

A value is a handle (an INTEGER, 0 = none). Read with a path of member names
and 0-based indexes: `"user.tags.0"` or `"items[2].id"` (`""` is the value
itself).

- **Parse**: `Json_Parse(text$)` returns the root, or 0 with `LastError`
  and `Json_ErrorPos` (1-based byte). Strings are bytes: `\uXXXX` escapes
  (with surrogate pairs) become UTF-8. Numbers are strict, nesting is
  limited to 500 levels, and a parse that fails keeps nothing.
- **Read**: `Json_Get(v, path$)` (the handle), `Json_Str` (a string as its
  text, a number or `true`/`false` as JSON text, null or missing as ""),
  `Json_Num`, `Json_Bool`, `Json_Type` (`"object"`, `"array"`,
  `"string"`, `"number"`, `"boolean"`, `"null"`; "" when missing),
  `Json_Has`, `Json_Count`, `Json_Key(v, index[, path$])`.
- **Build**: `Json_Object`, `Json_Array`, `Json_Set` (an object or array
  value), `Json_SetStr`, `Json_SetNum`, `Json_SetBool`, `Json_SetNull`,
  `Json_Remove`, `Json_Push`, `Json_PushStr`, `Json_PushNum`,
  `Json_PushBool`. Objects keep member order; setting a key that is there
  replaces its value; adding a value that already belongs to an array or
  object adds a copy.
- **Write**: `Json_Stringify(v)`; `Json_Stringify(v, 2)` indents by 2.
  Parsed numbers are written back exactly as they were.
- **Memory**: up to 32767 values exist at a time. `Json_Free v` gives back a
  value and everything in it (first removing it from its parent);
  `Json_FreeAll` gives back every value. When the pool is full,
  `Json_Parse` returns 0 with `LastError` "out of JSON value space".

## url

<!-- docs-test: output -->
```basic
'$IMPORT:'url'
DIM u AS Url
u.Parse "https://user@Example.com:8080/a/b.txt?q=1&s=a+b#top"
PRINT u.hostname; u.port; u.pathname; " "; u.hash
PRINT u.Query("s")
u.SetQuery "q", "two words"
PRINT u.Format
PRINT Url_Encode("a b/c"); " "; Url_Decode("a%20b")
PRINT Url_Resolve("http://a/b/c/d;p?q", "../g")
PRINT Url_FromPath("C:\My Files\a.txt")
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

The fields of a `Url` are named as in JavaScript's `URL`: `href`,
`protocol` (`"https:"`), `username`, `password`, `hostname`, `port` (a LONG),
`pathname`, `search`, `hash`, `host`, `origin`; they are variable-length
strings, so a component of any length is kept whole. Methods: `Parse text$`,
`Query$(key$)` (decoded; `+` is a space), `SetQuery key$, value$` (replaces
the key or adds it at the end, and updates `search` and `href`), `Format$`.
A port that is not a number from 0 to 65535 is an error: `port` stays 0 and
`LastError` says why.

`Url_Encode` encodes everything except the RFC 3986 unreserved characters
(JavaScript `encodeURIComponent`), `Url_Decode` decodes `%XX`,
`Url_Resolve(base$, ref$)` resolves a reference (RFC 3986 section 5.2,
Python `urljoin`), and `Url_FromPath` / `Url_ToPath` convert between file
paths and `file:///` URLs.

## path

<!-- docs-test: output -->
```basic
'$IMPORT:'path'
f$ = Path_Join("docs", "manual", "stdlib.md")   ' docs\manual\stdlib.md on Windows
PRINT Path_Base(f$); " "; Path_Stem(f$); " "; Path_Ext(f$)
PRINT Path_Dir(f$) = Path_Join("docs", "manual")
PRINT f$ = "docs" + Path_Sep + "manual" + Path_Sep + "stdlib.md"
PRINT Path_Exists("no-such-file.txt")
```

<!-- expected output -->
```text
stdlib.md stdlib .md
-1
-1
 0
```

`/` and `\` are both accepted; results use the separator of the computer
(`Path_Sep`: `\` on Windows, `/` elsewhere). `Path_Join` takes 2 to 4
parts. Also `Path_Normalize` (the computer's separator, no doubled
separators) and `Path_Exists` (-1 for a file or a folder).

## csv

<!-- docs-test: output -->
```basic
'$IMPORT:'csv'
DIM row AS List
row.Add "plain": row.Add "a,b": row.Add "say " + CHR$(34) + "hi" + CHR$(34)
line$ = Csv_Row(row)
PRINT line$
Csv_Parse row, line$
PRINT row.Count; row.Get(2)
Csv_Parse row, "x;y;z", ";"
PRINT row.Join("/")
```

<!-- expected output -->
```text
plain,"a,b","say ""hi"""
 3 say "hi"
x/y/z
```

`Csv_Row(row[, sep$])` writes the items of a `List` as one line (a cell
with a separator, quote or line break is quoted), `Csv_Parse row, text$[,
sep$]` replaces the items with the fields of one line, and `Csv_Quote`
quotes one cell. Importing `csv` imports `collections`.

## math

<!-- docs-test: output -->
```basic
'$IMPORT:'math'
PRINT Math_Min(3, 8); Math_Max(3, 8); Math_Clamp(15, 0, 10)
PRINT Math_Lerp(0, 10, .25); Math_Degrees(Math_Pi); Math_Radians(180) = Math_Pi
```

<!-- expected output -->
```text
 3  8  10
 2.5  180 -1
```

`Math_Lerp(a, b, t)` is `a + (b - a) * t`; `Math_Radians` and
`Math_Degrees` convert angles. Everything else is built into the language
(`ABS`, `SQR`, `INT`, ...).

## sys

<!-- docs-test: run -->
```basic
'$IMPORT:'sys'
PRINT Sys_Platform; Sys_Is64Bit
PRINT Sys_Env("EDITOR", "notepad")
PRINT Sys_HasEnv("PATH"), Sys_Home
PRINT Sys_ArgCount; "arguments after "; Sys_Arg(0)
FOR i = 1 TO Sys_ArgCount: PRINT Sys_Arg(i): NEXT
```

`Sys_Platform` is `"windows"`, `"linux"` or `"macos"`. `Sys_Env(name$[,
fallback$])` reads an environment variable, `Sys_HasEnv` tells whether it is
set, `Sys_Home` is the user's home folder, `Sys_Is64Bit` is -1 in a 64-bit
program. `Sys_ArgCount` counts the arguments after the program name and
`Sys_Arg(index[, fallback$])` returns one (0 = the program).

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

A `Result` is returned from your own SUBs instead of raising `ERROR`. Read
its fields directly: `Failed` (-1 or 0), `Value`, `Message`, `Code`. The
methods set them: `Ok [value$]` (success; clears an earlier error),
`Fail message$[, code]` (default code 1) and `Context where$` (puts
`"where: "` in front of the message of an error).

## regex

<!-- docs-test: output -->
```basic
'$IMPORT:'regex'
PRINT Regex_Test("^\d{3}-\d{4}$", "555-1234")
m$ = Regex_Find("(\w+)@(\w+)\.com", "mail bob@example.com")
PRINT m$; Regex_Pos; Regex_Group(1); " "; Regex_Group(2)
PRINT Regex_Replace("(\d+)", "a1b22", "<$1>")
PRINT Regex_Replace("\d", "a1b22", "#", -1)
DIM parts AS List
Regex_Split "\s*,\s*", "a , b,c", parts
PRINT parts.Join("|")
Regex_FindAll "\d+", "1 22 333", parts
PRINT parts.Count; parts.Get(-1)
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

Rust `regex` syntax: `\d \w \s`, classes, `(groups)`, `(?<name>...)`,
`(?i)` for case-insensitive matching; no backreferences or look-around.
Positions are 1-based bytes, like `INSTR`.

- `Regex_Test(pattern$, text$)`: -1 when the text has a match.
- `Regex_Find(pattern$, text$[, start])`: the first match at or after byte
  `start` ("" when none); then `Regex_Pos` is its position (0 = no match)
  and `Regex_Group(n)` its groups (0 = the whole match).
- `Regex_Replace(pattern$, text$, replacement$[, firstOnly])`: every match
  (only the first with -1); `$1`, `${name}` and `$$` in the replacement.
- `Regex_Split pattern$, text$, parts` and `Regex_FindAll pattern$, text$,
  found` fill a `List`.

A bad pattern gives 0 or "" and `LastError` says why.

## hash

<!-- docs-test: output -->
```basic
'$IMPORT:'hash'
PRINT Hash_Sha256("abc")
PRINT Hash_Md5("abc")
PRINT Hash_Sha1("abc")
PRINT LEFT$(Hash_Sha512("abc"), 16)
PRINT LCASE$(HEX$(Hash_Crc32("123456789")))
b$ = Base64_Encode("bytes?>")
PRINT b$; " "; Base64_Encode("bytes?>", -1); " "; Base64_Decode(b$)
PRINT Hex_Encode("AZ"); " "; Hex_Decode("41 42")
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

Digests are lower-case hex of the bytes of a string. `Hash_Crc32` returns
the CRC32 as a number (as used by zip and PNG). `Base64_Encode(s$, -1)` uses
the URL-safe alphabet without padding; `Base64_Decode` reads either kind
and ignores line breaks (invalid text gives "" and `LastError`).
`Hex_Decode` ignores spaces.

## zip

<!-- docs-test: output -->
```basic
'$IMPORT:'zip'
z& = Zip_Create("out.zip")
Zip_Write z&, "hello.txt", "hello"
Zip_AddFolder z&, "docs/"
Zip_Write z&, "docs/note.txt", "a note"
Zip_Close z&
DIM names AS List
Zip_List "out.zip", names
PRINT names.Join(", ")
PRINT Zip_Read("out.zip", "docs/note.txt")
Zip_Extract "out.zip", "outdir"
PRINT _FILEEXISTS("outdir/docs/note.txt")
x$ = Zip_Read("out.zip", "missing.txt")
PRINT LastError
```

<!-- expected output -->
```text
hello.txt, docs/, docs/note.txt
a note
-1
missing.txt: specified file not found in archive
```

Member names use `/` between folders. `Zip_Create` returns a handle (0 on
error); `Zip_Write` adds a member from a string, `Zip_AddFile z&, member$,
file$` from a file and `Zip_AddFolder` an empty folder; `Zip_Close` writes
the archive. `Zip_List` fills a `List` with the member names, `Zip_Read`
returns one member and `Zip_Extract` writes every member under a folder (it
refuses names such as `../x`). Errors leave `LastError` set.

## http

```basic
'$IMPORT:'http'
page$ = Http_Get("https://example.com/", 5000)           ' 5 s timeout
IF Http_Status <> 200 THEN PRINT "failed:"; Http_Status; LastError ELSE PRINT LEN(page$); "bytes"
PRINT Http_Header("content-type")
reply$ = Http_Post("https://example.com/api", "{}")      ' sent as application/json
reply$ = Http_Request("PUT", "https://example.com/api", "data", "Authorization: Bearer x")
```

A call returns the body of the answer (binary-safe). `Http_Status` is the
status (200, 404, ...) or -1 when no answer came back (bad URL, refused,
DNS, TLS, timeout: see `LastError`). `Http_Header(name$)` returns one
header of the last answer (any letter case) and `Http_Headers` all of them.
`Http_Post(url$, body$[, contentType$[, timeoutMs]])` and
`Http_Request(method$, url$[, body$[, headers$[, timeoutMs]]])` (headers:
`"Name: value"` lines) complete the set. A timeout of 0 means 30 seconds for
the whole request. HTTPS uses rustls with the Mozilla root certificates;
redirects are followed.

## utf8

<!-- docs-test: output -->
```basic
'$IMPORT:'utf8'
s$ = "สวัสดี"
PRINT LEN(s$); Utf8_Len(s$)                     ' bytes, characters
w$ = "naïve café"
PRINT Utf8_Mid(w$, 3, 3); " "; Utf8_Left(w$, 5); " "; Utf8_Right(w$, 4)
PRINT Utf8_Upper("straße"); " "; Utf8_Lower("ÀB")
PRINT Utf8_Asc(s$); Utf8_Chr(3647); Utf8_IsValid(s$); Utf8_IsValid(CHR$(255))
PRINT Utf8_Normalize("e" + Utf8_Chr(769)) = Utf8_Chr(233)
PRINT Utf8_FromCp437(CHR$(201) + CHR$(205) + CHR$(187)); " "; Utf8_ToCp437("é") = CHR$(130)
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

BASIC strings are bytes; these functions treat them as UTF-8 text, by
character. Bytes that are not valid UTF-8 count as one character each and
are kept. `Utf8_Mid(s$, start[, count])` works like `MID$`;
`Utf8_Upper` / `Utf8_Lower` use the full Unicode case mapping;
`Utf8_Normalize(s$[, form$])` gives `"NFC"` (default), `"NFD"`, `"NFKC"` or
`"NFKD"`; `Utf8_Asc(s$[, index])` and `Utf8_Chr` work like `ASC` and
`CHR$` with code points. `Utf8_FromCp437` turns DOS text into UTF-8
(`Utf8_FromCp437(s$, -1)` also maps bytes 1-31 and 127 to the PC symbols)
and `Utf8_ToCp437` turns it back (`?` where CP437 has no glyph).

## datetime

<!-- docs-test: output -->
```basic
'$IMPORT:'datetime'
t# = Date_Parse("2024-03-05T07:08:09Z")
PRINT Date_Iso(t#)
PRINT Date_Format(t#, "%A %d %B %Y, %H:%M")
PRINT Date_Year(t#); Date_Month(t#); Date_Day(t#); Date_Weekday(t#); Date_IsoWeek(t#)
PRINT Date_Iso(Date_AddDays(t#, 10))
PRINT Date_DiffDays(Date_Make(2024, 12, 25), Date_Make(2024, 3, 5))
PRINT Date_Format(Date_Parse("05/03/2024", "%d/%m/%Y"), "%F")
bad# = Date_Parse("not a date")
PRINT Date_IsValid(bad#); Date_IsValid(Date_Now)
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

A point in time is a DOUBLE: milliseconds since 1970-01-01 00:00:00 UTC, like
JavaScript's `Date.now()`. Every function works in UTC; pass -1 as the last
argument (`isLocal`) for the computer's time zone, as in
`Date_Iso(t#, -1)` (`"2024-03-05T14:08:09.000+07:00"` in UTC+7).

- `Date_Now`, `Date_Make(y, month, day[, h, mi, s, ms[, isLocal]])`,
  `Date_Parse(text$[, pattern$[, isLocal]])` (ISO 8601 / RFC 3339 when the
  pattern is "").
- `Date_Iso`, `Date_Format(t#[, pattern$[, isLocal]])` (default
  `"%Y-%m-%d %H:%M:%S"`).
- Parts: `Date_Year`, `Date_Month` (1-12), `Date_Day`, `Date_Hour`,
  `Date_Minute`, `Date_Second`, `Date_Millisecond`, `Date_Weekday`
  (0 = Sunday), `Date_YearDay`, `Date_IsoWeek`.
- `Date_AddDays(t#, days)`, `Date_DiffDays(a#, b#)`, `Date_Offset(t#)`
  (seconds the local zone is ahead of UTC).
- Invalid input gives NaN (`Date_IsValid(t#)` = 0) and sets `LastError`.

Format codes: `%Y` year, `%m` month 01-12, `%d` day, `%H` hour 00-23, `%I`
01-12, `%M` minute, `%S` second, `%3f` milliseconds, `%p` AM/PM, `%A`/`%a`
weekday name, `%B`/`%b` month name, `%j` day of the year, `%V` ISO week, `%z`
`+0700`, `%:z` `+07:00`, `%s` Unix seconds, `%F` = `%Y-%m-%d`, `%T` =
`%H:%M:%S`, `%%` a percent sign.
