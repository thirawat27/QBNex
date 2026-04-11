' =============================================================================
' QBNex Standard Library — Comprehensive Examples — stdlib_demo.bas
' =============================================================================
'
' Demonstrates every module of the QBNex stdlib in a single runnable file.
' Compile with:  qb stdlib_demo.bas
' =============================================================================

'$INCLUDE:'stdlib/qbnex_stdlib.bas'

' ============================================================
PRINT "============================================"
PRINT " QBNex Standard Library Demonstration"
PRINT "============================================"
PRINT

' ============================================================
' 1. OOP — Class Registry
' ============================================================
PRINT "--- OOP: Class Registry ---"
DIM animalID AS LONG, dogID AS LONG
animalID = QBNEX_RegisterClass&("Animal", "")
dogID    = QBNEX_RegisterClass&("Dog",    "Animal")
QBNEX_RegisterMethod animalID, "Speak"
QBNEX_RegisterMethod animalID, "Move"
QBNEX_RegisterMethod dogID,    "Speak"
QBNEX_RegisterMethod dogID,    "Fetch"

PRINT "Is Dog an Animal? "; QBNEX_IsInstance&(dogID, "Animal")    ' -1 (TRUE)
PRINT "Is Animal a Dog? "; QBNEX_IsInstance&(animalID, "Dog")    '  0 (FALSE)
QBNEX_DumpClasses
PRINT

' ============================================================
' 2. Collections — List
' ============================================================
PRINT "--- Collections: List ---"
DIM fruits AS QBNex_List
List_Init fruits
List_Add fruits, "Mango"
List_Add fruits, "Apple"
List_Add fruits, "Banana"
List_Add fruits, "Cherry"
List_Insert fruits, 1, "Avocado"
PRINT "Count: "; List_Count&(fruits)                   ' 5
PRINT "Item 1: "; List_Get$(fruits, 1)                 ' Avocado
List_Sort fruits
PRINT "Sorted item 1: "; List_Get$(fruits, 1)          ' Apple
List_Print fruits
List_Free fruits
PRINT

' ============================================================
' 3. Collections — Dictionary
' ============================================================
PRINT "--- Collections: Dictionary ---"
DIM config AS QBNex_Dict
Dict_Init config
Dict_Set config, "host",    "localhost"
Dict_Set config, "port",    "8080"
Dict_Set config, "debug",   "true"
Dict_SetLong config, "timeout", 30
PRINT "host  = "; Dict_Get$(config, "host")
PRINT "port  = "; Dict_Get$(config, "port")
PRINT "Has 'host'? "; Dict_Has&(config, "host")
Dict_Delete config, "debug"
PRINT "Count after delete: "; Dict_Count&(config)
DIM keyList AS QBNex_List
List_Init keyList
Dict_Keys config, keyList
PRINT "Keys: ";
DIM ki AS LONG
FOR ki = 1 TO List_Count&(keyList)
    PRINT List_Get$(keyList, ki); " ";
NEXT ki
PRINT
List_Free keyList
Dict_Free config
PRINT

' ============================================================
' 4. Collections — Stack & Queue
' ============================================================
PRINT "--- Collections: Stack ---"
DIM stk AS QBNex_Stack
Stack_Init stk
Stack_Push stk, "first"
Stack_Push stk, "second"
Stack_Push stk, "third"
PRINT "Pop: "; Stack_Pop$(stk)    ' third
PRINT "Peek: "; Stack_Peek$(stk)  ' second
Stack_Free stk

PRINT "--- Collections: Queue ---"
DIM que AS QBNex_Queue
Queue_Init que
Queue_Enqueue que, "task-A"
Queue_Enqueue que, "task-B"
Queue_Enqueue que, "task-C"
PRINT "Dequeue: "; Queue_Dequeue$(que)   ' task-A
PRINT "Count: "; Queue_Count&(que)        ' 2
Queue_Free que
PRINT

' ============================================================
' 5. Collections — Set
' ============================================================
PRINT "--- Collections: Set ---"
DIM tags AS QBNex_Set
Set_Init tags
Set_Add& tags, "qbnex"
Set_Add& tags, "basic"
Set_Add& tags, "qbnex"    ' duplicate
PRINT "Count: "; Set_Count&(tags)            ' 2
PRINT "Has 'basic'? "; Set_Has&(tags, "basic")   ' -1
Set_Remove tags, "basic"
PRINT "Count after remove: "; Set_Count&(tags)   ' 1
Set_Free tags
PRINT

' ============================================================
' 6. Strings — StringBuilder
' ============================================================
PRINT "--- Strings: StringBuilder ---"
DIM sb AS QBNex_StringBuilder
SB_Init sb
SB_Append sb, "QBNex v"
SB_AppendLong sb, 1
SB_Append sb, "."
SB_AppendLong sb, 0
SB_AppendLine sb, " is here!"
SB_Append sb, "Platform: "
SB_Append sb, Env_Platform$()
PRINT SB_ToString$(sb)
PRINT "Length: "; SB_Length&(sb)
SB_Free sb
PRINT

' ============================================================
' 7. Strings — Encoding
' ============================================================
PRINT "--- Strings: Encoding ---"
DIM encoded AS STRING, decoded AS STRING
encoded = Base64Encode$("Hello, World!")
decoded = Base64Decode$(encoded)
PRINT "Base64: "; encoded
PRINT "Decoded: "; decoded

PRINT "UrlEncode: "; UrlEncode$("hello world & more")
PRINT "UrlDecode: "; UrlDecode$("hello+world+%26+more")
PRINT "HtmlEncode: "; HtmlEncode$("<b>Bold & bright</b>")
PRINT

' ============================================================
' 8. Strings — Regex / Glob
' ============================================================
PRINT "--- Strings: Pattern Matching ---"
PRINT "*.bas matches hello.bas? "; GlobMatch&("hello.bas", "*.bas")   ' -1
PRINT "*.exe matches hello.bas? "; GlobMatch&("hello.bas", "*.exe")   '  0
PRINT "^hello.$ regex: "; RegexMatch&("hellos", "^hello.$")           ' -1
PRINT "^[0-9]+ regex on 'abc': "; RegexMatch&("abc", "^[0-9]+$")      '  0
PRINT

' ============================================================
' 9. Math — Vectors
' ============================================================
PRINT "--- Math: Vec3 ---"
DIM va AS Vec3, vb AS Vec3, vc AS Vec3
Vec3_Set va, 1.0, 2.0, 3.0
Vec3_Set vb, 4.0, 5.0, 6.0
Vec3_Add vc, va, vb
PRINT "a + b = "; Vec3_Str$(vc)
PRINT "dot(a,b) = "; Vec3_Dot(va, vb)
PRINT "len(a) = "; Vec3_Length(va)
DIM vn AS Vec3
Vec3_Normalize vn, va
PRINT "normalize(a) "; Vec3_Str$(vn); " len="; Vec3_Length(vn)
PRINT

' ============================================================
' 10. Math — Statistics
' ============================================================
PRINT "--- Math: Stats ---"
DIM nums(1 TO 6) AS DOUBLE
nums(1) = 4: nums(2) = 8: nums(3) = 15
nums(4) = 16: nums(5) = 23: nums(6) = 42
PRINT "Sum:    "; Stats_Sum#(nums(), 1, 6)
PRINT "Mean:   "; Stats_Mean#(nums(), 1, 6)
PRINT "Median: "; Stats_Median#(nums(), 1, 6)
PRINT "StdDev: "; Stats_StdDev#(nums(), 1, 6)
PRINT "Min:    "; Stats_Min#(nums(), 1, 6)
PRINT "Max:    "; Stats_Max#(nums(), 1, 6)
PRINT

' ============================================================
' 11. I/O — Path Manipulation
' ============================================================
PRINT "--- I/O: Path ---"
DIM testPath AS STRING
testPath = "C:\Users\foo\documents\readme.txt"
PRINT "Dir:      "; Path_Dir$(testPath)
PRINT "Filename: "; Path_Filename$(testPath)
PRINT "Basename: "; Path_Basename$(testPath)
PRINT "Ext:      "; Path_Extension$(testPath)
PRINT "Joined:   "; Path_Join$("C:\Users\foo", "docs\file.bas")
PRINT "ChangeExt:"; Path_ChangeExt$(testPath, ".bas")
PRINT

' ============================================================
' 12. I/O — JSON
' ============================================================
PRINT "--- I/O: JSON ---"
DIM jobj AS QBNex_JsonNode
Json_InitObject jobj
Json_ObjSetStr  jobj, "name",    "QBNex"
Json_ObjSetNum  jobj, "version", 1.0
Json_ObjSetBool jobj, "active",  -1
Json_ObjSetNull jobj, "legacy"

DIM jsonOut AS STRING
jsonOut = Json_Stringify$(jobj)
PRINT "Serialised: "; jsonOut

DIM parsed AS QBNex_JsonNode
Json_Parse parsed, jsonOut
PRINT "name = "; Json_ObjGet$(parsed, "name")
PRINT "version = "; Json_ObjGetNum#(parsed, "version")
PRINT "active = "; Json_ObjGetBool&(parsed, "active")
Json_Free jobj
Json_Free parsed

' Array JSON
DIM jarr AS QBNex_JsonNode
Json_InitArray jarr
Json_ArrAddStr  jarr, "alpha"
Json_ArrAddNum  jarr, 42.5
Json_ArrAddRaw  jarr, "true"
PRINT "Array: "; Json_Stringify$(jarr)
Json_Free jarr
PRINT

' ============================================================
' 13. I/O — CSV
' ============================================================
PRINT "--- I/O: CSV ---"
DIM cw AS QBNex_CsvWriter
CSV_WriterInit cw, "test_data.csv"
CSV_WriteRow cw, "Name,Score,City"
CSV_WriteField cw, "Alice" : CSV_WriteField cw, "95" : CSV_WriteField cw, "Bangkok"
CSV_EndRow cw
CSV_WriteField cw, "Bob"   : CSV_WriteField cw, "87" : CSV_WriteField cw, "Chiang Mai"
CSV_EndRow cw
CSV_WriterClose cw

DIM csvRow AS QBNex_List
List_Init csvRow
CSV_ReadRow "test_data.csv", 2, csvRow
PRINT "CSV Row 2 Field 1: "; List_Get$(csvRow, 1)   ' Alice
PRINT "CSV Row 2 Field 2: "; List_Get$(csvRow, 2)   ' 95
List_Free csvRow

KILL "test_data.csv"
PRINT

' ============================================================
' 14. DateTime
' ============================================================
PRINT "--- DateTime ---"
DIM now AS QBNex_DateTime, future AS QBNex_DateTime
DT_Now now
PRINT "Now: "; DT_Format$(now, "YYYY-MM-DD HH:MI:SS")
PRINT "DOW: "; QBNEX_DT_DAYNAMES(now.DOW)

DT_AddDays future, now, 100
PRINT "+100 days: "; DT_Format$(future, "YYYY-MM-DD")
PRINT "Diff back: "; DT_DiffDays&(future, now); " days"

DIM xmas AS QBNex_DateTime
DT_Parse xmas, "2026-12-25", "YYYY-MM-DD"
PRINT "Christmas: "; DT_Format$(xmas, "DDDD, DD MMMM YYYY")
PRINT

' ============================================================
' 15. Error Handling
' ============================================================
PRINT "--- Error Handling ---"
Err_Raise 404, "Resource not found: /api/data", 0
IF Err_HasError&() THEN
    PRINT "Caught: "; Err_Format$(Err_LastCode&(), Err_LastMessage$())
    Err_Clear
END IF
Err_Assert (1 = 1), "Math is broken"   ' passes silently
PRINT "Assert passed."
PRINT

' ============================================================
' 16. System — Environment & Platform
' ============================================================
PRINT "--- System: Environment ---"
PRINT "Platform: "; Env_Platform$()
PRINT "Is 64-bit: "; Env_Is64Bit&()
PRINT "PATH (first 40 chars): "; LEFT$(Env_Get$("PATH", "(not set)"), 40); "..."
PRINT

' ============================================================
' 17. OOP — Generics / Optional / Pair
' ============================================================
PRINT "--- OOP: Generics ---"
DIM scores AS QBNex_TypedList
TL_Init scores, "LONG"
TL_AddLong scores, 88
TL_AddLong scores, 42
TL_AddLong scores, 99
TL_AddLong scores, 17
TL_SortNumeric scores
TL_Print scores

DIM opt AS QBNex_Optional
Opt_SetSome opt, "found it"
PRINT "Optional value: "; Opt_Get$(opt)
Opt_SetNone opt
PRINT "Optional with default: "; Opt_GetOrDefault$(opt, "default")

DIM pair AS QBNex_Pair
Pair_Set pair, "key", "value"
PRINT "Pair: "; Pair_First$(pair); " -> "; Pair_Second$(pair)
PRINT

' ============================================================
PRINT "============================================"
PRINT " All stdlib demos complete!"
PRINT "============================================"
