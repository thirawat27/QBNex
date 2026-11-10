$CONSOLE:ONLY
' An unhandled error under QBNEX_NO_ERROR_DIALOG=1 (set by the runner): the
' message goes to stderr, the program ends with exit code 1, no dialog, and
' everything printed before the error is kept.
PRINT "before"
PRINT "same line ";
ERROR 53
PRINT "never printed"
