$CONSOLE:ONLY
' A timer freed inside its own handler: its number, handed out again, fires, and
' an interval that is not above 0 is an illegal function call.
t1 = _FREETIMER
ON TIMER(t1, .1) GOSUB once
TIMER(t1) ON
t0! = TIMER
DO: _LIMIT 200: LOOP UNTIL fired1 OR TIMER - t0! > 2
PRINT "first fired:"; fired1
t2 = _FREETIMER
PRINT "same index reused:"; t2 = t1
ON TIMER(t2, .1) GOSUB again
TIMER(t2) ON
t0! = TIMER
DO: _LIMIT 200: LOOP UNTIL fired2 OR TIMER - t0! > 2
PRINT "second fired:"; fired2
ON ERROR GOTO h
z = 0
ON TIMER(z) GOSUB again
SYSTEM
once:
fired1 = -1
TIMER(t1) FREE
RETURN
again:
fired2 = -1
RETURN
h:
PRINT "error"; ERR
RESUME NEXT
