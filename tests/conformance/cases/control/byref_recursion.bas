$CONSOLE:ONLY
' Recursion with BYREF parameters: every level works on the first caller's
' variable, also when a level reaches it by its SHARED name.
DIM SHARED total AS LONG
n% = 5
CALL down(n%)
PRINT
PRINT "n% ="; n%
total = 0
CALL acc(total, 4)
PRINT "total ="; total
SYSTEM

SUB down (k AS INTEGER)
    IF k > 0 THEN
        k = k - 1
        CALL down(k)
    END IF
    PRINT k;
END SUB

SUB acc (t AS LONG, d AS INTEGER)
    IF d = 0 THEN EXIT SUB
    total = total + d
    t = t * 2
    CALL acc(t, d - 1)
END SUB
