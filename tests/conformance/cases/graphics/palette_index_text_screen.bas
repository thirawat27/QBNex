$CONSOLE
' _RED/_GREEN/_BLUE of a colour number past the 256 palette entries of a text page is
' an illegal function call (it read far past the palette), and VIEW is a graphics
' statement: error 5 on a text page (its border was written past the page).
ON ERROR GOTO h
_DEST _CONSOLE
PRINT _RED(40000, 0)
PRINT _RED(1, 0); _GREEN(2, 0); _BLUE(1, 0)
d& = _NEWIMAGE(80, 25, 0)
_DEST d&
VIEW (1, 1)-(78, 23), , &HFFFFFFFF
_DEST _CONSOLE
PRINT "done"
SYSTEM
h:
_DEST _CONSOLE
PRINT "error"; ERR
RESUME NEXT
