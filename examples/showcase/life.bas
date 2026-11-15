' Conway's Game of Life on a wrapping 128 x 76 grid. Live cells are coloured
' by age and dead cells leave a fading trail. Any key closes.
' Space restarts with a new random world.
CONST GW = 128, GH = 76, CELL = 5, TOP = 20
SCREEN _NEWIMAGE(GW * CELL, GH * CELL + TOP, 32)
_TITLE "QBNex Game of Life"
DIM SHARED cur(GW - 1, GH - 1) AS INTEGER, nxt(GW - 1, GH - 1) AS INTEGER, heat(GW - 1, GH - 1)
RANDOMIZE TIMER
Seed

DO
    ' One generation: count the eight neighbours with wrap-around
    pop = 0
    FOR x = 0 TO GW - 1
        xl = (x + GW - 1) MOD GW: xr = (x + 1) MOD GW
        FOR y = 0 TO GH - 1
            yu = (y + GH - 1) MOD GH: yd = (y + 1) MOD GH
            n = SGN(cur(xl, yu)) + SGN(cur(x, yu)) + SGN(cur(xr, yu)) + SGN(cur(xl, y)) + SGN(cur(xr, y)) + SGN(cur(xl, yd)) + SGN(cur(x, yd)) + SGN(cur(xr, yd))
            IF cur(x, y) THEN
                IF n = 2 OR n = 3 THEN nxt(x, y) = cur(x, y) + 1 ELSE nxt(x, y) = 0: heat(x, y) = 1
            ELSE
                IF n = 3 THEN nxt(x, y) = 1 ELSE nxt(x, y) = 0
            END IF
        NEXT
    NEXT
    ' Draw and swap
    LINE (0, 0)-(_WIDTH - 1, TOP - 1), _RGB32(16, 18, 30), BF
    FOR x = 0 TO GW - 1
        FOR y = 0 TO GH - 1
            cur(x, y) = nxt(x, y)
            px = x * CELL: py = y * CELL + TOP
            IF cur(x, y) THEN
                pop = pop + 1
                a = cur(x, y): IF a > 40 THEN a = 40
                t = a / 40
                LINE (px, py)-(px + CELL - 2, py + CELL - 2), _RGB32(120 + 135 * (1 - t), 255 - 150 * t, 255), BF
            ELSE
                h = heat(x, y)
                LINE (px, py)-(px + CELL - 1, py + CELL - 1), _RGB32(8 + 90 * h, 10 + 20 * h, 24 + 110 * h), BF
                heat(x, y) = h * 0.9
            END IF
        NEXT
    NEXT
    gen = gen + 1
    COLOR _RGB32(220, 230, 255), _RGB32(16, 18, 30)
    _PRINTSTRING (8, 2), "Generation" + STR$(gen) + "    Population" + STR$(pop) + "    Space = new world"
    _DISPLAY
    _LIMIT 30
    k$ = INKEY$
    IF k$ = " " THEN Seed: gen = 0: k$ = ""
LOOP UNTIL k$ <> ""
SYSTEM

SUB Seed
    FOR x = 0 TO GW - 1
        FOR y = 0 TO GH - 1
            IF RND < 0.28 THEN cur(x, y) = 1 ELSE cur(x, y) = 0
            heat(x, y) = 0
        NEXT
    NEXT
END SUB
