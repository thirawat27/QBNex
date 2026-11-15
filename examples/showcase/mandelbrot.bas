' Mandelbrot set with smooth colouring: a deep zoom into "seahorse valley".
' 256,000 pixels x up to 600 iterations in double precision.
DEFDBL A-Z
CONST SCRW = 640, SCRH = 400, MAXIT = 600
SCREEN _NEWIMAGE(SCRW, SCRH, 32)
_TITLE "QBNex Mandelbrot"

cx = -0.7435: cy = 0.1314: span = 0.0072
FOR py = 0 TO SCRH - 1
    FOR px = 0 TO SCRW - 1
        x0 = cx + (px - SCRW / 2) * span / SCRW
        y0 = cy + (py - SCRH / 2) * span / SCRW
        x = 0: y = 0: it = 0
        DO WHILE x * x + y * y < 256 AND it < MAXIT
            xt = x * x - y * y + x0
            y = 2 * x * y + y0
            x = xt
            it = it + 1
        LOOP
        IF it = MAXIT THEN
            PSET (px, py), _RGB32(0, 0, 0)
        ELSE
            mu = it + 1 - LOG(LOG(SQR(x * x + y * y))) / LOG(2)
            t = SQR(mu) * 0.55
            r = 128 + 127 * SIN(t + 4.3)
            g = 128 + 127 * SIN(t + 5.1)
            b = 128 + 127 * SIN(t + 5.9)
            PSET (px, py), _RGB32(r, g, b)
        END IF
    NEXT
NEXT
SLEEP
SYSTEM
