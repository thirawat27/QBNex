$CONSOLE:ONLY
' RETURN label out of an ON TIMER handler that interrupted a SUB: the SUB is left (as
' RESUME label leaves it), main-module code runs on the main module's variables, and the
' timer can fire again. It ran main's code on the SUB's variables, and the timer never
' fired a second time.
DIM SHARED n AS INTEGER
DIM v1 AS LONG, v2 AS LONG, v3 AS LONG
ON TIMER(0.1) GOSUB tick
TIMER ON
CALL busy
PRINT "not reached"
SYSTEM
tick:
n = n + 1
RETURN done
done:
v1 = 11: v2 = 22: v3 = 33
q$ = "main string"
PRINT "at done:"; v1; v2; v3; q$
IF n >= 3 THEN TIMER OFF: PRINT "three ticks": SYSTEM
t! = TIMER
DO: LOOP UNTIL TIMER - t! > 5
PRINT "the timer did not fire again"
SYSTEM
SUB busy
    DO
    LOOP
END SUB
