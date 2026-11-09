$CONSOLE:ONLY
ON ERROR GOTO handler
ERROR 5
PRINT "after error 5"
DIM a(5)
a(10) = 1
PRINT "after bounds"
attempt = 0
ERROR 61
PRINT "after retry"
ON ERROR GOTO 0
PRINT "done"
SYSTEM

handler:
PRINT "caught"; ERR
IF ERR = 61 AND attempt = 0 THEN
    attempt = 1
    PRINT "retrying"
    RESUME
END IF
IF ERR = 61 THEN RESUME NEXT
RESUME NEXT
