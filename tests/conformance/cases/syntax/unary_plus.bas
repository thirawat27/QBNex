$CONSOLE:ONLY
' A leading unary plus is accepted as in QBasic (QB64 rejected x = +5).
x = +5: PRINT x
PRINT +5
a = 2 * +3: PRINT a
PRINT -+2; 2 - +1; (+4); ABS(+3)
CONST c = +7: PRINT c
PRINT 1 + +2; 3 ^ +2; 2 = +2
DIM z(+3): PRINT UBOUND(z)
s$ = "a" + "b": PRINT s$
SYSTEM
