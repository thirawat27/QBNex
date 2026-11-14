$CONSOLE:ONLY
' An include line in a $IF branch that is not compiled opens no file
$IF 0 THEN
    '$INCLUDE:'no_such_file_anywhere.bi'
$END IF
PRINT "compiled"
SYSTEM
