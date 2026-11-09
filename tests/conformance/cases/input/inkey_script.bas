$CONSOLE
SCREEN 12
_DEST _CONSOLE
t! = TIMER
DO
    k$ = INKEY$
    IF LEN(k$) = 2 THEN PRINT "ext"; ASC(k$, 2)
    IF LEN(k$) = 1 THEN PRINT "chr"; ASC(k$)
    IF k$ = CHR$(27) THEN EXIT DO
    _LIMIT 200
LOOP UNTIL TIMER - t! > 15
PRINT "done"
SYSTEM
