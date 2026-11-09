$CONSOLE:ONLY
' VARPTR/SADD keep these strings in conventional memory (DBLOCK, cmem strings).
a$ = "first"
p = VARPTR(a$)
b$ = "second"
q = SADD(b$)
FOR i = 1 TO 200
    a$ = a$ + CHR$(48 + i MOD 10)
    IF i MOD 50 = 0 THEN b$ = b$ + a$
NEXT
PRINT LEN(a$); LEFT$(a$, 12); RIGHT$(a$, 5)
PRINT LEN(b$); MID$(b$, 4, 10)
a$ = LEFT$(a$, 3) + UCASE$(b$)
PRINT LEN(a$); LEFT$(a$, 15)
MID$(a$, 2, 3) = "***": PRINT LEFT$(a$, 10)
c$ = a$: a$ = "": PRINT LEN(c$); LEN(a$)
DEF SEG
IF PEEK(VARPTR(b$)) + 256 * PEEK(VARPTR(b$) + 1) = LEN(b$) THEN PRINT "descriptor length ok" ELSE PRINT "descriptor length differs"
SYSTEM
