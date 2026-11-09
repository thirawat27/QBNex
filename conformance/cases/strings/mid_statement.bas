$CONSOLE:ONLY
' MID$ statement edge cases (start, length, source shorter or longer).
a$ = "abcdefgh"
MID$(a$, 3) = "XY": PRINT a$
MID$(a$, 7) = "12345": PRINT a$
MID$(a$, 1, 2) = "pqrs": PRINT a$
MID$(a$, 4, 0) = "zz": PRINT a$
MID$(a$, 8, 5) = "!": PRINT a$
b$ = "hello world"
MID$(b$, 7, 5) = UCASE$(MID$(b$, 7, 5)): PRINT b$
FOR i = 1 TO LEN(b$) STEP 2: MID$(b$, i, 1) = "_": NEXT: PRINT b$
DIM f AS STRING * 10
f = "0123456789"
MID$(f, 5, 3) = "abcdef": PRINT "["; f; "]"
c$ = "same"
MID$(c$, 2) = c$: PRINT c$
SYSTEM
