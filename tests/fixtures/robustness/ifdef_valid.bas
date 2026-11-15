' $IF / $ELSEIF / $ELSE / $END IF must compile (they used to hit an internal error)
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
