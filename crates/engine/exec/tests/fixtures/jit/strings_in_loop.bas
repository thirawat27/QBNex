FOR i% = 1 TO 60
    a$ = a$ + CHR$(65 + i% MOD 26)
NEXT
PRINT a$; LEN(a$)
