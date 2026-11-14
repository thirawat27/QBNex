$CONSOLE
' PRINT that scrolls an image lower than one character row (it wrote past the
' image's pixels), and _COPYIMAGE to mode 33.
FOR k = 1 TO 20
    t& = _NEWIMAGE(640, 10, 32)
    _DEST t&
    PRINT "a"
    PRINT "b"
    _DEST 0
    _FREEIMAGE t&
    u& = _NEWIMAGE(3, 3, 256)
    _DEST u&
    PRINT "c"
    PRINT "d"
    _DEST 0
    _FREEIMAGE u&
NEXT
a& = _NEWIMAGE(4, 4, 32)
b& = _COPYIMAGE(a&, 33)
_DEST _CONSOLE
PRINT "alive"; b& < -1
SYSTEM
