$CONSOLE:ONLY
' A whole array passed as a() is the caller's array during the call: its
' SHARED name sees the same elements, and REDIM through either name changes
' it for both (the SHARED name was an undimensioned array during the call).
DIM SHARED g(20) AS INTEGER
REDIM SHARED d(5) AS LONG
DIM SHARED u() AS SINGLE
g(15) = 7
CALL p(g())
PRINT "after p: g(15) ="; g(15); " g(3) ="; g(3)
d(2) = 9
CALL q(d())
PRINT "after q: UBOUND(d) ="; UBOUND(d); " d(2) ="; d(2); " d(40) ="; d(40)
CALL r(u())
PRINT "after r: UBOUND(u) ="; UBOUND(u); " u(30) ="; u(30)
SYSTEM

SUB p (a() AS INTEGER)
    PRINT "in p: g(15) ="; g(15); " a(15) ="; a(15)
    a(3) = 4
    PRINT "in p: g(3) ="; g(3)
    g(15) = 8
    PRINT "in p: a(15) ="; a(15)
END SUB

SUB q (a() AS LONG)
    REDIM _PRESERVE a(50) AS LONG
    PRINT "in q: UBOUND(d) ="; UBOUND(d); " d(2) ="; d(2)
    d(40) = 1
    PRINT "in q: a(40) ="; a(40)
END SUB

SUB r (a() AS SINGLE)
    REDIM a(30) AS SINGLE
    a(30) = 2.5
    PRINT "in r: UBOUND(u) ="; UBOUND(u)
END SUB
