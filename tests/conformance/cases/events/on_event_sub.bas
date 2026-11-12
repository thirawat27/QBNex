$CONSOLE:ONLY
' ON TIMER and ON KEY with a SUB name (QB64): the SUB runs as the event handler
DIM SHARED n AS LONG
ON TIMER(0.02) tick
TIMER ON
t# = TIMER(.001)
DO: _LIMIT 200: LOOP UNTIL n >= 2 OR TIMER(.001) - t# > 3
TIMER OFF
PRINT "ticks"; n >= 2
ON KEY(1) pressed
KEY(1) ON
KEY(1) OFF
PRINT "done"
SYSTEM

SUB tick
    n = n + 1
END SUB

SUB pressed
    PRINT "F1"
END SUB
