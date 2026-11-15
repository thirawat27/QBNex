$CONSOLE:ONLY
' A SHARED string passed by reference keeps its text under its own name while the
' procedure runs (it was empty until the return).
DIM SHARED s AS STRING
s = "hello"
CALL p(s)
PRINT "after: "; s
SYSTEM
SUB p (x AS STRING)
    PRINT "shared: "; s; ", parameter: "; x
    x = x + " world"
END SUB
