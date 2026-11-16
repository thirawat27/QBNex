$CONSOLE:ONLY
'$IMPORT:'utf8'
'$IMPORT:'collections'
'$IMPORT:'datetime'
' The native UTF-8 and date/time modules (UTC only: deterministic), and the
' List and Dict of strings that replace the native handle collections.
s$ = "a" + Utf8_Chr$(233) + Utf8_Chr$(8364) + Utf8_Chr$(128512) + "b"
PRINT LEN(s$); Utf8_Len&(s$); Hex_Of$(Utf8_Mid$(s$, 2, 2)); " "; Hex_Of$(Utf8_Mid$(s$, 4)); " "; Hex_Of$(Utf8_Right$(s$, 2)); " "; Hex_Of$(Utf8_Left$(s$, 2)); " "; Utf8_Asc&(s$, 4); Utf8_Asc&(s$, 9); Utf8_Asc&(s$)
PRINT Utf8_Upper$("stra" + Utf8_Chr$(223) + "e") = "STRASSE"; Utf8_Lower$("ABC" + Utf8_Chr$(201)) = "abc" + Utf8_Chr$(233)
d$ = "e" + Utf8_Chr$(769)
PRINT Utf8_Len&(d$); Utf8_Len&(Utf8_Normalize$(d$)); Utf8_Normalize$(Utf8_Chr$(233), "NFD") = d$; Utf8_Normalize$(d$, "nfc") = Utf8_Chr$(233)
PRINT Utf8_IsValid&(s$); Utf8_IsValid&("x" + CHR$(255)); Utf8_Len&("x" + CHR$(255) + CHR$(195)); "["; Utf8_Chr$(-5); "]"
u$ = Utf8_FromCp437$("Gr" + CHR$(129) + CHR$(225) + "e " + CHR$(219) + CHR$(13))
PRINT Hex_Of$(u$); " back ok "; (Utf8_ToCp437$(u$) = "Gr" + CHR$(129) + CHR$(225) + "e " + CHR$(219) + CHR$(13))
PRINT Hex_Of$(Utf8_FromCp437$(CHR$(1) + CHR$(3), -1)); " "; Hex_Of$(Utf8_FromCp437$(CHR$(1))); " "; Utf8_ToCp437$(Utf8_Chr$(8364) + "x")

' a List of strings (numbers stored as text) and a Dict
DIM l AS List
l.Add "pear": l.Add STR$(2.5): l.Add "Apple": l.Add STR$(-1)
l.Insert 1, "fig": l.Set 0, "banana": l.RemoveAt 4
PRINT l.Count; "["; l.Join$("|"); "]"; VAL(l.Get$(2)); "["; l.Get$(9); "]"; l.IndexOf("fig"); l.IndexOf(STR$(2.5)); l.IndexOf("zz")
l.Sort
PRINT "sorted ["; l.Join$(","); "]"
l.Sort -1
PRINT "desc ["; l.Join$(","); "]"
l.Reverse
PRINT "reversed ["; l.Join$(","); "]"
DIM dc AS Dict
dc.Set "name", "Ann": dc.Set "age", STR$(42): dc.Set "city", "Bangkok": dc.Set "age", STR$(43)
dc.Remove "city"
PRINT dc.Count; dc.Get$("name"); VAL(dc.Get$("age")); dc.Get$("age"); dc.Has("city"); dc.Has("name")
FOR i& = 0 TO dc.Count - 1: PRINT " key "; dc.Key$(i&); "="; dc.Value$(i&);: NEXT: PRINT
l.Free
PRINT "freed count"; l.Count; "["; l.Get$(0); "]"
l.Add "again": PRINT "reused"; l.Count; l.Get$(0)
DIM big AS List
FOR i& = 1 TO 5000: big.Add STR$(i&): NEXT
PRINT "big"; big.Count; VAL(big.Get$(4999)); VAL(big.Get$(-5000))
big.Free: dc.Free
PRINT "dict freed"; dc.Count

t# = Date_Make#(2024, 3, 5, 7, 8, 9, 10)
PRINT t#; Date_Iso$(t#)
PRINT Date_Format$(t#, "%A %d %B %Y, %H:%M:%S.%3f day %j week %V %%")
PRINT Date_Format$(t#); " "; Date_Format$(t#, "%F %T %a %b %I %p %s")
PRINT Date_Year&(t#); Date_Month&(t#); Date_Day&(t#); Date_Hour&(t#); Date_Minute&(t#); Date_Second&(t#); Date_Millisecond&(t#); Date_Weekday&(t#); Date_YearDay&(t#); Date_IsoWeek&(t#)
PRINT Date_Parse#("2024-03-05T14:08:09.010+07:00") = t#; Date_Parse#("05/03/2024 07:08", "%d/%m/%Y %H:%M"); Date_Iso$(Date_Parse#("1900-01-01"))
bad# = Date_Parse#("yesterday")
PRINT Date_IsValid&(bad#); Date_IsValid&(t#); (LEN(LastError$) > 0); Date_IsValid&(Date_Make#(2023, 2, 29))
PRINT Date_Iso$(Date_AddDays#(t#, 365)); Date_DiffDays#(Date_Make#(2024, 12, 25), Date_Make#(2024, 1, 1))
PRINT Date_Iso$(-1); " now ok "; (Date_Now# > t#); " local ok "; (LEN(Date_Iso$(t#, -1)) = 29); (Date_Offset&(t#) MOD 60 = 0)
PRINT "done"
SYSTEM

FUNCTION Hex_Of$ (s$)
    FOR i% = 1 TO LEN(s$)
        r$ = r$ + RIGHT$("0" + HEX$(ASC(s$, i%)), 2)
    NEXT
    Hex_Of$ = r$
END FUNCTION
