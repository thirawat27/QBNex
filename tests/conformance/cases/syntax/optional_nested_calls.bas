$CONSOLE:ONLY
' OPTIONAL defaults are filled for a call inside the arguments of another call too
' (h(h(1)) was "Incorrect number of arguments"), and a bracket in a string literal
' does not end the argument list.
PRINT h(h(1))
PRINT h(h(1), h(2, 3))
PRINT hlen(g$("a)b"))
show h(1)
show h(h(2)), "x"
IF h(1) = 6 THEN show h(h(1)): PRINT "ok" ELSE PRINT "no"
SYSTEM
FUNCTION h (x, OPTIONAL y = 5)
    h = x + y
END FUNCTION
FUNCTION g$ (s$, OPTIONAL t$ = "!")
    g$ = s$ + t$
END FUNCTION
SUB show (v, OPTIONAL label$ = "value")
    PRINT label$; v
END SUB
FUNCTION hlen (s$)
    hlen = LEN(s$)
END FUNCTION
