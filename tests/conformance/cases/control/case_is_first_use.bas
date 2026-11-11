$CONSOLE:ONLY
' SELECT CASE IS with a variable that appears there for the first time
x = 5
SELECT CASE x
CASE IS >= lim: PRINT "ge"
CASE ELSE: PRINT "lt"
END SELECT
lim = 3
SELECT CASE x
CASE IS < lim, 9 TO first2: PRINT "low"
CASE IS > lim: PRINT "high"
END SELECT
SYSTEM
