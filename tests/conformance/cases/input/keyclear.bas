$CONSOLE
' _KEYCLEAR empties the INKEY$ and _KEYHIT buffers; a buffer number outside 1-3 is an error.
SCREEN 12
_DEST _CONSOLE
_DELAY 1.5
_KEYCLEAR 1
PRINT "inkey ["; INKEY$; "]"
PRINT "keyhit"; _KEYHIT <> 0
_KEYCLEAR 2
PRINT "keyhit"; _KEYHIT
_DELAY 1.5
_KEYCLEAR
PRINT "inkey ["; INKEY$; "]"; _KEYHIT
ON ERROR GOTO oops
_KEYCLEAR 4
_KEYCLEAR 0
_KEYCLEAR 3
PRINT "done"
SYSTEM
oops:
PRINT "error"; ERR
RESUME NEXT
