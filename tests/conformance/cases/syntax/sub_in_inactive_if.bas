$CONSOLE:ONLY
' A SUB in a $IF branch that is not compiled: the module code after it is still
' compiled (every line after it disappeared without a message).
$IF 0 THEN
    SUB zz
        PRINT "never"
    END SUB
$END IF
x = 5
PRINT "x ="; x
$IF 1 THEN
    plat
$ELSE
    PRINT "not compiled"
$END IF
PRINT "done"
SYSTEM
$IF 0 THEN
    SUB plat
        PRINT "wrong plat"
    END SUB
$ELSE
    SUB plat
        PRINT "plat"
    END SUB
$END IF
