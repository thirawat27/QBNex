$CONSOLE:ONLY
' ERASE of a static array (DIM with constant bounds) only clears it: the array
' keeps its dimensions. ERASE of a dynamic array ($DYNAMIC / REDIM) frees it.
DIM b(3)
DIM s$(1 TO 2)
DIM m%(1, 2 TO 3)
b(2) = 5: s$(1) = "x": m%(1, 3) = 9
ERASE b, s$, m%
PRINT b(2); "["; s$(1); "]"; m%(1, 3)
b(2) = 7: PRINT b(2)
PRINT LBOUND(s$); UBOUND(s$); LBOUND(m%, 2); UBOUND(m%, 2)
REDIM d(5)
d(1) = 1
ERASE d
ON ERROR GOTO bad
d(1) = 2
PRINT "no error"
END
bad:
PRINT "error"; ERR
RESUME NEXT
