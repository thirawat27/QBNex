' a block opened inside $IF must still be closed: $END IF does not close it
$IF 1 THEN
IF 1 THEN
$END IF
PRINT 1
