$CONSOLE:ONLY
' _MEMGET with STRING * n, INPUT # into a _FLOAT keeps its digits, a value passed
' to a _BIT parameter is wrapped.
DIM m AS _MEM
s$ = "hello world"
m = _MEM(s$)
x$ = _MEMGET(m, m.OFFSET + 6, STRING * 4)
PRINT x$
DIM f AS _FLOAT, l AS LONG
OPEN "f.txt" FOR OUTPUT AS #1: PRINT #1, "1.2345678901234567891": CLOSE
OPEN "f.txt" FOR INPUT AS #1: INPUT #1, f: CLOSE: KILL "f.txt"
PRINT f = 1.2345678901234567891##
l = 70000: p 9: p (l)
SYSTEM
SUB p (b AS _BIT * 3)
    PRINT "p"; b
END SUB
