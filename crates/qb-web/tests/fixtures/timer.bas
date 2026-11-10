t1# = TIMER(.001)
_DELAY .2
t2# = TIMER(.001)
d# = t2# - t1#
IF d# < 0 THEN d# = d# + 86400
IF d# >= .15 AND d# < 5 THEN PRINT "timer ok" ELSE PRINT "timer bad"; d#
IF TIMER >= 0 AND TIMER < 86400 THEN PRINT "range ok"
