$CONSOLE:ONLY
DECLARE SUB Bump (x AS INTEGER)
DIM SHARED total AS LONG
a% = 5
Bump a%
PRINT a%
Bump (a%)
PRINT a%
PRINT Fact&(10)
PRINT Twice$("ab")
Counter: Counter: Counter
PRINT total
SYSTEM

SUB Bump (x AS INTEGER)
    x = x + 1
END SUB

FUNCTION Fact& (n AS LONG)
    IF n <= 1 THEN Fact& = 1 ELSE Fact& = n * Fact&(n - 1)
END FUNCTION

FUNCTION Twice$ (s AS STRING)
    Twice$ = s + s
END FUNCTION

SUB Counter STATIC
    c = c + 1
    total = total + c
END SUB
