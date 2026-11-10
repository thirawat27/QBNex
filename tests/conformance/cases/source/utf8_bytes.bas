$CONSOLE:ONLY
' UTF-8 source keeps its bytes too: ─│
u$ = "é─"
PRINT LEN(u$)
FOR i = 1 TO LEN(u$): PRINT ASC(u$, i);: NEXT: PRINT
SYSTEM
