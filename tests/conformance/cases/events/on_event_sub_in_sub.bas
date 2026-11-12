$CONSOLE:ONLY
' ON TIMER with a SUB name, set up inside a SUB
DIM SHARED n AS LONG
setup
t# = TIMER(.001)
DO: _LIMIT 200: LOOP UNTIL n >= 2 OR TIMER(.001) - t# > 3
TIMER OFF
PRINT "ticks"; n >= 2
SYSTEM

SUB setup
    ON TIMER(0.02) tick
    TIMER ON
END SUB

SUB tick
    n = n + 1
END SUB
