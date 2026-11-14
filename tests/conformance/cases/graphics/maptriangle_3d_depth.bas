$CONSOLE
' The 3D form of _MAPTRIANGLE projects with perspective (it was drawn at the x, y
' values as a 2D triangle), the depth buffer keeps the nearer triangle, and
' _CLOCKWISE culls the other winding.
SCREEN _NEWIMAGE(320, 240, 32)
red& = _NEWIMAGE(8, 8, 32): _DEST red&: CLS , _RGB32(255, 0, 0)
grn& = _NEWIMAGE(8, 8, 32): _DEST grn&: CLS , _RGB32(0, 255, 0)
_DEST 0
_DEPTHBUFFER ON
_MAPTRIANGLE (0, 0)-(7, 0)-(0, 7), red& TO(-1, 1, -3)-(1, 1, -3)-(-1, -1, -3)
_MAPTRIANGLE (0, 0)-(7, 0)-(0, 7), grn& TO(-5, 5, -9)-(5, 5, -9)-(-5, -5, -9)
_SOURCE 0
n& = 0: g& = 0
FOR y = 0 TO 239: FOR x = 0 TO 319
    c~& = POINT(x, y)
    IF c~& = _RGB32(255, 0, 0) THEN n& = n& + 1
    IF c~& = _RGB32(0, 255, 0) THEN g& = g& + 1
NEXT: NEXT
_DEST _CONSOLE
PRINT "red in front"; n& > 1000; "green behind it"; g& > 1000; HEX$(POINT(150, 100))
_DEST 0: CLS: _DEPTHBUFFER _CLEAR
_MAPTRIANGLE _CLOCKWISE (0, 0)-(0, 7)-(7, 0), red& TO(-1, 1, -3)-(-1, -1, -3)-(1, 1, -3)
_SOURCE 0
_DEST _CONSOLE
PRINT "culled"; POINT(150, 100) = _RGB32(0, 0, 0)
ON ERROR GOTO h
p& = _NEWIMAGE(8, 8, 256)
_MAPTRIANGLE (0, 0)-(7, 0)-(0, 7), p& TO(-1, 1, -3)-(1, 1, -3)-(-1, -1, -3), red&
SYSTEM
h:
_DEST _CONSOLE: PRINT "error"; ERR: RESUME NEXT
