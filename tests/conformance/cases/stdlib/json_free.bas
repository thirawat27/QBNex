$CONSOLE:ONLY
'$IMPORT:'json'
' Json_Free gives a document's values back: 40000 parses no longer run out of
' the value pool; a failed parse keeps nothing.
FOR i = 1 TO 40000
    r% = Json_Parse%("[1,2]")
    IF r% = 0 THEN PRINT "fail at"; i; LastError$: EXIT FOR
    Json_Free r%
NEXT
PRINT "parsed"; i - 1
FOR i = 1 TO 40000
    IF Json_Parse%("[1,{" + CHR$(34) + "a" + CHR$(34) + ":2},") THEN PRINT "unexpected": EXIT FOR
NEXT
r% = Json_Parse%("{" + CHR$(34) + "a" + CHR$(34) + ":[1,2,3]," + CHR$(34) + "b" + CHR$(34) + ":[4]," + CHR$(34) + "a" + CHR$(34) + ":5}")
PRINT Json_Stringify$(r%)
b% = Json_Get%(r%, "b")
Json_Free b%
PRINT Json_Stringify$(r%); " ["; Json_Type$(b%); "]"; Json_Has%(r%, "b")
Json_Free r%
PRINT "["; Json_Type$(r%); "] "; Json_Stringify$(r%)
FOR i = 1 TO 40000
    o% = Json_Object: Json_SetStr o%, "k", "v"
NEXT
Json_FreeAll
PRINT "after FreeAll"; Json_Parse%("[]") > 0
SYSTEM
