'$INCLUDE:'compiler/stdlib/qbnex.bas'

DIM names AS List

names.Add "QBNex"
IF names.Count <> 1 OR Str_Repeat$("ab", 2) <> "abab" THEN
    PRINT "qbnex include failed"
    SYSTEM 1
END IF

PRINT names.Get$(0); " include"
