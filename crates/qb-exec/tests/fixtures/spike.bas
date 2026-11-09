$CONSOLE:ONLY
' R11 spike fixture: PRINT, numeric variables, IF, FOR, GOSUB/RETURN, SUB, strings
DIM SHARED total AS LONG
DIM n AS INTEGER
x = 1.5
y# = 1 / 3
PRINT "x="; x; "y="; y#
FOR i% = 1 TO 5 STEP 2
    IF i% = 3 THEN PRINT "three" ELSE PRINT i%;
NEXT i%
PRINT
FOR k = 3 TO 1 STEP -1
    n = n + k * 2
NEXT
IF n > 20 THEN
    PRINT "huge"; n
ELSEIF n > 10 THEN
    PRINT "big"; n
ELSE
    PRINT "small"
END IF
GOSUB Report
Greet "Bob"
Greet "Al" + "ice"
a$ = "Hello, World"
PRINT LEFT$(a$, 5) + "!"; LEN(a$); UCASE$(MID$(a$, 8, 5))
x = x * 2 + 7 \ 2 - 2 ^ 3
PRINT x; 7 MOD 3; -x; (x < 0) AND (n = 12)
GOSUB Report
SYSTEM

Report:
PRINT "report"; total
RETURN

SUB Greet (who$)
    total = total + 1
    PRINT "hi "; who$; total
END SUB
