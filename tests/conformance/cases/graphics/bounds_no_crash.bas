$CONSOLE
' Statements that wrote outside the pixel buffer or read outside the image table.
ON ERROR GOTO h
FOR k = 1 TO 10
    big& = _NEWIMAGE(5000, 5000, 256)
    _DEST big&
    LINE (0, 2832)-(4999, 4999), 7
    _DEST 0
    _FREEIMAGE big&
NEXT
FOR k = 1 TO 10
    tiny& = _NEWIMAGE(3, 3, 13)
    _DEST tiny&
    PRINT "A";
    _DEST 0
    _FREEIMAGE tiny&
NEXT
_DEST _CONSOLE
PRINT "drawn"
x& = _WIDTH(-2147483648)
SCREEN _NEWIMAGE(640, 480, 32)
WIDTH 20000, 10000
_DEST _CONSOLE
PRINT "done"
SYSTEM
h:
_DEST _CONSOLE
PRINT "error"; ERR
RESUME NEXT
