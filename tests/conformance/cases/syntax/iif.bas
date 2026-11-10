$CONSOLE:ONLY
x = 7
PRINT _IIF(x > 5, "big", "small"); _IIF(x < 5, 1, 2); _IIF(x, 2, 1.5)
calls = 0
PRINT _IIF(x = 7, 10, Bump%); calls
SYSTEM

FUNCTION Bump%
    SHARED calls
    calls = calls + 1
    Bump% = 99
END FUNCTION
