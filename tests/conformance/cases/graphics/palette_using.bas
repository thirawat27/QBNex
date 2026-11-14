$CONSOLE
' PALETTE USING sets one attribute an element; -1 leaves an attribute as it is.
SCREEN 13
DIM pal&(255)
FOR i = 0 TO 255: pal&(i) = i MOD 64: NEXT: pal&(7) = -1: pal&(5) = 63
PALETTE USING pal&(0)
_DEST _CONSOLE: PRINT HEX$(_PALETTECOLOR(5, 0)); " "; HEX$(_PALETTECOLOR(1, 0))
PRINT HEX$(_PALETTECOLOR(7, 0))
SYSTEM
