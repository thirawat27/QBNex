$CONSOLE:ONLY
' Leaving procedures with RESUME label keeps what they did to their BYREF
' arguments (a string, a number, a whole array), and the error handler sees
' the arguments' current values while the procedures are suspended; what the
' handler changes is what the procedure sees after RESUME NEXT.
REDIM a(100) AS INTEGER
a(5) = 7
s$ = "keep"
k% = 1
ON ERROR GOTO h
CALL p(s$, a(), k%)
cont:
PRINT "s$ = "; s$; ", k% ="; k%; ", UBOUND(a) ="; UBOUND(a); ", a(5) ="; a(5)
m$ = "x"
ON ERROR GOTO h2
CALL q(m$)
PRINT "m$ = "; m$
SYSTEM

h:
PRINT "handler: s$ = "; s$; ", k% ="; k%
RESUME cont

h2:
PRINT "handler 2: m$ = "; m$
m$ = m$ + "!"
RESUME NEXT

SUB p (x AS STRING, b() AS INTEGER, n AS INTEGER)
    CALL p2(x, b(), n)
    PRINT "not reached"
END SUB

SUB p2 (y AS STRING, c() AS INTEGER, j AS INTEGER)
    y = y + "+changed"
    j = j + 41
    REDIM c(200) AS INTEGER
    c(5) = 9
    ERROR 5
END SUB

SUB q (z AS STRING)
    z = z + "y"
    ERROR 5
    PRINT "q: z = "; z
    z = z + "z"
END SUB
