$SCREENHIDE
$CONSOLE
' SCREEN mode switches: sizes, fonts kept from the previous text layout,
' WIDTH overrides in SCREEN 0, text in every graphics mode
r$ = ""
SCREEN 0: GOSUB measure
WIDTH 40: GOSUB measure
WIDTH 40, 43: GOSUB measure
WIDTH 80, 50: GOSUB measure
SCREEN 0: GOSUB measure
WIDTH , 25: GOSUB measure
WIDTH 80, 43: GOSUB measure
SCREEN 9: GOSUB measure
PRINT "SCREEN 9 after 80x43"
_DISPLAY
SCREEN 0: GOSUB measure
SCREEN 12: GOSUB measure
WIDTH 80, 60: GOSUB measure
SCREEN 11: GOSUB measure
PRINT "SCREEN 11 after 80x60"
_DISPLAY
SCREEN 10: GOSUB measure
SCREEN 13: GOSUB measure
PRINT "SCREEN 13"
_DISPLAY
SCREEN 1: GOSUB measure
SCREEN 2: GOSUB measure
SCREEN 8: GOSUB measure
SCREEN 7: GOSUB measure
SCREEN 7, , 1, 0: GOSUB measure
PRINT "page 1"
SCREEN 7, , 0, 0
COLOR 14: PRINT "page 0"
PCOPY 1, 0
SCREEN 12
FOR c = 1 TO 15
    COLOR c: LOCATE c, c * 3: PRINT "QB"; c
NEXT
COLOR 15, 1: PRINT "end"
GOSUB measure
_DEST _CONSOLE
PRINT r$
SYSTEM

measure:
r$ = r$ + STR$(_WIDTH) + STR$(_HEIGHT) + STR$(_FONTWIDTH) + STR$(_FONTHEIGHT) + STR$(_PIXELSIZE) + CHR$(10)
RETURN
