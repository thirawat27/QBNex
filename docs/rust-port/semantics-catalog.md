# Semantics Catalog — พฤติกรรมที่ Rust port ต้องรักษา

> จุดประสงค์: ระบบเดิมไม่มีสเปกภาษาที่เป็นเอกสาร — สเปกจริงคือโค้ดใน `compiler/qbnex.bas` และ `internal/c/*.cpp`.
> ไฟล์นี้รวมพฤติกรรมที่ "ซ่อน" อยู่ เพื่อให้แต่ละข้อกลายเป็นกลุ่ม test ใน `tests/conformance/`.
> แต่ละข้อมี ref ไปที่โค้ดเดิม. ข้อที่ `Verify first: Yes` ต้องรันกับ binary เดิมยืนยันก่อน implement.

## 1. Lexing / Preprocessing

| # | พฤติกรรม | Ref | หมายเหตุ |
| --- | --- | --- | --- |
| L1 | อ่านไฟล์: ตัด UTF-8 BOM, ปฏิเสธ UTF-16, ตรวจ UTF-8 ไม่ถูกต้อง | sources.bas:114-161 | `tests/smoke/encoding_smoke` |
| L2 | string literal เก็บเป็น byte; มี C-style octal escape ภายใน | parsing.bas:963-1017 | ต้องรักษา byte ที่ไม่ใช่ ASCII (CP437) |
| L3 | line continuation `_` ท้ายบรรทัด | parsing.bas:1909-1939 | |
| L4 | metacommand ในคอมเมนต์: `'$STATIC`, `'$DYNAMIC`, `'$INCLUDE:'f'`, `'$IMPORT:'m'` | parsing.bas:1843-1901 | |
| L5 | `$IF/$ELSEIF/$ELSE/$END IF/$LET/$ERROR`, symbol ที่กำหนดไว้: WINDOWS, LINUX, MAC, 32BIT, 64BIT, VERSION | directives.bas:186-534; qbnex.bas:56-71 | |
| L6 | metacommand: `$CONSOLE[:ONLY]`, `$CHECKING`, `$RESIZE`, `$ASSERTS`, `$NOPREFIX`, `$DEBUG`, `$VERSIONINFO`, `$EXEICON`, `$SCREENHIDE/SHOW` | directives.bas:1-185, 557, 635 | |
| L7 | `$INCLUDE` ซ้อนได้ ≤ 100 ชั้น, resolve relative ไฟล์ที่ include ก่อน แล้ว path ตามที่เขียน | qbnex.bas:2358-2463, 10584-10640 | |
| L8 | `$IMPORT:'a.b'` → `<exe dir>/source/stdlib/a/b.bas`; `qbnex`/`all` → stdlib.bas; de-dup | imports.bas:47-90 | path ฐานจะเปลี่ยนในระบบใหม่ |
| L9 | ส่วนขยาย QBNex: `IMPORT m`, `x += 1` (`-= *= /=`), `#` comment, `FUNC`, one-line `DEF name(x)=...` | README.md:153-159, 702-713 | Verify first: Yes — ดูว่า `DEF` แบบ QB (DEF FN) รองรับหรือไม่ (subs_functions.bas:66 ระบุเป็น stub) |

## 2. Control Flow

| # | พฤติกรรม | Ref |
| --- | --- | --- |
| C1 | line number เป็น label; label ชื่อ `name:` | qbnex.bas:2697-2746 |
| C2 | `IF ... THEN 100` / `ELSE 100` = implied GOTO | parsing.bas:1028-1031 |
| C3 | GOSUB/RETURN ต่อ scope (main และใน SUB), stack 16384 entries ขยายได้; `RETURN label` | qbnex.bas:8231-8283, 12759-12766; qbx.cpp:704-707 |
| C4 | RETURN โดยไม่มี GOSUB ใน SUB → error 3 | qbnex.bas:4092 |
| C5 | `ON ERROR GOTO label`, `ON ERROR GOTO 0`, `ERR`, `ERL`, `_ERRORLINE` | qbnex.bas:8389-8391 |
| C6 | `RESUME` = รัน statement ที่ error ซ้ำ · `RESUME NEXT` = ไป statement ถัดไป · `RESUME label` | qbnex.bas:8289-8340; qbx.cpp:2100-2129 |
| C7 | error ใน SUB ใช้ handler ของ main module และ RESUME กลับเข้า SUB ได้ (เพราะ `fix_error` เรียก `QBMAIN(NULL)` ซ้อนบน stack ของ SUB) | libqb.cpp:6432-6482 — **Verify first: Yes** |
| C8 | error ที่ไม่มี handler → message box ถาม continue (GUI) แล้วจบ; บาง code เป็น critical → `exit` ทันที | libqb.cpp:6436-6528 |
| C9 | `SELECT CASE` รวม `CASE IS`, `TO`, `CASE ELSE`, `SELECT EVERYCASE` | qbnex.bas:5466-5570, 2675-2693 |
| C10 | `FOR/NEXT`: ค่า final/step ประเมินครั้งเดียว, step ลบ, ตัวนับเป็น float ได้ | qbnex.bas:5123-5185 |
| C11 | `DO/LOOP WHILE/UNTIL` pre/post-test, `WHILE/WEND`, `EXIT DO/FOR/WHILE/SELECT/SUB/FUNCTION`, `_CONTINUE` | qbnex.bas:4964-5026, 6057-6111, 6954 |
| C12 | `ON n GOTO/GOSUB` | qbnex.bas xongotogosub ~12770+ |
| C13 | `ON TIMER(n) GOSUB`, `ON KEY(n)`, `ON STRIG(n)`, `TIMER ON/OFF/STOP`, รูปแบบ SUB ของ QB64 | qbnex.bas:6142-6549; qbx.cpp:1819-2091 |
| C14 | event ถูก poll ทุก statement (`qbevent`) — latency ระดับ statement | qbnex.bas:10518-10520 |
| C15 | `END`, `SYSTEM`, `STOP`, `RUN [line]`, `CHAIN` + `COMMON [SHARED]` | libqb.cpp:21080, 21857; qbx.cpp:1196-1500 |
| C16 | **TopLevelRuntime**: บรรทัด executable ระดับบนถูกย้ายไปเป็น `SUB QBNEX_TOPLEVEL_RUNTIME0` แล้วเรียกก่อน `sub_end` | classes.bas:1-124 — **Verify first: Yes** (ดูเหมือน unconditional; อาจเปลี่ยน semantics ของ GOTO/GOSUB ไปยัง label ระดับบน) |

## 3. Types & Data

| # | พฤติกรรม | Ref |
| --- | --- | --- |
| T1 | ชนิด: `_BIT[*n]`, `_BYTE`, `INTEGER`, `LONG`, `_INTEGER64`, `_UNSIGNED` ทุกตัว, `SINGLE`, `DOUBLE`, `_FLOAT` (= C `long double`, 80-bit บน x86), `_OFFSET`, `STRING`, `STRING * n`, UDT, `_MEM` (UDT #1) | types.bas:105-181; qbnex.bas:738-766, 1294-1364 |
| T2 | sigil `% & && ! # ## $ %% ~% ` ฯลฯ, overload ตาม sigil (`a$` กับ `a%` เป็นคนละตัว) | symbols.bas:64-84 |
| T3 | `DEFINT/DEFLNG/DEFSNG/DEFDBL/DEFSTR/_DEFINE` ตามช่วงตัวอักษร | qbnex.bas:735-736, 2052-2057, 4732-4737 |
| T4 | `OPTION BASE 0/1`, `OPTION _EXPLICIT`, `OPTION _EXPLICITARRAY` | qbnex.bas:9674-9708 |
| T5 | UDT layout packed ไม่มี padding, offset คำนวณตอน compile (เช่น stride 2869, field +548 ใน compiler เอง) | main.txt:9029 (snapshot) |
| T6 | array: หลายมิติ, bounds `a(lo TO hi)`, `$STATIC`/`$DYNAMIC`, `REDIM`, `REDIM _PRESERVE`, `ERASE`, `LBOUND/UBOUND(a, dim)`, bounds check → error 9 | variables.bas:159-216, 474-489; libqb.cpp:16964-16978; qbx.cpp:969-973 |
| T7 | `SHARED`, `STATIC` (ตัวแปรและ SUB ... STATIC), `COMMON SHARED`, `DIM SHARED` | qbnex.bas (หลายจุด) |
| T8 | parameter ส่งแบบ by-reference เป็นค่าตั้งต้น; ค่าคงที่/expression ส่งผ่าน temp; `BYVAL` | main.txt:1898 (`&(pass26=0)`) |
| T9 | CONST + consteval (precedence ของตัวเอง) | consteval.bas:2-148 |
| T10 | conversion overflow: CINT/CLNG/CSNG → error 6 | qbx.cpp:820-899 |
| T11 | การปัดเศษแบบ QB (banker's rounding ผ่าน x87 `fistp`) สำหรับ CINT/CLNG/การกำหนดค่าลง integer | libqb.cpp:149-210 |

## 4. Strings & Formatting

| # | พฤติกรรม | Ref |
| --- | --- | --- |
| S1 | string เป็น byte, ยาวได้ถึง 2^31; fixed-length pad ด้วย space | common.h:162-174 |
| S2 | `STR$` ใส่ space นำหน้าบวก; จำนวนหลักของ SINGLE/DOUBLE แบบ QB; รูปแบบ `E`/`D` exponent | libqb.cpp:7775, 8027 |
| S3 | `VAL` รับ `&H`, `&O`, `&B`, หยุดที่ตัวอักษรที่ไม่ใช่ตัวเลข, ข้าม space | libqb.cpp:13634 |
| S4 | `PRINT` zones (14 คอลัมน์), `;` `,`, `TAB`, `SPC`, ขึ้นบรรทัดเมื่อเกินความกว้าง | libqb.cpp:11821 |
| S5 | `PRINT USING` ทุก format (`#`, `.`, `,`, `+`, `-`, `$$`, `**`, `^^^^`, `!`, `&`, `\ \`) | libqb.cpp — Verify first: Yes (หาตำแหน่ง) |
| S6 | `MKI$/MKL$/MKS$/MKD$/CVI/...`, `MKSMBF$/CVSMBF` (MBF) | msbin.cpp |
| S7 | `INPUT`, `LINE INPUT`, `INPUT$`, `WRITE` (quote/comma rules), `INKEY$` (2-byte extended codes) | libqb.cpp:7726 |
| S8 | CP437 ↔ Unicode สำหรับ console/หน้าจอ | libqb.cpp:~830-852, 2872 |

## 5. Numeric

| # | พฤติกรรม | Ref |
| --- | --- | --- |
| N1 | `RND` / `RANDOMIZE` — ลำดับต้องตรง QB (LCG) เพื่อให้โปรแกรมที่ seed แล้วได้ผลเดิม | libqb.cpp:13980, 14015 |
| N2 | `TIMER` (วินาทีตั้งแต่เที่ยงคืน, ความละเอียด), `_DELAY`, `_LIMIT` | libqb.cpp:14030-14103 |
| N3 | integer division `\`, `MOD` (เครื่องหมายตามตัวตั้ง), `^` | — Verify first: Yes |
| N4 | division by zero / overflow → error 11/6 (เดิมผ่าน SIGFPE) | qbx.cpp:2136-2167 |

## 6. Files & System

| # | พฤติกรรม | Ref |
| --- | --- | --- |
| F1 | `OPEN` ทั้งสองไวยากรณ์ (`OPEN "f" FOR mode AS #n LEN=`, `OPEN mode$, #n, f$`), mode INPUT/OUTPUT/APPEND/RANDOM/BINARY | libqb.cpp:14205; 24149-24760 |
| F2 | `GET/PUT` (record และ byte position), `FIELD`, `LSET/RSET`, `SEEK`, `LOC`, `LOF`, `EOF`, `FREEFILE` | libqb.cpp:15656-15913, 17424-17502, 19356 |
| F3 | `OPEN "COMn:..."`, `SCRN:` | libqb.cpp:2554-2598 |
| F4 | `SHELL` (รอ/ไม่รอ, `_HIDE`, `_DONTWAIT`), `ENVIRON$`, `COMMAND$`, `_CWD$`, `KILL/NAME/MKDIR/RMDIR/CHDIR/FILES` | libqb.cpp:17768-19230, 22563-22660 |
| F5 | `DATA/READ/RESTORE label` (DATA เก็บเป็น byte blob `data.bin`) | qbnex.bas:11048-11063 |
| F6 | `PEEK/POKE/DEF SEG/VARPTR/VARSEG/SADD/BLOAD/BSAVE/CALL INTERRUPT` บน emulated conventional memory `cmem[1114099]` และ BIOS tick ที่ `0x46c` | qbx.cpp:695, 1097-1113; libqb.cpp:6707-6738, 15550 — ดู D4 |
| F7 | `_MEM*` family + lock validation (error 300/308/309) | libqb.cpp:27412-27720 |
| F8 | TCP: `_OPENHOST/_OPENCLIENT/_OPENCONNECTION`, `GET/PUT` บน handle, `_CONNECTED`, `_CONNECTIONADDRESS` | libqb.cpp:22680-23365 |
| F9 | `DECLARE LIBRARY` / `DECLARE DYNAMIC LIBRARY` (เรียก C/.dll/.so ของผู้ใช้) | qbnex.bas:2065, 4101; linker.bas:248-300 — ดู D5 |

## 7. Graphics / Input / Audio

| # | พฤติกรรม | Ref |
| --- | --- | --- |
| G1 | `SCREEN` 0,1,2,7,8,9,10,11,12,13 + `_NEWIMAGE` 32-bit, pages (`SCREEN m,,apage,vpage`, `PCOPY`) | libqb.cpp:8595, 3646-3667 |
| G2 | `PSET/PRESET/LINE (B/BF/style)/CIRCLE (arc, aspect)/PAINT (tile)/DRAW/POINT/VIEW/WINDOW/PMAP` | libqb.cpp:10018-12313, 22220 |
| G3 | palette: `PALETTE`, `PALETTE USING`, `COLOR`, `_RGB/_RGBA/_RGB32`, `_SETALPHA`, `_PALETTECOLOR`, default palettes | libqb.cpp:8307, 8402, 20044, 20248, 28712-28745 |
| G4 | `_PUTIMAGE`, `_COPYIMAGE`, `_FREEIMAGE`, `_LOADIMAGE`, `_SOURCE/_DEST`, `_DISPLAY/_AUTODISPLAY`, `_MAPTRIANGLE` | libqb.cpp:4240, 19762-19870, 26756 |
| G5 | fonts: 8x8/8x14/8x16 bitmap, `_LOADFONT/_FONT/_FREEFONT/_PRINTSTRING/_PRINTWIDTH` | libqb.cpp:5393, 20601-20858 |
| G6 | หน้าต่าง: `_TITLE`, `_ICON`, `_FULLSCREEN`, `$RESIZE`, `_RESIZE`, `_SCREENMOVE`, `_SCREENHIDE` | libqb.cpp:21899, 23823, 28109 |
| I1 | keyboard: `INKEY$`, `_KEYHIT` (scancode/unicode), `_KEYDOWN`, `ON KEY` | libqb.cpp:2229-2288, 26031-26050 |
| I2 | mouse: `_MOUSEINPUT/_MOUSEX/_MOUSEBUTTON/_MOUSEWHEEL`, `INT 33h` emulation | libqb.cpp:5431-5469, 19549-19660, 15550 |
| I3 | gamepad: `_DEVICES`, `_DEVICEINPUT`, `STICK/STRIG` | common.h:263-299 |
| A1 | `SOUND f, d`, `BEEP`, `PLAY` MML (state ข้ามคำสั่ง: octave/tempo/length) | audio.cpp:1141-1850 |
| A2 | synth ของ QBNex: `_VOICE` (4 voices), `_ADSR`, `_WAVE` (sine/square/saw/triangle/noise/LFSR/custom) | audio.cpp:787-1015, 1853-1946 |
| A3 | `_SNDOPEN/_SNDPLAY/_SNDLOOP/_SNDVOL/_SNDBAL/_SNDRAW/_SNDRAWLEN/_MEMSOUND` | audio.cpp:1956-2510 |
| A4 | `SUB _GL` เรียกจาก render thread พร้อม OpenGL context | gui.cpp:1664-1680; opengl_methods.bas — ดู D6 |

## 8. Compiler Surface (CLI/Diagnostics)

| # | พฤติกรรม | Ref |
| --- | --- | --- |
| D1 | flags: `-c -o -x -w -r/--warnings-as-errors -q -m -d -k/--compact-errors -e -s[:k=v] -p -z -h -v -i -g` (`-l` ถูกปฏิเสธ) | cli.bas:136-234; qbnex.bas:856-882 |
| D2 | config `internal/config.ini` ส่วน `[COMPILER SETTINGS]`: SaveExeWithSource, IgnoreWarnings, DebugInfo | compiler_settings.bas:1-24 |
| D3 | รูปแบบ diagnostics และ code range: 1000 general, 1100 parser, 1200 semantic, 1300 codegen, 2000 warnings | error_handler_defs.bas:25-122; error_handler.bas:1347-1514 |
| D4 | legacy frontend หยุดที่ error แรก | qbnex.bas:13282-13285; errors.bas:117-187 |
| D5 | warning "unused variable" (`-w`), promote ด้วย `-r` | qbnex.bas:13582 |
