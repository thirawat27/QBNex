$SCREENHIDE
$CONSOLE
DIM a%(2000), b&(4000)
FOR m = 1 TO 4
  SELECT CASE m
    CASE 1: SCREEN 1
    CASE 2: SCREEN 12
    CASE 3: SCREEN 13
    CASE 4: SCREEN _NEWIMAGE(160, 120, 32)
  END SELECT
  FOR i = 0 TO 30
    LINE (i * 2, 0)-(60 - i, 40), (i MOD 15) + 1
  NEXT
  CIRCLE (30, 20), 15, 2
  GET (0, 0)-(31, 23), a%
  GET (5, 5)-(16, 14), b&
  PUT (70, 0), a%, PSET
  PUT (70, 30), a%, PRESET
  PUT (110, 0), a%
  PUT (110, 0), a%, XOR
  PUT (110, 30), b&, AND
  PUT (110, 50), b&, OR
  PUT (5, 60), a%
  PUT (6, 61), a%, XOR
  _DEST _CONSOLE
  PRINT m; a%(0); a%(1); a%(2); a%(3); b&(1); b&(2)
  _DEST 0
  _DISPLAY
NEXT
SYSTEM
