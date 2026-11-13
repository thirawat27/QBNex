$CONSOLE
_DEST _CONSOLE
' Memory-mode program: DIM with constant bounds is static, so the array exists
' even when its DIM statement never runs (it is allocated before the program starts).
IF C& = 0 THEN PRINT "first" ELSE DIM M%(8)
M%(3) = 7: PRINT M%(3)
SYSTEM
GET (1, 1)-(2, 2), M%
