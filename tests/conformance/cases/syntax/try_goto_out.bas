$CONSOLE:ONLY
' GOTO out of a TRY body ends the TRY (the handler set before it is back); GOTO to
' a label inside the body does not.
ON ERROR GOTO h
FOR i = 1 TO 3
    TRY
        IF i = 2 THEN GOTO outside
        GOTO inside
        PRINT "skipped"
        inside:
        ERROR 5
        PRINT "not reached"
    CATCH e
        PRINT "caught"; e; "i ="; i
    END TRY
NEXT
outside:
ERROR 9
PRINT "end"
SYSTEM
h:
PRINT "handler"; ERR
RESUME NEXT
