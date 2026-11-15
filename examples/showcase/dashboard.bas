' Sales dashboard from DATA statements: KPI cards, a bar chart with a trend
' line and a donut chart. The same code could read a CSV with LINE INPUT.
CONST PI = 3.14159265
SCREEN _NEWIMAGE(640, 400, 32)
_TITLE "QBNex sales dashboard"
DIM sales(1 TO 12), month$(1 TO 12), share(1 TO 4), region$(1 TO 4), rc(1 TO 4) AS _UNSIGNED LONG
FOR i = 1 TO 12: READ month$(i), sales(i): total = total + sales(i): NEXT
DATA Jan,42,Feb,48,Mar,45,Apr,61,May,58,Jun,72,Jul,69,Aug,81,Sep,77,Oct,90,Nov,96,Dec,112
FOR i = 1 TO 4: READ region$(i), share(i): NEXT
DATA North,34,South,27,East,22,West,17
rc(1) = _RGB32(86, 156, 255): rc(2) = _RGB32(255, 159, 67): rc(3) = _RGB32(46, 204, 138): rc(4) = _RGB32(238, 90, 125)

bg~& = _RGB32(18, 22, 34): card~& = _RGB32(30, 36, 54): muted~& = _RGB32(140, 150, 175)
CLS , bg~&
_PRINTMODE _KEEPBACKGROUND
COLOR _RGB32(235, 240, 255): _PRINTSTRING (20, 14), "Sales overview 2026"
COLOR muted~&: _PRINTSTRING (440, 14), "QBNex BASIC report"

' KPI cards
FOR k = 0 TO 2
    x = 20 + k * 205
    LINE (x, 40)-(x + 190, 96), card~&, BF
    LINE (x, 40)-(x + 3, 96), rc(k + 1), BF
NEXT
COLOR muted~&: _PRINTSTRING (34, 50), "Revenue (k$)": _PRINTSTRING (239, 50), "Best month": _PRINTSTRING (444, 50), "Growth Jan-Dec"
COLOR _RGB32(255, 255, 255)
_PRINTSTRING (34, 70), LTRIM$(STR$(total))
_PRINTSTRING (239, 70), "Dec  " + LTRIM$(STR$(sales(12)))
_PRINTSTRING (444, 70), "+" + LTRIM$(STR$(INT((sales(12) / sales(1) - 1) * 100))) + "%"
COLOR rc(3): _PRINTSTRING (118, 70), CHR$(24) + " 18% YoY"

' Bar chart
LINE (20, 110)-(400, 385), card~&, BF
COLOR _RGB32(235, 240, 255): _PRINTSTRING (32, 118), "Monthly revenue"
x0 = 52: y0 = 360: ch = 200: mx = 120
FOR g = 0 TO 120 STEP 30
    y = y0 - g / mx * ch
    LINE (x0, y)-(388, y), _RGB32(45, 52, 74)
    COLOR muted~&: _PRINTSTRING (24, y - 8), RIGHT$("  " + LTRIM$(STR$(g)), 3)
NEXT
FOR i = 1 TO 12
    bx = x0 + 6 + (i - 1) * 28: bh = sales(i) / mx * ch
    FOR yy = 0 TO bh
        t = yy / ch
        LINE (bx, y0 - yy)-(bx + 17, y0 - yy), _RGB32(60 + 50 * t, 110 + 60 * t, 220 + 35 * t)
    NEXT
    COLOR muted~&: _PRINTSTRING (bx + 1, 366), LEFT$(month$(i), 2)
NEXT
' Trend line (least squares)
FOR i = 1 TO 12: sx = sx + i: sy = sy + sales(i): sxy = sxy + i * sales(i): sxx = sxx + i * i: NEXT
slope = (12 * sxy - sx * sy) / (12 * sxx - sx * sx): icpt = (sy - slope * sx) / 12
FOR w = -1 TO 1
    LINE (x0 + 15, y0 - (slope + icpt) / mx * ch + w)-(x0 + 15 + 11 * 28, y0 - (slope * 12 + icpt) / mx * ch + w), rc(2)
NEXT

' Donut chart
LINE (412, 110)-(620, 385), card~&, BF
COLOR _RGB32(235, 240, 255): _PRINTSTRING (424, 118), "Revenue by region"
cx = 516: cy = 225
FOR y = cy - 80 TO cy + 80
    FOR x = cx - 80 TO cx + 80
        d = SQR((x - cx) ^ 2 + (y - cy) ^ 2)
        IF d >= 46 AND d <= 78 THEN
            ' Clockwise from 12 o'clock, as a fraction of the full circle
            f = (PI / 2 - _ATAN2(cy - y, x - cx)) / (2 * PI): IF f < 0 THEN f = f + 1
            acc = 0
            FOR i = 1 TO 4
                acc = acc + share(i) / 100
                IF f <= acc THEN PSET (x, y), rc(i): EXIT FOR
            NEXT
        END IF
    NEXT
NEXT
COLOR _RGB32(255, 255, 255): _PRINTSTRING (cx - 20, cy - 8), "100%"
FOR i = 1 TO 4
    ly = 318 + ((i - 1) \ 2) * 26: lx = 428 + ((i - 1) MOD 2) * 96
    LINE (lx, ly + 3)-(lx + 10, ly + 13), rc(i), BF
    COLOR muted~&: _PRINTSTRING (lx + 16, ly), region$(i) + STR$(share(i)) + "%"
NEXT
SLEEP
SYSTEM
