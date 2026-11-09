$CONSOLE:ONLY
GOSUB first
PRINT "back in main"
n = 2
ON n GOSUB one, two, three
ON 3 GOTO one, two, three
PRINT "not reached"
one: PRINT "one": RETURN
two: PRINT "two": RETURN
three: PRINT "three"
GOTO 100
PRINT "skipped"
100 PRINT "line 100"
IF 1 THEN 200
PRINT "skipped too"
200 PRINT "done"
SYSTEM
first:
PRINT "in first"
RETURN
