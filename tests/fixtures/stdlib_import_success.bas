'$IMPORT:'qbnex'

DIM encoded$
DIM joined$
DIM rendered$
DIM parts AS List

encoded$ = "QBNex"

joined$ = Path_Join$("src", "main.bas")
IF joined$ <> "src\main.bas" AND joined$ <> "src/main.bas" THEN
    PRINT "Path_Join failed"
    SYSTEM 1
END IF

o% = Json_Object
Json_SetStr o%, "name", "QBNex"
rendered$ = Json_Stringify$(o%)
IF rendered$ <> "{" + CHR$(34) + "name" + CHR$(34) + ":" + CHR$(34) + "QBNex" + CHR$(34) + "}" THEN
    PRINT "Json_Stringify failed"
    SYSTEM 1
END IF

parts.Split "a,b,c"
IF parts.Count <> 3 OR parts.Join$("+") <> "a+b+c" THEN
    PRINT "List_Split failed"
    SYSTEM 1
END IF

IF LastError$ <> "" THEN
    PRINT "LastError failed: "; LastError$
    SYSTEM 1
END IF

PRINT encoded$
PRINT joined$
PRINT rendered$
PRINT parts.Join$("+")
