' Star Patrol: a small playable shooter. Left/Right to move, Space to fire,
' Esc to quit. With no key pressed the ship plays itself (attract mode).
' Shows _KEYDOWN input, _DISPLAY double buffering, _LIMIT frame pacing,
' sprites from DATA strings, particles and SOUND effects.
CONST SCRW = 640, SCRH = 400, NSTAR = 140, NFOE = 24, NSHOT = 16, NPART = 300
SCREEN _NEWIMAGE(SCRW, SCRH, 32)
_TITLE "QBNex Star Patrol"
RANDOMIZE 42
DIM SHARED stx(NSTAR), sty(NSTAR), stv(NSTAR)
DIM SHARED fx(NFOE), fy(NFOE), falive(NFOE), fkind(NFOE)
DIM SHARED shx(NSHOT), shy(NSHOT), shon(NSHOT)
DIM SHARED px(NPART), py(NPART), pvx(NPART), pvy(NPART), plife(NPART), pc(NPART) AS _UNSIGNED LONG
DIM SHARED spr$(2)
FOR k = 0 TO 2
    FOR r = 1 TO 8: READ row$: spr$(k) = spr$(k) + row$: NEXT
NEXT
' Three original 11x8 sprites: two foes and the player ship ('#' = pixel)
DATA "...#####...",".#########.","##.#.#.#.##","###########","..#.....#..",".#.#...#.#.","#.........#",".#.......#."
DATA "....###....","..#######..",".##.###.##.","###########","#.#######.#","#.#.....#.#","...##.##...","..#.....#.."
DATA ".....#.....","....###....","....#.#....","...#####...",".##.###.##.","###########","##.#####.##","#...#.#...#"

FOR i = 0 TO NSTAR: stx(i) = RND * SCRW: sty(i) = RND * SCRH: stv(i) = 0.3 + RND * 2.2: NEXT
FOR i = 0 TO NFOE - 1
    c = i MOD 12: rw = i \ 12 ' two curved chevrons
    fx(i) = 52 + c * 46: fy(i) = 50 + rw * 46 + ABS(c - 5.5) * 12: falive(i) = 1: fkind(i) = rw
NEXT
' Pre-render the planet once into an off-screen image, then _PUTIMAGE it every frame
planet& = _NEWIMAGE(240, 160, 32): _DEST planet&
FOR y = 0 TO 159
    FOR x = 0 TO 239
        dx = x - 120: dy = y - 80: d = SQR(dx * dx + dy * dy)
        IF d < 70 THEN
            l = (-dx * 0.6 - dy * 0.7 + SQR(4900 - d * d) * 0.4) / 70: IF l < 0.08 THEN l = 0.08
            band = 0.85 + 0.15 * SIN(dy / 6)
            PSET (x, y), _RGB32(170 * l * band, 110 * l * band, 230 * l * band)
        END IF
    NEXT
NEXT
FOR a = 0 TO 6.2832 STEP 0.002
    rx = COS(a): ry = SIN(a)
    IF ry > 0 OR ABS(rx * 110) > 70 THEN
        FOR rr = 98 TO 116 STEP 2
            PSET (120 + rx * rr, 80 + ry * rr * 0.2), _RGB32(150 + rr, 120 + rr / 2, 200)
        NEXT
    END IF
NEXT
_DEST 0
shipx = SCRW / 2: score = 0: hi = 12800: lives = 3: frame = 0: cool = 0

DO
    frame = frame + 1
    ' --- input, or the built-in pilot when no key is held
    demo = -1
    IF _KEYDOWN(19200) THEN shipx = shipx - 4: demo = 0
    IF _KEYDOWN(19712) THEN shipx = shipx + 4: demo = 0
    fire = _KEYDOWN(32): IF fire THEN demo = 0
    IF demo THEN
        target = Nearest(shipx)
        IF target >= 0 THEN
            IF fx(target) + 18 > shipx + 3 THEN shipx = shipx + 3
            IF fx(target) + 18 < shipx - 3 THEN shipx = shipx - 3
            IF ABS(fx(target) + 18 - shipx) < 20 THEN fire = -1
        END IF
    END IF
    IF shipx < 20 THEN shipx = 20
    IF shipx > SCRW - 20 THEN shipx = SCRW - 20
    cool = cool - 1
    IF fire AND cool <= 0 THEN
        FOR s = 0 TO NSHOT - 1
            IF shon(s) = 0 THEN shon(s) = 1: shx(s) = shipx: shy(s) = 352: cool = 9: SOUND 1400, 0.3: EXIT FOR
        NEXT
    END IF

    ' --- update
    sway = SIN(frame / 40) * 40
    FOR s = 0 TO NSHOT - 1
        IF shon(s) THEN
            shy(s) = shy(s) - 7: IF shy(s) < 20 THEN shon(s) = 0
            FOR i = 0 TO NFOE - 1
                IF falive(i) THEN
                    ex = fx(i) + sway: ey = fy(i) + 6 * SIN(frame / 15 + i)
                    IF shx(s) > ex AND shx(s) < ex + 33 AND shy(s) > ey AND shy(s) < ey + 24 THEN
                        falive(i) = 0: shon(s) = 0: score = score + 100 * (2 - fkind(i))
                        Burst ex + 16, ey + 12, fkind(i)
                        SOUND 180, 0.6
                    END IF
                END IF
            NEXT
        END IF
    NEXT
    IF Nearest(shipx) < 0 THEN ' wave cleared: a new one
        FOR i = 0 TO NFOE - 1: falive(i) = 1: NEXT
    END IF

    ' --- draw
    CLS , _RGB32(6, 6, 18)
    FOR i = 0 TO NSTAR
        sty(i) = sty(i) + stv(i): IF sty(i) > SCRH THEN sty(i) = 0: stx(i) = RND * SCRW
        b = 60 + stv(i) * 70: PSET (stx(i), sty(i)), _RGB32(b, b, b + 30)
    NEXT
    _PUTIMAGE (420, 220), planet&
    FOR i = 0 TO NFOE - 1
        IF falive(i) THEN
            IF fkind(i) THEN c~& = _RGB32(255, 110, 200) ELSE c~& = _RGB32(110, 255, 160)
            Sprite fkind(i), fx(i) + sway, fy(i) + 6 * SIN(frame / 15 + i), 3, c~&
        END IF
    NEXT
    FOR s = 0 TO NSHOT - 1
        IF shon(s) THEN LINE (shx(s) - 1, shy(s))-(shx(s) + 1, shy(s) + 10), _RGB32(255, 255, 140), BF
    NEXT
    FOR k = 0 TO NPART
        IF plife(k) > 0 THEN
            px(k) = px(k) + pvx(k): py(k) = py(k) + pvy(k): plife(k) = plife(k) - 1
            LINE (px(k), py(k))-(px(k) + 1, py(k) + 1), pc(k), BF
        END IF
    NEXT
    Sprite 2, shipx - 16, 350, 3, _RGB32(120, 200, 255)
    LINE (shipx - 3, 375)-(shipx + 3, 378 + RND * 8), _RGB32(255, 160, 40), BF
    ' HUD
    COLOR _RGB32(255, 255, 255): _PRINTMODE _KEEPBACKGROUND
    _PRINTSTRING (16, 8), "SCORE " + RIGHT$("000000" + LTRIM$(STR$(score)), 6)
    _PRINTSTRING (260, 8), "HI " + RIGHT$("000000" + LTRIM$(STR$(hi)), 6)
    _PRINTSTRING (500, 8), "LIVES"
    FOR l = 1 TO lives: Sprite 2, 545 + l * 22, 10, 1.5, _RGB32(120, 200, 255): NEXT
    IF demo THEN COLOR _RGB32(255, 220, 90): _PRINTSTRING (196, 384), "DEMO - press Space to play"
    _DISPLAY
    _LIMIT 60
LOOP UNTIL _KEYDOWN(27)
SYSTEM

' Index of a live foe closest to x, or -1 when the wave is cleared
FUNCTION Nearest (x)
    Nearest = -1: best = 1E+9
    FOR i = 0 TO NFOE - 1
        IF falive(i) THEN
            d = ABS(fx(i) + 18 - x) + fy(i) * -0.5
            IF d < best THEN best = d: Nearest = i
        END IF
    NEXT
END FUNCTION

SUB Sprite (k, x, y, size, c AS _UNSIGNED LONG)
    FOR r = 0 TO 7
        FOR col = 0 TO 10
            IF MID$(spr$(k), r * 11 + col + 1, 1) = "#" THEN
                LINE (x + col * size, y + r * size)-(x + col * size + size - 1, y + r * size + size - 1), c, BF
            END IF
        NEXT
    NEXT
END SUB

SUB Burst (x, y, kind)
    n = 0
    FOR k = 0 TO NPART
        IF plife(k) <= 0 THEN
            a = RND * 6.2832: v = RND * 3.5
            px(k) = x: py(k) = y: pvx(k) = COS(a) * v: pvy(k) = SIN(a) * v: plife(k) = 20 + RND * 25
            IF kind THEN pc(k) = _RGB32(255, 150 + RND * 100, 200) ELSE pc(k) = _RGB32(150 + RND * 100, 255, 120)
            n = n + 1: IF n > 40 THEN EXIT SUB
        END IF
    NEXT
END SUB
