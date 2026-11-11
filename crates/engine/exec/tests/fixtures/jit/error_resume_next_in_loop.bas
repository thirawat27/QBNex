ON ERROR GOTO h
FOR i% = 1 TO 20
    x% = 100 \ (i% MOD 3)
    s% = s% + x%
NEXT
PRINT s%; e%
END
h:
e% = e% + 1
RESUME NEXT
