$SCREENHIDE
$CONSOLE
SCREEN 7
W = 320: H = 200: c1 = 14: c2 = 4: c3 = 10: c4 = 9
tile$ = CHR$(&HFF) + CHR$(0) + CHR$(&HAA) + CHR$(&H55)
PALETTE 9, 2
COLOR 15, 1
' --- primitives (shared body; c1..c4 are mode colors) ---
PSET (5, 5), c1
PSET STEP(3, 0), c2
PRESET (12, 5)
PSET (-5, -5), c1
LINE (0, 0)-(W - 1, H - 1), c1
LINE (W - 1, 0)-(0, H - 1), c2, , &HF0F0
LINE -STEP(-20, -10), c3
LINE (10, 20)-(60, 50), c4, B
LINE (70, 20)-(120, 50), c2, BF
LINE (130, 20)-STEP(30, 30), c3, B, &HAAAA
LINE (-50, 100)-(W + 50, 110), c1
CIRCLE (W \ 2, H \ 2), 40, c3
CIRCLE (W \ 2, H \ 2), 30, c2, 0, 3.14159 / 2
CIRCLE (W \ 2, H \ 2), 25, c1, -0.5, -4
CIRCLE (W \ 2, H \ 2), 20, c4, 5, 1
CIRCLE (40, H - 40), 30, c2, , , 0.3
CIRCLE (100, H - 40), 30, c3, , , 2.5
CIRCLE STEP(30, 0), 10, c1
CIRCLE (0, 0), 25, c4
PAINT (35, 35), c3, c4
LINE (180, 60)-(230, 100), c1, B
PAINT (200, 80), c2, c1
PAINT (W - 3, 2), c4
LINE (240, 60)-(290, 100), c1, B
PAINT (260, 80), tile$, c1
VIEW (20, 120)-(120, 180), c1, c2
LINE (0, 0)-(200, 200), c3
CIRCLE (50, 30), 40, c4
VIEW
WINDOW (-1, -1)-(1, 1)
LINE (-1, 0)-(1, 0), c2
CIRCLE (0, 0), .5, c1
PSET (.25, .25), c3
px = PMAP(.5, 0): py = PMAP(.5, 1): wx = PMAP(px, 2): wy = PMAP(py, 3)
WINDOW SCREEN (0, 0)-(W * 2, H * 2)
LINE (0, 0)-(W, H), c4
WINDOW
pt1 = POINT(5, 5): pt2 = POINT(70, 30): pt3 = POINT(-1, 0)
cx = POINT(0): cy = POINT(1)
_DEST _CONSOLE
PRINT pt1; pt2; pt3; cx; cy; px; py; wx; wy
SYSTEM
