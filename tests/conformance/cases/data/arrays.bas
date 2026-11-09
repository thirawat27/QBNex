$CONSOLE:ONLY
'$DYNAMIC
DIM m(1 TO 3, 0 TO 2) AS LONG
FOR r = 1 TO 3: FOR c = 0 TO 2: m(r, c) = r * 10 + c: NEXT: NEXT
PRINT m(2, 1); m(3, 2); LBOUND(m, 2); UBOUND(m, 1)
REDIM v(5) AS INTEGER
FOR i = 0 TO 5: v(i) = i * i: NEXT
REDIM _PRESERVE v(8) AS INTEGER
PRINT v(5); v(8); UBOUND(v)
DIM s$(2)
s$(0) = "x": s$(2) = "z"
PRINT s$(0); "|"; s$(1); "|"; s$(2)
SWAP s$(0), s$(2)
PRINT s$(0); s$(2)
SYSTEM
