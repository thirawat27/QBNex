OPTION BASE 1
DIM total AS LONG
DIM values(1 TO 3) AS INTEGER
name$ = "QBNex"

FOR i = 1 TO 3
    values(i) = i * 10
    total = total + values(i)
NEXT

IF total > 0 THEN PRINT name$

SELECT CASE values(2)
CASE 20
    PRINT total
CASE ELSE
    PRINT 0
END SELECT
