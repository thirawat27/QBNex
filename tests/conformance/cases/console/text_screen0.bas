' R12a: SCREEN 0 cell buffer, PRINT layout, LOCATE/CSRLIN/POS, VIEW PRINT, CLS,
' control characters and SCREEN(); the screen page is read back with SCREEN().
$CONSOLE
SCREEN 0
WIDTH 40, 12
COLOR 7, 0
CLS
PRINT "ABC";
PRINT 12; -3; 4.5
PRINT "x", "y", "z"
PRINT "tab:"; TAB(10); "t"; SPC(3); "s"
r1 = CSRLIN: c1 = POS(0)
LOCATE 6, 35: PRINT "WRAPPED";
r2 = CSRLIN: c2 = POS(0)
LOCATE 8, 1: COLOR 14, 1: PRINT "Col"; : COLOR 7, 0
PRINT CHR$(28); CHR$(28); "^"; CHR$(29); CHR$(29); "<"
LOCATE 9, 40: PRINT "E";
r3 = CSRLIN: c3 = POS(0)
PRINT "n"
VIEW PRINT 10 TO 11
PRINT "v1": PRINT "v2": PRINT "v3";
r4 = CSRLIN
VIEW PRINT
LOCATE 1, 1, 0
_DEST _CONSOLE
PRINT r1; c1; r2; c2; r3; c3; r4
FOR y = 1 TO 12
    l$ = ""
    FOR x = 1 TO 40
        l$ = l$ + CHR$(SCREEN(y, x))
    NEXT
    PRINT "|"; RTRIM$(l$); "|"
NEXT
PRINT SCREEN(8, 1, 1); SCREEN(8, 4, 1); SCREEN(1, 1, 1)
_DEST 0
CLS 2
_DEST _CONSOLE
PRINT "after cls:"; CHR$(SCREEN(1, 1)); SCREEN(1, 1, 1)
SYSTEM
