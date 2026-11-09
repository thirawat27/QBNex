$CONSOLE:ONLY
ON ERROR GOTO h
Work
PRINT "main continues"
SYSTEM
h:
PRINT "handled"; ERR
RESUME NEXT

SUB Work
    PRINT "work start"
    ERROR 7
    PRINT "work after error"
END SUB
