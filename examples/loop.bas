' loop.bas
PRINT "Even and Odd Numbers (1-20)"
PRINT

FOR i = 1 TO 20
    IF i MOD 2 = 0 THEN
        PRINT i; " is even"
    ELSE
        PRINT i; " is odd"
    END IF
NEXT i

PRINT
PRINT "Countdown"
count = 10
DO WHILE count > 0
    PRINT count
    count = count - 1
LOOP
PRINT "Blast off!"
