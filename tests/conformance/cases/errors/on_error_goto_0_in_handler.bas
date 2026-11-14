$CONSOLE:ONLY
' ON ERROR GOTO 0 inside a handler passes the error on: the program stops with the
' error being handled, as in QBasic (it went on in the handler and could RESUME).
ON ERROR GOTO h
ERROR 5
PRINT "after the error"
SYSTEM
h:
PRINT "in handler, ERR ="; ERR
IF ERR = 5 THEN ON ERROR GOTO 0
PRINT "not reached"
RESUME NEXT
