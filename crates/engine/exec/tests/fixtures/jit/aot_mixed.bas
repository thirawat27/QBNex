FOR i% = 1 TO 30
    s& = s& + fact&(i% MOD 8)
    GOSUB g
NEXT
FOR i% = 1 TO 60
    a$ = a$ + CHR$(65 + i% MOD 26)
    x! = x! + i%
NEXT
PRINT s&; c&; a$; x!
END
g:
c& = c& + 1
RETURN
FUNCTION fact& (n&)
    IF n& <= 1 THEN fact& = 1 ELSE fact& = n& * fact&(n& - 1)
END FUNCTION
