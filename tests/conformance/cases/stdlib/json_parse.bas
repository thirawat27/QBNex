$CONSOLE:ONLY
'$IMPORT:'json'
' JSON (RFC 8259) parser, tree access and serializer (fixes L-13).
Json_Init
q$ = CHR$(34)

doc$ = "{ " + q$ + "name" + q$ + ": " + q$ + "QBNex" + q$ + ", " + q$ + "version" + q$ + ": 1.25e2,"
doc$ = doc$ + q$ + "tags" + q$ + ": [" + q$ + "basic" + q$ + ", " + q$ + "rust" + q$ + ", 3, -0.5, true, false, null, [], {}],"
doc$ = doc$ + q$ + "nested" + q$ + ": {" + q$ + "a" + q$ + ": {" + q$ + "b" + q$ + ": [1, [2, [3]]]}}," + CHR$(10)
doc$ = doc$ + q$ + "esc" + q$ + ": " + q$ + "line\nquote\" + q$ + "slash\/back\\tab\t" + q$ + "}"
root% = JSON_Parse%(doc$)
PRINT "root type"; JsonGetType%(root%); "members"; JsonObjectCount%(root%)
FOR i% = 0 TO JsonObjectCount%(root%) - 1
    PRINT " key "; JsonObjectKey$(root%, i%); " type"; JsonGetType%(JsonArrayGet%(root%, i%))
NEXT
PRINT "name="; JsonGetString$(JsonObjectGet%(root%, "name"))
PRINT "version="; JsonGetNumber#(JsonObjectGet%(root%, "version"))
tags% = JsonObjectGet%(root%, "tags")
PRINT "tags length"; JsonArrayLength%(tags%)
FOR i% = 0 TO JsonArrayLength%(tags%) - 1
    v% = JsonArrayGet%(tags%, i%)
    PRINT " tag"; i%; "type"; JsonGetType%(v%); "="; JSON_Stringify$(v%)
NEXT
PRINT "deep 3 ="; JsonGetNumber#(JsonArrayGet%(JsonArrayGet%(JsonArrayGet%(JsonObjectGet%(JsonObjectGet%(JsonObjectGet%(root%, "nested"), "a"), "b"), 1), 1), 0))
PRINT "missing key ="; JsonObjectGet%(root%, "nope"); " out of range ="; JsonArrayGet%(tags%, 99)
e$ = JsonGetString$(JsonObjectGet%(root%, "esc"))
PRINT "esc bytes:";
FOR i% = 1 TO LEN(e$): PRINT ASC(e$, i%);: NEXT: PRINT
s1$ = JSON_Stringify$(root%)
PRINT s1$
r2% = JSON_Parse%(s1$)
PRINT "round trip same: "; JSON_Stringify$(r2%) = s1$
PRINT JSON_StringifyPretty$(JsonObjectGet%(root%, "nested"))

' \u escapes become UTF-8 (BMP and surrogate pair), and are written back raw
u% = JSON_Parse%(q$ + "A\u00e9\u20ac\ud83d\ude00\u0001" + q$)
u$ = JsonGetString$(u%)
PRINT "utf8:";
FOR i% = 1 TO LEN(u$): PRINT " "; HEX$(ASC(u$, i%));: NEXT: PRINT
PRINT "re-escaped: "; JSON_Stringify$(u%)

' numbers
DATA "0","-0","12","-3.5","1E3","2.5e-3","1e400","-1e400","1e-400","123456789012345678"
FOR i% = 1 TO 10
    READ n$
    v% = JSON_Parse%(n$)
    PRINT " num "; n$; " ->"; JsonGetNumber#(v%); " text "; JSON_Stringify$(v%)
NEXT

' errors with 1-based byte positions
DATA "","   ","[1,2","[1 2]","{\x22x\x22:1,}","{x:1}","01","1.","-","1e+","tru","nul","[1]x"
DATA "\x22abc","\x22\q\x22","\x22\u12g4\x22","\x22\ud800\x22","\x22a<TAB>b\x22","[,]","{}}"
FOR i% = 1 TO 20
    READ t$
    t$ = Expand$(t$)
    v% = JSON_Parse%(t$)
    PRINT " bad"; i%; "result"; v%; "pos"; JSON_ErrorPosition&; "msg "; JSON_LastError$
NEXT

' building documents
o% = JsonCreateObject%
JsonObjectSet o%, "n", JsonCreateNumber%(0.25)
JsonObjectSet o%, "big", JsonCreateNumber%(1D+300)
JsonObjectSet o%, "neg", JsonCreateNumber%(-0.5)
JsonObjectSet o%, "s", JsonCreateString%("tab" + CHR$(9) + "nul" + CHR$(0) + q$)
a% = JsonCreateArray%
JsonArrayPush a%, JsonCreateBoolean%(-1)
JsonArrayPush a%, JsonCreateNull%
JsonArrayPush a%, a% ' a copy of itself
JsonObjectSet o%, "list", a%
JsonObjectSet o%, "list2", a% ' already has a parent: deep copy
JsonObjectSet o%, "n", JsonCreateString%("replaced")
JsonObjectRemove o%, "neg"
PRINT JSON_Stringify$(o%)
PRINT "list2 length"; JsonArrayLength%(JsonObjectGet%(o%, "list2"))
PRINT "reparse: "; JSON_Stringify$(JSON_Parse%(JSON_Stringify$(o%))) = JSON_Stringify$(o%)

' a large array (pool grows beyond its initial 500 slots)
big$ = "["
FOR i% = 1 TO 2000
    big$ = big$ + LTRIM$(STR$(i%))
    IF i% < 2000 THEN big$ = big$ + ","
NEXT
big$ = big$ + "]"
b% = JSON_Parse%(big$)
PRINT "big length"; JsonArrayLength%(b%); "last"; JsonGetNumber#(JsonArrayGet%(b%, 1999)); "same "; JSON_Stringify$(b%) = big$
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
