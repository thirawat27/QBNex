$CONSOLE:ONLY
'$IMPORT:'json'
' JsonFree gives a document's values back: 40000 parses no longer run out of
' the value pool; a failed parse keeps nothing.
FOR i = 1 TO 40000
    r% = JSON_Parse%("[1,2]")
    IF r% = 0 THEN PRINT "fail at"; i; JSON_LastError$: EXIT FOR
    JsonFree r%
NEXT
PRINT "parsed"; i - 1
FOR i = 1 TO 40000
    IF JSON_Parse%("[1,{" + CHR$(34) + "a" + CHR$(34) + ":2},") THEN PRINT "unexpected": EXIT FOR
NEXT
r% = JSON_Parse%("{" + CHR$(34) + "a" + CHR$(34) + ":[1,2,3]," + CHR$(34) + "b" + CHR$(34) + ":[4]," + CHR$(34) + "a" + CHR$(34) + ":5}")
PRINT JSON_Stringify$(r%)
b% = JsonObjectGet%(r%, "b")
JsonFree b%
PRINT JSON_Stringify$(r%); JsonValid%(b%)
JsonFree r%
PRINT JsonValid%(r%)
SYSTEM
