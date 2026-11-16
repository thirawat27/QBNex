$CONSOLE:ONLY
'$IMPORT:'json'
' JSON (RFC 8259) parser, path access, builders and serializer (the legacy json.bas was a stub).
q$ = CHR$(34)

doc$ = "{ " + q$ + "name" + q$ + ": " + q$ + "QBNex" + q$ + ", " + q$ + "version" + q$ + ": 1.25e2,"
doc$ = doc$ + q$ + "tags" + q$ + ": [" + q$ + "basic" + q$ + ", " + q$ + "rust" + q$ + ", 3, -0.5, true, false, null, [], {}],"
doc$ = doc$ + q$ + "nested" + q$ + ": {" + q$ + "a" + q$ + ": {" + q$ + "b" + q$ + ": [1, [2, [3]]]}}," + CHR$(10)
doc$ = doc$ + q$ + "esc" + q$ + ": " + q$ + "line\nquote\" + q$ + "slash\/back\\tab\t" + q$ + "}"
root% = Json_Parse%(doc$)
PRINT "root type "; Json_Type$(root%); " members"; Json_Count%(root%); " ["; LastError$; "]"
FOR i% = 0 TO Json_Count%(root%) - 1
    PRINT " key "; Json_Key$(root%, i%); " type "; Json_Type$(root%, Json_Key$(root%, i%))
NEXT
PRINT "name="; Json_Str$(root%, "name"); " version="; Json_Num#(root%, "version"); " as text "; Json_Str$(root%, "version")
tags% = Json_Get%(root%, "tags")
PRINT "tags length"; Json_Count%(tags%); Json_Count%(root%, "tags")
FOR i% = 0 TO Json_Count%(tags%) - 1
    p$ = LTRIM$(STR$(i%))
    PRINT " tag"; i%; "type "; Json_Type$(tags%, p$); " = "; Json_Stringify$(Json_Get%(tags%, p$)); " str ["; Json_Str$(tags%, p$); "] num"; Json_Num#(tags%, p$); " bool"; Json_Bool%(tags%, p$)
NEXT
PRINT "deep 3 ="; Json_Num#(root%, "nested.a.b[1][1][0]"); Json_Num#(root%, "nested.a.b.1.1.0"); " "; Json_Type$(root%, "nested.a.b[1]")
PRINT "missing key ="; Json_Get%(root%, "nope"); " out of range ="; Json_Get%(tags%, "99"); " ["; Json_Type$(root%, "name.x"); "] ["; Json_Str$(root%, "tags[x]"); "]"; Json_Has%(root%, "nested.a"); Json_Has%(root%, "nested.z")
PRINT "keys of a nested object: "; Json_Key$(root%, 0, "nested.a"); " ["; Json_Key$(root%, 0, "tags"); "]"
e$ = Json_Str$(root%, "esc")
PRINT "esc bytes:";
FOR i% = 1 TO LEN(e$): PRINT ASC(e$, i%);: NEXT: PRINT
s1$ = Json_Stringify$(root%)
PRINT s1$
r2% = Json_Parse%(s1$)
PRINT "round trip same: "; Json_Stringify$(r2%) = s1$
PRINT Json_Stringify$(Json_Get%(root%, "nested"), 2)
PRINT Json_Stringify$(Json_Get%(root%, "tags"), 4)

' \u escapes become UTF-8 (BMP and surrogate pair), and are written back raw
u% = Json_Parse%(q$ + "Aé€😀\u0001" + q$)
u$ = Json_Str$(u%)
PRINT "utf8:";
FOR i% = 1 TO LEN(u$): PRINT " "; HEX$(ASC(u$, i%));: NEXT: PRINT
PRINT "re-escaped: "; Json_Stringify$(u%)

' numbers
DATA "0","-0","12","-3.5","1E3","2.5e-3","1e400","-1e400","1e-400","123456789012345678"
FOR i% = 1 TO 10
    READ n$
    v% = Json_Parse%(n$)
    PRINT " num "; n$; " ->"; Json_Num#(v%); " text "; Json_Stringify$(v%)
NEXT

' errors with 1-based byte positions
DATA "","   ","[1,2","[1 2]","{\x22x\x22:1,}","{x:1}","01","1.","-","1e+","tru","nul","[1]x"
DATA "\x22abc","\x22\q\x22","\x22\u12g4\x22","\x22\ud800\x22","\x22a<TAB>b\x22","[,]","{}}"
FOR i% = 1 TO 20
    READ t$
    t$ = Expand$(t$)
    v% = Json_Parse%(t$)
    PRINT " bad"; i%; "result"; v%; "pos"; Json_ErrorPos&; "msg "; LastError$
NEXT
v% = Json_Parse%("[true]")
PRINT "after a good parse ["; LastError$; "]"

' building documents
o% = Json_Object
Json_SetNum o%, "n", 0.25
Json_SetNum o%, "big", 1D+300
Json_SetNum o%, "neg", -0.5
Json_SetStr o%, "s", "tab" + CHR$(9) + "nul" + CHR$(0) + q$
Json_SetBool o%, "yes", 7
Json_SetNull o%, "nothing"
a% = Json_Array
Json_PushBool a%, -1
Json_Push a%, Json_Parse%("null")
Json_PushStr a%, "x"
Json_PushNum a%, 2
Json_Push a%, a% ' a copy of itself
Json_Set o%, "list", a%
Json_Set o%, "list2", a% ' already has a parent: deep copy
Json_SetStr o%, "n", "replaced"
Json_Remove o%, "neg"
Json_Remove o%, "no such key"
PRINT Json_Stringify$(o%)
PRINT "list2 length"; Json_Count%(o%, "list2"); " last of list2 "; Json_Stringify$(Json_Get%(o%, "list2[4]")); " nothing "; Json_Type$(o%, "nothing"); " ["; Json_Str$(o%, "nothing"); "]"
PRINT "reparse: "; Json_Stringify$(Json_Parse%(Json_Stringify$(o%))) = Json_Stringify$(o%)
e% = Json_Object
PRINT Json_Stringify$(e%); Json_Stringify$(Json_Array); Json_Stringify$(e%, 2); Json_Stringify$(0)

' a large array (pool grows beyond its initial size)
big$ = "["
FOR i% = 1 TO 2000
    big$ = big$ + LTRIM$(STR$(i%))
    IF i% < 2000 THEN big$ = big$ + ","
NEXT
big$ = big$ + "]"
b% = Json_Parse%(big$)
PRINT "big length"; Json_Count%(b%); "last"; Json_Num#(b%, "1999"); "same "; Json_Stringify$(b%) = big$
PRINT "done"
SYSTEM

FUNCTION Expand$ (t$)
    r$ = t$
    DO
        p% = INSTR(r$, "\x22")
        IF p% = 0 THEN EXIT DO
        r$ = LEFT$(r$, p% - 1) + CHR$(34) + MID$(r$, p% + 4)
    LOOP
    DO
        p% = INSTR(r$, "<TAB>")
        IF p% = 0 THEN EXIT DO
        r$ = LEFT$(r$, p% - 1) + CHR$(9) + MID$(r$, p% + 5)
    LOOP
    Expand$ = r$
END FUNCTION
