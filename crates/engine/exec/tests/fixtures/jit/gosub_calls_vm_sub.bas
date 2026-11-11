FOR i& = 1 TO 200
    GOSUB g
NEXT
PRINT c&
END
g:
bump c&
RETURN
SUB bump (n&)
    s$ = "x" + STR$(n&)
    n& = n& + LEN(s$) - 2
END SUB
