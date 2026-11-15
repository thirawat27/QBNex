$CONSOLE:ONLY
' $IF only selects lines: a block may open inside a $IF branch and close after
' $END IF, or open before the $IF and close inside it ($END IF used to close
' whatever block was open: "END IF without IF").
$IF 1 THEN
IF 1 THEN
$END IF
    PRINT "if opened in $IF"
END IF
$IF 1 THEN
DO WHILE i < 2
$END IF
    i = i + 1
LOOP
PRINT "do"; i
IF 1 THEN
$IF 1 THEN
    PRINT "closed in $IF"
END IF
$END IF
SELECT CASE 2
$IF 1 THEN
    CASE 1: PRINT "one"
    CASE 2: PRINT "two"
$END IF
END SELECT
FOR k = 1 TO 2
$IF 1 THEN
NEXT
$END IF
PRINT "for"; k
w = 0
WHILE w < 3
$IF 0 THEN
    w = 99
$ELSEIF 1 THEN
    w = w + 1
$END IF
WEND
PRINT "while"; w
s
SYSTEM
SUB s
$IF 1 THEN
    PRINT "sub"
END SUB
$END IF
