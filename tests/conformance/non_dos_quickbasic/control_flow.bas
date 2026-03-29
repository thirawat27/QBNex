OPTION BASE 1
total = 0

FOR i = 1 TO 3
    total = total + i
NEXT

IF total = 6 THEN PRINT "flow"

SELECT CASE total
CASE 6
    PRINT "select"
CASE ELSE
    PRINT "bad"
END SELECT

DO
    total = total - 1
LOOP UNTIL total = 4

PRINT total
