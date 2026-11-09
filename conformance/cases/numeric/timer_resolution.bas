$CONSOLE:ONLY
' TIMER (qbrt::timer): value range and resolution, not the time itself.
t! = TIMER
PRINT t! >= 0 AND t! < 86400
m# = TIMER(0.001)
PRINT m# >= 0 AND m# < 86400
PRINT ABS(m# * 1000 - CLNG(m# * 1000)) < 0.0001
w# = TIMER(1)
PRINT w# = INT(w#)
ON ERROR GOTO 0
SYSTEM
