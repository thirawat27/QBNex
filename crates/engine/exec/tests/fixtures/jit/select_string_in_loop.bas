FOR i& = 1 TO 3000
    k$ = MID$("abc", i& MOD 3 + 1, 1)
    SELECT CASE k$
        CASE "a": a& = a& + 1
        CASE "b": b& = b& + i&
        CASE ELSE: c& = c& + 1
    END SELECT
NEXT
PRINT a&; b&; c&
