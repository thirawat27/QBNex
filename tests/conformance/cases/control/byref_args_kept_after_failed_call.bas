$CONSOLE:ONLY
' A call that fails on a later argument (subscript out of range) gives the
' string and the array of its earlier BYREF arguments back to the caller.
ON ERROR GOTO h
DIM b(5) AS INTEGER
DIM c(3) AS LONG
c(2) = 42
a$ = "hello"
f a$, b(100)
PRINT "["; a$; "]"
g c(), b(100)
PRINT c(2); UBOUND(c)
SYSTEM
h:
PRINT "error"; ERR
RESUME NEXT

SUB f (s$, x AS INTEGER)
END SUB

SUB g (q() AS LONG, x AS INTEGER)
END SUB
