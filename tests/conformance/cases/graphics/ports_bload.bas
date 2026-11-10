$SCREENHIDE
$CONSOLE
' OUT/INP on the VGA DAC (&H3C7-&H3C9), WAIT on the retrace bit (&H3DA),
' BSAVE/BLOAD of SCREEN 13 memory (DEF SEG = &HA000) and of an array
' (DEF SEG = VARSEG, VARPTR), PEEK/POKE through VARPTR, TIME$ = (no effect).
SCREEN 13
OUT &H3C8, 5: OUT &H3C9, 63: OUT &H3C9, 32: OUT &H3C9, 0
OUT &H3C7, 5: r = INP(&H3C9): g = INP(&H3C9): b = INP(&H3C9)
p& = _PALETTECOLOR(5)
LINE (10, 10)-(40, 30), 5, BF
PSET (100, 100), 9
WAIT &H3DA, 8
WAIT &H3DA, 8, 8
DEF SEG = &HA000
v1 = PEEK(10 * 320 + 10)
BSAVE "ports_scr.tmp", 0, 64000
CLS
v2 = PEEK(10 * 320 + 10)
BLOAD "ports_scr.tmp"
v3 = POINT(20, 20): v4 = POINT(100, 100)
DEF SEG
DIM a(1 TO 8) AS INTEGER, b(1 TO 8) AS INTEGER
FOR i = 1 TO 8: a(i) = i * 1000 + i: NEXT
DEF SEG = VARSEG(a(1))
BSAVE "ports_arr.tmp", VARPTR(a(1)), 16
DEF SEG = VARSEG(b(1))
BLOAD "ports_arr.tmp", VARPTR(b(1))
k = 7: q = PEEK(VARPTR(k))
POKE VARPTR(b(2)), 1
DEF SEG
KILL "ports_scr.tmp": KILL "ports_arr.tmp"
TIME$ = "12:00:00"
_DEST _CONSOLE
PRINT r; g; b; HEX$(p&); v1; v2; v3; v4
FOR i = 1 TO 8: PRINT b(i);: NEXT
PRINT
PRINT q
SYSTEM
