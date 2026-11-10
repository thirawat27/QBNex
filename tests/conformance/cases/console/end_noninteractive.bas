$CONSOLE:ONLY
' END in a console program waits for a key only when stdin and stdout are the
' console. Under the runner (redirected, QBNEX_NO_ERROR_DIALOG=1) it ends at once.
PRINT "bye"
END
