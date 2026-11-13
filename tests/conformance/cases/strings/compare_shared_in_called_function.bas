$CONSOLE:ONLY
' A string comparison on a SHARED string inside a FUNCTION called from a hot
' loop (the JIT compiled the comparison against the wrong frame).
TYPE T
    x AS LONG
END TYPE
DIM SHARED s$
DIM SHARED u AS T
DIM i AS LONG, n AS LONG
s$ = "abc"
chg s$
FOR i = 1 TO 3000
    n = n + f%
NEXT
PRINT n
SYSTEM

FUNCTION f%
    IF s$ = "abc" THEN f% = 1 ELSE f% = 100
END FUNCTION

SUB chg (a$)
END SUB
