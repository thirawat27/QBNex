$CONSOLE:ONLY
'$IMPORT:'ext.utf8'
'$IMPORT:'ext.collections'
'$IMPORT:'ext.datetime'
' qb-ext UTF-8 helpers, handle collections and date/time (UTC only: deterministic).
s$ = "a" + Utf8_Chr$(233) + Utf8_Chr$(8364) + Utf8_Chr$(128512) + "b"
PRINT LEN(s$); Utf8_Len&(s$); Hex_Of$(Utf8_Mid$(s$, 2, 2)); " "; Hex_Of$(Utf8_Right$(s$, 2)); " "; Utf8_CodePoint&(s$, 4); Utf8_CodePoint&(s$, 9)
PRINT Utf8_Upper$("stra" + Utf8_Chr$(223) + "e") = "STRASSE"; Utf8_Lower$("ABC" + Utf8_Chr$(201)) = "abc" + Utf8_Chr$(233)
d$ = "e" + Utf8_Chr$(769)
PRINT Utf8_Len&(d$); Utf8_Len&(Utf8_Normalize$(d$, "NFC")); Utf8_Normalize$(Utf8_Chr$(233), "NFD") = d$
PRINT Utf8_IsValid&(s$); Utf8_IsValid&("x" + CHR$(255)); Utf8_Len&("x" + CHR$(255) + CHR$(195))
u$ = Cp437_ToUtf8$("Gr" + CHR$(129) + CHR$(225) + "e " + CHR$(219) + CHR$(13))
PRINT Hex_Of$(u$); " back ok "; (Utf8_ToCp437$(u$) = "Gr" + CHR$(129) + CHR$(225) + "e " + CHR$(219) + CHR$(13))
PRINT Hex_Of$(Cp437_ToUtf8Graphics$(CHR$(1) + CHR$(3))); " "; Utf8_ToCp437$(Utf8_Chr$(8364) + "x")

l& = XList_New&
XList_Add l&, "pear": XList_AddNum l&, 2.5: XList_Add l&, "Apple": XList_AddNum l&, -1
XList_Insert l&, 1, "fig": XList_Set l&, 0, "banana": XList_Remove l&, 4
PRINT XList_Count&(l&); "["; XList_Join$(l&, "|"); "]"; XList_Type&(l&, 2); XList_Type&(l&, 9)
PRINT XList_GetNum#(l&, 2); "["; XList_Get$(l&, 2); "]"; XList_IndexOf&(l&, "fig"); XList_IndexOfNum&(l&, 2.5); XList_IndexOf&(l&, "zz")
XList_Sort l&, 0, -1
PRINT "sorted ["; XList_Join$(l&, ","); "]"
XList_Sort l&, -1, 0
PRINT "desc ["; XList_Join$(l&, ","); "]"
dc& = XDict_New&
XDict_Set dc&, "name", "Ann": XDict_SetNum dc&, "age", 42: XDict_Set dc&, "city", "Bangkok": XDict_SetNum dc&, "age", 43
XDict_Remove dc&, "city"
PRINT XDict_Count&(dc&); XDict_Get$(dc&, "name"); XDict_GetNum#(dc&, "age"); XDict_Get$(dc&, "age"); XDict_Has&(dc&, "city"); XDict_Type&(dc&, "name")
FOR i& = 0 TO XDict_Count&(dc&) - 1: PRINT " key "; XDict_Key$(dc&, i&);: NEXT: PRINT
XColl_Free l&
PRINT "freed count"; XList_Count&(l&); " "; Ext_LastError$
big& = XList_New&
FOR i& = 1 TO 5000: XList_AddNum big&, i&: NEXT
PRINT "big"; XList_Count&(big&); XList_GetNum#(big&, 4999)
XColl_Free big&: XColl_Free dc&

t# = DT_Make#(2024, 3, 5, 7, 8, 9, 10)
PRINT t#; DT_ISO$(t#)
PRINT DT_Format$(t#, "%A %d %B %Y, %H:%M:%S.%3f day %j week %V %%")
PRINT DT_Year&(t#); DT_Month&(t#); DT_Day&(t#); DT_Hour&(t#); DT_Minute&(t#); DT_Second&(t#); DT_Millisecond&(t#); DT_Weekday&(t#); DT_YearDay&(t#); DT_IsoWeek&(t#)
PRINT DT_Parse#("2024-03-05T14:08:09.010+07:00", "") = t#; DT_Parse#("05/03/2024 07:08", "%d/%m/%Y %H:%M"); DT_ISO$(DT_Parse#("1900-01-01", ""))
bad# = DT_Parse#("yesterday", "")
PRINT DT_IsValid&(bad#); DT_IsValid&(t#); DT_IsValid&(DT_Make#(2023, 2, 29, 0, 0, 0, 0))
PRINT DT_ISO$(DT_AddDays#(t#, 365)); DT_DiffDays#(DT_Make#(2024, 12, 25, 0, 0, 0, 0), DT_Make#(2024, 1, 1, 0, 0, 0, 0))
PRINT DT_ISO$(-1); " now ok "; (DT_Now# > t#); " local ok "; (LEN(DT_ISOLocal$(t#)) = 29)
PRINT "done"
SYSTEM

FUNCTION Hex_Of$ (s$)
    FOR i% = 1 TO LEN(s$)
        r$ = r$ + RIGHT$("0" + HEX$(ASC(s$, i%)), 2)
    NEXT
    Hex_Of$ = r$
END FUNCTION
