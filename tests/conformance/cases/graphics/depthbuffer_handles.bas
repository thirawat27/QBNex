$CONSOLE:ONLY
' _DEPTHBUFFER: the screen (0 or 1, or no handle) is accepted; a handle above 1
' is error 5 and a negative handle that is not a hardware image is error 258.
ON ERROR GOTO trap
_DEPTHBUFFER ON
_DEPTHBUFFER LOCK
_DEPTHBUFFER _CLEAR
_DEPTHBUFFER OFF, 0
_DEPTHBUFFER ON, 1
PRINT "screen ok"
_DEPTHBUFFER ON, 2
PRINT "after 2"
_DEPTHBUFFER _CLEAR, -12345
PRINT "after negative"
SYSTEM
trap:
PRINT "error"; ERR
RESUME NEXT
