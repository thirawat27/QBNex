$CONSOLE:ONLY
'$IMPORT:'qbnex'
' Monolithic stdlib helpers: collections, strings, paths, JSON/CSV builders,
' dates, results and the class registry (fixes found in the R13 audit).
DIM l AS QBNex_List
List_Init l
List_Add l, "a": List_Add l, "": List_Add l, "c,d"
List_Set l, 1, "b"
PRINT "list"; List_Count&(l); "["; List_Join$(l, "|"); "] idx"; List_IndexOf&(l, "c,d"); List_Contains&(l, "zz")
List_RemoveAt l, 0
PRINT "after remove ["; List_Join$(l, "|"); "]"
DIM st AS QBNex_Stack: Stack_Init st
Stack_Push st, "1": Stack_Push st, "2"
PRINT "stack "; Stack_Pop$(st); Stack_Peek$(st); Stack_Count&(st)
DIM qu AS QBNex_Queue: Queue_Init qu
Queue_Enqueue qu, "x": Queue_Enqueue qu, "y"
PRINT "queue "; Queue_Dequeue$(qu); Queue_Peek$(qu); Queue_Count&(qu)
DIM hs AS QBNex_HashSet: HashSet_Init hs
HashSet_Add hs, "k": HashSet_Add hs, "k": HashSet_Add hs, "m"
PRINT "set "; HashSet_Count&(hs); HashSet_ToString$(hs, ",")
DIM dc AS QBNex_Dictionary: Dict_Init dc
Dict_Set dc, "a", "1": Dict_Set dc, "b", "2": Dict_Set dc, "a", "3"
PRINT "dict "; Dict_Count&(dc); Dict_Get$(dc, "a", "?"); Dict_Get$(dc, "z", "?"); Dict_KeyAt$(dc, 1)
DIM sb AS QBNex_StringBuilder: SB_Init sb
SB_Append sb, "ab": SB_Append sb, "cd"
PRINT "sb "; SB_ToString$(sb); SB_Length&(sb)

pad$ = "ab"
PRINT "pad ["; Text_PadLeft$("7", 6, pad$); "] ["; Text_PadRight$("7", 4, ""); "] pad still ["; pad$; "]"
PRINT Text_StartsWith&("hello", "he"); Text_EndsWith&("hello", "lo"); Text_Contains&("hello", "z"); "["; Text_Repeat$("ab", 3); "]"

a$ = "dir//sub/": b$ = "/f.txt"
j$ = Path_Join$(a$, b$)
sep$ = Path_Separator$
ok1% = (j$ = "dir" + sep$ + "sub" + sep$ + "f.txt")
ok2% = (a$ = "dir//sub/" AND b$ = "/f.txt")
PRINT "join ok"; ok1%; " args kept"; ok2%
ok3% = (Path_DirName$("/x") = sep$)
PRINT "file ["; Path_FileName$("a/b/c.tar.gz"); "] ext ["; Path_Extension$("a/b/c.tar.gz"); "] root dir ok"; ok3%

PRINT Json_Object2$("k", Json_String$("a" + CHR$(1) + CHR$(8) + CHR$(12) + CHR$(34)), "n", "1")
PRINT CSV_Row3$("plain", "a,b", "q" + CHR$(34) + "x")

DIM d AS QBNex_Date
Date_FromUnixMs d, -86400001#
PRINT "pre-1970 "; Date_ToISOString$(d); " weekday"; Date_GetDay&(d); " value"; Date_ValueOf#(d)
Date_FromParts d, 1900, 0, 1, 0, 0, 0, 0
PRINT "1900-01-01 value"; Date_ValueOf#(d); " weekday"; Date_GetDay&(d)
Date_FromUnixMs d, Date_ValueOf#(d)
PRINT "round trip "; Date_ToISOString$(d)
Date_FromUnixMs d, 951782400000#
PRINT "leap "; Date_ToISOString$(d); Date_GetDay&(d)

DIM r AS QBNex_Result, r2 AS QBNex_Result
Result_FailWithContext r, 7, "disk full", "saving", "file.txt"
Result_Propagate r2, r, "export", ""
PRINT Result_Describe$(r2)
Result_Ok r, "v"
msg$ = "  keep  "
PRINT Result_Expect$(r, msg$); " ["; msg$; "]"

FOR i& = 1 TO 260
    id& = QBNEX_RegisterClass&("C" + LTRIM$(STR$(i&)), 0)
NEXT
PRINT "classes"; QBNEX_ClassCount; " last id"; id&; " find C256"; QBNEX_FindClass&("C256"); " C257"; QBNEX_FindClass&("C257")
PRINT "done"
SYSTEM
