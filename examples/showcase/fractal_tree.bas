' Recursive fractal tree at sunset: recursion, trigonometry, gradients.
CONST SCRW = 640, SCRH = 400, PI = 3.14159265
SCREEN _NEWIMAGE(SCRW, SCRH, 32)
_TITLE "QBNex fractal tree"
RANDOMIZE 7

' Sky gradient with a glowing sun, one pixel at a time
FOR y = 0 TO SCRH - 1
    t = y / SCRH
    FOR x = 0 TO SCRW - 1
        d = SQR((x - 470) ^ 2 + (y - 300) ^ 2)
        glow = 1 - d / 170: IF glow < 0 THEN glow = 0
        glow = glow * glow
        r = 40 + 215 * t + 200 * glow: g = 30 + 120 * t ^ 1.5 + 150 * glow: b = 90 + 40 * t + 40 * glow
        IF d < 42 THEN r = 255: g = 240: b = 190
        PSET (x, y), _RGB32(r, g, b)
    NEXT
NEXT

' Rolling hills
FOR x = 0 TO SCRW - 1
    h = 335 + 18 * SIN(x / 70) + 9 * SIN(x / 23 + 1)
    LINE (x, h)-(x, SCRH - 1), _RGB32(35, 22, 40)
NEXT

Branch 220, 345, PI / 2, 78, 11
SLEEP
SYSTEM

SUB Branch (x, y, ang, length, depth)
    IF depth = 0 THEN
        ' Blossom
        FOR i = 1 TO 3
            bx = x + RND * 8 - 4: by = y + RND * 8 - 4: br = 1 + RND * 2
            FOR rr = 0 TO br: CIRCLE (bx, by), rr, _RGB32(255, 120 + RND * 80, 160 + RND * 60): NEXT
        NEXT
        EXIT SUB
    END IF
    x2 = x + COS(ang) * length
    y2 = y - SIN(ang) * length
    shade = depth / 11
    FOR w = -depth \ 3 TO depth \ 3
        LINE (x + w, y)-(x2 + w, y2), _RGB32(30 + 40 * (1 - shade), 18 + 20 * (1 - shade), 24)
    NEXT
    spread = 0.38 + RND * 0.12
    Branch x2, y2, ang + spread, length * (0.68 + RND * 0.1), depth - 1
    Branch x2, y2, ang - spread, length * (0.68 + RND * 0.1), depth - 1
END SUB
