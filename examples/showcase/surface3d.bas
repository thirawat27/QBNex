' 3D surface plot of z = sin(r) / r: projection, a painter's-algorithm mesh,
' flat shading and a triangle rasteriser written in BASIC.
CONST SCRW = 640, SCRH = 400, N = 44
SCREEN _NEWIMAGE(SCRW, SCRH, 32)
_TITLE "QBNex 3D surface"
DIM SHARED sx(N, N), sy(N, N), wz(N, N), wx(N, N), wy(N, N)

' Background
FOR y = 0 TO SCRH - 1
    LINE (0, y)-(SCRW - 1, y), _RGB32(14 + y / 20, 18 + y / 16, 34 + y / 10)
NEXT

yaw = 0.7: pitch = 0.55
FOR i = 0 TO N
    FOR j = 0 TO N
        x = (i / N - 0.5) * 22: y = (j / N - 0.5) * 22
        r = SQR(x * x + y * y): IF r < 0.0001 THEN z = 1 ELSE z = SIN(r) / r
        wx(i, j) = x: wy(i, j) = y: wz(i, j) = z
        ' Rotate around the vertical axis, then tilt toward the camera
        rx = x * COS(yaw) - y * SIN(yaw): ry = x * SIN(yaw) + y * COS(yaw)
        rz = z * 9
        v = rz * COS(pitch) + ry * SIN(pitch): d = ry * COS(pitch) - rz * SIN(pitch) + 34
        sx(i, j) = SCRW / 2 + rx * 470 / d
        sy(i, j) = SCRH / 2 + 5 - v * 470 / d
    NEXT
NEXT

' Draw the quads from the back row to the front (far = larger rotated y)
DIM order(N * N) AS LONG, depth(N * N)
k = 0
FOR i = 0 TO N - 1
    FOR j = 0 TO N - 1
        cx = (wx(i, j) + wx(i + 1, j + 1)) / 2: cy = (wy(i, j) + wy(i + 1, j + 1)) / 2
        depth(k) = cx * SIN(yaw) + cy * COS(yaw): order(k) = i * N + j: k = k + 1
    NEXT
NEXT
' Shell sort by depth, far first
gap = k \ 2
DO WHILE gap > 0
    FOR a = gap TO k - 1
        b = a
        DO WHILE b >= gap
            IF depth(b - gap) >= depth(b) THEN EXIT DO
            SWAP depth(b - gap), depth(b): SWAP order(b - gap), order(b)
            b = b - gap
        LOOP
    NEXT
    gap = gap \ 2
LOOP

FOR q = 0 TO k - 1
    i = order(q) \ N: j = order(q) MOD N
    ' Surface normal from the two diagonals, lit from the upper left
    ax = wx(i + 1, j + 1) - wx(i, j): ay = wy(i + 1, j + 1) - wy(i, j): az = (wz(i + 1, j + 1) - wz(i, j)) * 9
    bx = wx(i, j + 1) - wx(i + 1, j): by = wy(i, j + 1) - wy(i + 1, j): bz = (wz(i, j + 1) - wz(i + 1, j)) * 9
    nx = ay * bz - az * by: ny = az * bx - ax * bz: nz = ax * by - ay * bx
    nl = SQR(nx * nx + ny * ny + nz * nz): IF nz < 0 THEN nl = -nl
    lit = (nx * -0.4 + ny * 0.5 + nz * 0.77) / nl: IF lit < 0 THEN lit = 0
    h = (wz(i, j) + 0.25) / 1.25 ' height 0..1 picks the colour
    r = 255 * h: g = 80 + 140 * (1 - ABS(h - 0.45) * 2): b = 255 * (1 - h)
    sh = 0.3 + 0.7 * lit
    col~& = _RGB32(r * sh, g * sh, b * sh)
    Tri sx(i, j), sy(i, j), sx(i + 1, j), sy(i + 1, j), sx(i + 1, j + 1), sy(i + 1, j + 1), col~&
    Tri sx(i, j), sy(i, j), sx(i + 1, j + 1), sy(i + 1, j + 1), sx(i, j + 1), sy(i, j + 1), col~&
    edge~& = _RGB32(r * sh * 0.6, g * sh * 0.6, b * sh * 0.6)
    LINE (sx(i, j), sy(i, j))-(sx(i + 1, j), sy(i + 1, j)), edge~&
    LINE (sx(i, j), sy(i, j))-(sx(i, j + 1), sy(i, j + 1)), edge~&
NEXT
COLOR _RGB32(220, 230, 255): _PRINTMODE _KEEPBACKGROUND
_PRINTSTRING (16, 14), "z = sin(r) / r"
SLEEP
SYSTEM

' Filled triangle by horizontal spans
SUB Tri (ax, ay, bx, by, qx, qy, c AS _UNSIGNED LONG)
    x1 = ax: y1 = ay: x2 = bx: y2 = by: x3 = qx: y3 = qy ' work on copies: the arguments are passed by reference
    IF y2 < y1 THEN SWAP y1, y2: SWAP x1, x2
    IF y3 < y1 THEN SWAP y1, y3: SWAP x1, x3
    IF y3 < y2 THEN SWAP y2, y3: SWAP x2, x3
    IF y3 - y1 < 0.5 THEN EXIT SUB
    FOR y = -INT(-y1) TO INT(y3)
        xa = x1 + (x3 - x1) * (y - y1) / (y3 - y1)
        IF y < y2 THEN
            IF y2 - y1 > 0 THEN xb = x1 + (x2 - x1) * (y - y1) / (y2 - y1) ELSE xb = x2
        ELSE
            IF y3 - y2 > 0 THEN xb = x2 + (x3 - x2) * (y - y2) / (y3 - y2) ELSE xb = x2
        END IF
        LINE (xa, y)-(xb, y), c
    NEXT
END SUB
