$CONSOLE
' _PRINTSTRING puts text at a pixel (a cell of a text screen) without moving the
' cursor or scrolling; _MEMIMAGE gives the pixels of an image as a _MEM block.
DIM m AS _MEM
ON ERROR GOTO h
SCREEN 13
COLOR 15, 1
_PRINTSTRING (10, 20), "AB"
FOR y = 20 TO 27: FOR x = 10 TO 25
        IF POINT(x, y) = 15 THEN ink = ink + 1
        IF POINT(x, y) = 1 THEN paper = paper + 1
NEXT: NEXT
_DEST _CONSOLE
PRINT "cells"; ink; paper; ink + paper
PRINT "around"; POINT(9, 20); POINT(26, 20); POINT(10, 19); POINT(10, 28)
_DEST 0
' only the ink, then relative to the last position, then across the edge
_PRINTMODE _KEEPBACKGROUND
_PRINTSTRING (100, 100), "A"
_PRINTSTRING STEP(8, 0), "A"
_PRINTSTRING (316, 196), "W"
_PRINTSTRING (-4, -4), "W"
n = 0: n2 = 0
FOR y = 100 TO 107: FOR x = 100 TO 107
        IF POINT(x, y) = 15 THEN n = n + 1
        IF POINT(x + 8, y) = 15 THEN n2 = n2 + 1
        IF POINT(x, y) = 1 THEN paper2 = paper2 + 1
NEXT: NEXT
r = CSRLIN: c = POS(0)
_DEST _CONSOLE
PRINT "keep"; n; n2; paper2; r; c
_DEST 0
' on another image
i& = _NEWIMAGE(32, 16, 32)
_PRINTSTRING (0, 0), "Hi", i&
_SOURCE i&
n = 0
FOR y = 0 TO 15: FOR x = 0 TO 31
        IF POINT(x, y) = _RGB32(255, 255, 255) THEN n = n + 1
NEXT: NEXT
_SOURCE 0
_DEST _CONSOLE
PRINT "image"; n > 20; _DEST = _CONSOLE
_PRINTSTRING (0, 0), "x", -99
_DEST 0
' a text screen: row 3, column 5; the cursor stays
SCREEN 0
LOCATE 7, 9
_PRINTSTRING (5, 3), "text"
r = CSRLIN: c = POS(0)
a = SCREEN(3, 5): b = SCREEN(3, 8)
_DEST _CONSOLE
PRINT "text"; a; b; r; c
_DEST 0
' the pixels of the screen and of an image
SCREEN 13
PSET (3, 2), 9
m = _MEMIMAGE
v~%% = _MEMGET(m, m.OFFSET + 2 * 320 + 3, _UNSIGNED _BYTE)
_MEMPUT m, m.OFFSET + 5, 12 AS _UNSIGNED _BYTE
p = POINT(5, 0)
_DEST _CONSOLE
PRINT "screen"; m.SIZE; m.ELEMENTSIZE; v~%%; p
m = _MEMIMAGE(i&)
PRINT "image"; m.SIZE; m.ELEMENTSIZE; _MEMEXISTS(m)
_FREEIMAGE i&
PRINT "freed"; _MEMEXISTS(m)
m = _MEMIMAGE(-99)
s& = _SCREENIMAGE
PRINT "desktop"; _WIDTH(s&) > 0; _HEIGHT(s&) > 0; _PIXELSIZE(s&)
_FREEIMAGE s&
PRINT "end"
SYSTEM
h: _DEST _CONSOLE: PRINT "error"; ERR: RESUME NEXT
