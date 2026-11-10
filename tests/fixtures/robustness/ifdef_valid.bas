' $IF / $ELSEIF / $ELSE / $END IF must compile (they hit an internal error before R14)
$IF WIN THEN
PRINT "windows"
$ELSEIF LINUX THEN
PRINT "linux"
$ELSE
PRINT "other"
$END IF
$IF WIN THEN
$IF 64BIT THEN
PRINT 64
$END IF
$END IF
