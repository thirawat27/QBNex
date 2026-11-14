$CONSOLE:ONLY
' Every ordered comparison with NaN is false, in IF and loop conditions as in an
' expression (IF n < 1 was taken: its "not less" jump was a "greater or equal" one).
DIM x AS DOUBLE, n AS DOUBLE, s AS SINGLE
x = 1E+308
x = x * 10
n = x - x
IF n < 1 THEN PRINT "lt wrong" ELSE PRINT "lt ok"
IF n > 1 THEN PRINT "gt wrong" ELSE PRINT "gt ok"
IF n <= 1 THEN PRINT "le wrong" ELSE PRINT "le ok"
IF n >= 1 THEN PRINT "ge wrong" ELSE PRINT "ge ok"
IF n = n THEN PRINT "eq wrong" ELSE PRINT "eq ok"
IF n <> n THEN PRINT "ne ok" ELSE PRINT "ne wrong"
c = 0
DO
    c = c + 1
    IF c > 3 THEN PRINT "DO LOOP WHILE wrong": EXIT DO
LOOP WHILE n < 10
PRINT "c ="; c
c = 0
WHILE n > 0
    c = c + 1
    IF c > 3 THEN PRINT "WHILE wrong": GOTO out1
WEND
out1:
c = 0
DO UNTIL n < 5
    c = c + 1
    IF c >= 3 THEN EXIT DO
LOOP
PRINT "DO UNTIL ran"; c
s = n
IF s < 1 THEN PRINT "single wrong" ELSE PRINT "single ok"
v% = (n < 1)
PRINT "value of (n < 1):"; v%
FOR d# = 1 TO 3
    t = t + d#
NEXT
PRINT "FOR:"; t
SYSTEM
