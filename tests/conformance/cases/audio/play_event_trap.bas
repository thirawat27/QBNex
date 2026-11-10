$CONSOLE:ONLY
' PLAY event trapping statements compile (background-music refill pattern).
n% = 2
ON PLAY(n% + 1) GOSUB refill
PLAY ON
PLAY STOP
PLAY OFF
PRINT "play events"; n%
SYSTEM

refill:
PRINT "refill"
RETURN
