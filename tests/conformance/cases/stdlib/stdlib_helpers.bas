$CONSOLE:ONLY
'$IMPORT:'qbnex'
' Standard library helpers: collections, strings, paths, JSON and CSV
' building, dates, results and the class registry (fixes found in an audit).
DIM l AS List
l.Add "a": l.Add "": l.Add "c,d"
l.Set 1, "b"
PRINT "list"; l.Count; "["; l.Join$("|"); "] idx"; l.IndexOf("c,d"); l.Has("zz"); "["; l.Join$; "]"
l.RemoveAt 0
PRINT "after remove ["; l.Join$("|"); "]"
l.Split "x;;y", ";"
PRINT "split"; l.Count; "["; l.Join$("|"); "]";
l.Split "abc", ""
PRINT l.Count; "["; l.Join$("|"); "]";
l.Split "", ","
PRINT l.Count; "["; l.Get$(0); "]"
DIM st AS List
st.Add "1": st.Add "2"
PRINT "stack "; st.Pop$; st.Get$(-1); st.Count
DIM qu AS List
qu.Add "x": qu.Add "y"
PRINT "queue "; qu.Pop$(0); qu.Get$(0); qu.Count
DIM hs AS Set
hs.Add "k": hs.Add "k": hs.Add "m"
PRINT "set "; hs.Count; hs.Join$(","); " "; hs.Join$
DIM dc AS Dict
dc.Set "a", "1": dc.Set "b", "2": dc.Set "a", "3"
PRINT "dict "; dc.Count; dc.Get$("a", "?"); dc.Get$("z", "?"); dc.Key$(1); "["; dc.Key$(5); "]"
dc.Clear
PRINT "cleared"; dc.Count; dc.Has("a")
DIM sb AS List
sb.Add "ab": sb.Add "cd"
PRINT "built "; sb.Join$(""); LEN(sb.Join$(""))

pad$ = "ab"
PRINT "pad ["; Str_PadLeft$("7", 6, pad$); "] ["; Str_PadRight$("7", 4); "] ["; Str_PadLeft$("long", 2); "] pad still ["; pad$; "]"
PRINT Str_StartsWith&("hello", "he"); Str_EndsWith&("hello", "lo"); Str_Has&("hello", "z"); Str_Has&("hello", ""); "["; Str_Repeat$("ab", 3); "]"
t$ = "  a-b-c " + CHR$(9)
PRINT "["; Str_Replace$(t$, "-", "+"); "] ["; Str_Trim$(t$); "] ["; Str_Replace$("aaa", "aa", "b"); "] ["; Str_Replace$("x", "", "y"); "] kept"; LEN(t$)

a$ = "dir//sub/": b$ = "/f.txt"
j$ = Path_Join$(a$, b$)
sep$ = Path_Sep$
ok1% = (j$ = "dir" + sep$ + "sub" + sep$ + "f.txt")
ok2% = (a$ = "dir//sub/" AND b$ = "/f.txt")
ok4% = (Path_Join$("a", "b", "c", "d.txt") = "a" + sep$ + "b" + sep$ + "c" + sep$ + "d.txt")
PRINT "join ok"; ok1%; " args kept"; ok2%; " four parts"; ok4%
ok3% = (Path_Dir$("/x") = sep$)
PRINT "file ["; Path_Base$("a/b/c.tar.gz"); "] ext ["; Path_Ext$("a/b/c.tar.gz"); "] stem ["; Path_Stem$("a\b\c.tar.gz"); "] no ext ["; Path_Ext$(".profile"); "] root dir ok"; ok3%; " ["; Path_Dir$("plain"); "]"

o% = Json_Object
Json_SetStr o%, "k", "a" + CHR$(1) + CHR$(8) + CHR$(12) + CHR$(34)
Json_SetNum o%, "n", 1
PRINT Json_Stringify$(o%)
DIM row AS List
row.Add "plain": row.Add "a,b": row.Add "q" + CHR$(34) + "x": row.Add ""
line$ = Csv_Row$(row)
PRINT line$; " "; Csv_Row$(row, ";")
Csv_Parse row, line$
PRINT row.Count; "["; row.Join$("|"); "]"
Csv_Parse row, "1;" + CHR$(34) + "x;y" + CHR$(34), ";"
PRINT row.Count; "["; row.Join$("|"); "]"

t# = -86400001#
PRINT "pre-1970 "; Date_Iso$(t#); " weekday"; Date_Weekday&(t#); " value"; t#
t# = Date_Make#(1900, 1, 1)
PRINT "1900-01-01 value"; t#; " weekday"; Date_Weekday&(t#)
PRINT "round trip "; Date_Iso$(Date_Parse#(Date_Iso$(t#)))
t# = 951782400000#
PRINT "leap "; Date_Iso$(t#); Date_Weekday&(t#)

DIM r AS Result
r.Fail "disk full", 7
r.Context "saving"
r.Context "export"
PRINT r.Failed; r.Code; r.Message
IF r.Failed THEN r.Context "starting": PRINT r.Message ELSE PRINT r.Value
r.Ok "v"
IF r.Failed THEN r.Context "starting": PRINT r.Message ELSE PRINT r.Value; r.Failed; r.Code; "["; r.Message; "]"
r.Fail "plain"
PRINT r.Code; "["; r.Value; "]"
msg$ = "  keep  "
PRINT "["; Str_Trim$(msg$); "] ["; msg$; "]"

FOR i& = 1 TO 260
    id& = QBNEX_RegisterClass&("C" + LTRIM$(STR$(i&)), 0)
NEXT
PRINT "classes"; QBNEX_ClassCount; " last id"; id&; " find C256"; QBNEX_FindClass&("C256"); " C257"; QBNEX_FindClass&("C257")
PRINT "math"; Math_Min#(2, -3); Math_Max#(2, -3); Math_Clamp#(15, 0, 10); Math_Lerp#(10, 20, 0.25); INT(Math_Degrees#(Math_Pi#) + 0.5); Math_Radians#(180) = Math_Pi#
PRINT "sys "; Sys_Platform$ <> "unknown"; Sys_Env$("QBNEX_NO_SUCH_VARIABLE", "dflt"); Sys_HasEnv&("QBNEX_NO_SUCH_VARIABLE"); Sys_ArgCount&; "["; Sys_Arg$(3, "none"); "]"; LEN(Sys_Home$) > 0
PRINT "done"
SYSTEM
