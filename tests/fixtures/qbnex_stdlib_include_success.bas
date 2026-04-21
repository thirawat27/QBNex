'$INCLUDE:'source/stdlib/qbnex_stdlib.bas'

DIM version$

version$ = QBNex_StdLib_Version$
IF LEN(version$) = 0 THEN
    PRINT "QBNex_StdLib_Version failed"
    SYSTEM 1
END IF

PRINT version$
