$CONSOLE:ONLY
' Multi-line DEF FN: module variables (s, i, last, a(), total) are shared, STATIC ones stay local
DIM a(3)
a(1) = 5: total = 100
DEF FnSum (n)
    STATIC calls
    calls = calls + 1
    s = 0
    FOR i = 1 TO n
        s = s + i
    NEXT
    IF n > 10 THEN FnSum = -1: EXIT DEF
    FnSum = s + a(1) + total
    last = calls
END DEF
PRINT FnSum(3)
PRINT FnSum(20)
PRINT last; i
SYSTEM
