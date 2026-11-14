$CONSOLE:ONLY
' A SUB redimensions the SHARED array whose element it got BYREF: the parameter
' still addresses valid memory (the old elements), the program does not crash.
TYPE P
    x AS LONG
    y AS LONG
END TYPE
REDIM SHARED a(10) AS P
DIM SHARED n AS LONG
work
PRINT n; UBOUND(a); a(5).x
SYSTEM
SUB work
    DIM i AS LONG
    FOR i = 1 TO 3000
        grow a(0).x
    NEXT
END SUB
SUB grow (v AS LONG)
    n = n + 1
    REDIM _PRESERVE a(n + 10) AS P
    v = 7
    a(5).x = n
END SUB
