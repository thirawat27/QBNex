'$IMPORT:'qbnex'

DIM encoded$
DIM joined$
DIM rendered$
DIM version$

encoded$ = "QBNex"

joined$ = Path_Join$("src", "main.bas")
IF joined$ <> "src\main.bas" AND joined$ <> "src/main.bas" THEN
    PRINT "Path_Join failed"
    SYSTEM 1
END IF

rendered$ = Json_String$("QBNex")
IF rendered$ <> CHR$(34) + "QBNex" + CHR$(34) THEN
    PRINT "Json_String failed"
    SYSTEM 1
END IF

version$ = QBNex_StdLib_Version$
IF LEN(version$) = 0 THEN
    PRINT "QBNex_StdLib_Version failed"
    SYSTEM 1
END IF

PRINT encoded$
PRINT joined$
PRINT rendered$
PRINT version$
