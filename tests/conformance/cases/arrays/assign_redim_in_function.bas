$CONSOLE:ONLY
' a(i) = f: the element is located after f has run. f changes the size of the array;
' the value went to the old bytes (lost inside a SUB, a crash at module level).
TYPE pt
    x AS LONG
END TYPE
DIM SHARED u AS pt
REDIM SHARED a(1000) AS LONG
DIM SHARED n AS LONG
a(3) = g&
PRINT "main: a(3) ="; a(3); "UBOUND ="; UBOUND(a)
CALL w
REDIM SHARED b(10) AS LONG
b(2) = h&
PRINT "no TYPE element: b(2) ="; b(2); "UBOUND ="; UBOUND(b)
SYSTEM
FUNCTION g&
    n = n + 1000
    REDIM _PRESERVE a(1000 + n) AS LONG
    g& = 7
END FUNCTION
FUNCTION h&
    REDIM _PRESERVE b(5000) AS LONG
    h& = 9
END FUNCTION
SUB w
    a(4) = g&
    PRINT "sub: a(4) ="; a(4); "UBOUND ="; UBOUND(a)
END SUB
