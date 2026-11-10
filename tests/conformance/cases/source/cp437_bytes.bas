$CONSOLE:ONLY
' CP437 source: box drawing …Õª in a comment
b$ = "…ÕÕª"
PRINT LEN(b$)
FOR i = 1 TO LEN(b$): PRINT ASC(b$, i);: NEXT: PRINT
IF MID$(b$, 2, 1) = "Õ" THEN PRINT "match"
c$ = "cafÇ ‡·"
FOR i = 1 TO LEN(c$): PRINT ASC(c$, i);: NEXT: PRINT
SYSTEM
