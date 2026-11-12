' bounds that read another array (UBOUND(b)) must still resize the named array, not b
REDIM a(5), b(10)
FOR i = 0 TO 5: a(i) = i * 2: NEXT
REDIM _PRESERVE a(UBOUND(b))
PRINT UBOUND(a); UBOUND(b); a(5)
REDIM c(UBOUND(a) + 2)
PRINT UBOUND(c); UBOUND(a)
REDIM m(2, 3), n(4, 5)
REDIM _PRESERVE m(2, UBOUND(n, 2))
PRINT UBOUND(m, 1); UBOUND(m, 2); UBOUND(n, 1); UBOUND(n, 2)
REDIM m(UBOUND(n, 1), LBOUND(b) + 1 TO UBOUND(n, 2))
PRINT UBOUND(m, 1); LBOUND(m, 2); UBOUND(m, 2); UBOUND(n, 1)
DIM d(LBOUND(b) + 3 TO UBOUND(b))
PRINT LBOUND(d); UBOUND(d); UBOUND(b)
