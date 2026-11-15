' each $IF branch opens the block that is closed after $END IF
$IF WIN THEN
IF a = 0 THEN
$ELSE
IF a = 1 THEN
$END IF
    PRINT 1
END IF
$IF WIN THEN
DO WHILE i < 2
$ELSE
DO WHILE i < 3
$END IF
    i = i + 1
LOOP
