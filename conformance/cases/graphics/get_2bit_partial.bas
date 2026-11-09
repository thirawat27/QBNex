$SCREENHIDE
$CONSOLE
' SCREEN 1 GET of a width that is not a multiple of 4 pixels, then PUT back
SCREEN 1
FOR y = 0 TO 3: FOR x = 0 TO 6: PSET (x, y), (x + y) MOD 4: NEXT: NEXT
DIM a%(20)
GET (0, 0)-(6, 3), a%
PUT (50, 50), a%, PSET
bad = 0
FOR y = 0 TO 3: FOR x = 0 TO 6
        IF POINT(50 + x, 50 + y) <> POINT(x, y) THEN bad = bad + 1
NEXT: NEXT
r$ = STR$(a%(0)) + STR$(a%(1))
FOR i = 2 TO 5: r$ = r$ + " " + HEX$(a%(i) AND &HFFFF&): NEXT
SCREEN 0
_DEST _CONSOLE
PRINT bad; r$
SYSTEM
