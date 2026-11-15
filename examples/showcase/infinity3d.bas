' A rotating infinity sign: a rainbow tube along a figure-eight curve, spinning
' forever. Two renderers draw the same mesh:
'   GPU  - the high-level 3D API (_MESH*, _CAMERA, _LIGHT, _DRAW3D) on wgpu
'   TRI  - _MAPTRIANGLE 3D with a depth buffer, transformed and lit in BASIC
' M switches renderer, ESC quits. Command line: FRAMES=n stops after n frames,
' TRI starts with the _MAPTRIANGLE renderer.
CONST PI = 3.14159265
CONST SEGS = 160, RINGS = 12
CONST NV = (SEGS + 1) * (RINGS + 1), NT = SEGS * RINGS * 2
SCREEN _NEWIMAGE(800, 600, 32)
_TITLE "QBNex 3D: infinity"

DIM px(NV - 1) AS SINGLE, py(NV - 1) AS SINGLE, pz(NV - 1) AS SINGLE
DIM nx(NV - 1) AS SINGLE, ny(NV - 1) AS SINGLE, nz(NV - 1) AS SINGLE, tu(NV - 1) AS SINGLE
DIM rx(NV - 1) AS SINGLE, ry(NV - 1) AS SINGLE, rz(NV - 1) AS SINGLE, lit(NV - 1) AS SINGLE
DIM ia(NT - 1) AS LONG, ib(NT - 1) AS LONG, ic(NT - 1) AS LONG

' texture: hue along x with darker stripes, brightness from row 0 (dark) to 63 (full)
tex& = _NEWIMAGE(256, 64, 32)
_DEST tex&
FOR x = 0 TO 255
    h = x / 256
    r = 0.5 + 0.5 * COS(2 * PI * h): g = 0.5 + 0.5 * COS(2 * PI * (h - 1 / 3)): b = 0.5 + 0.5 * COS(2 * PI * (h - 2 / 3))
    stripe = 1: IF (x \ 8) MOD 2 = 1 THEN stripe = 0.8
    FOR y = 0 TO 63
        k = (y + 1) / 64 * stripe * 255
        PSET (x, y), _RGB32(r * k, g * k, b * k)
    NEXT
NEXT
_DEST 0

' the curve: a lemniscate of Bernoulli lifted in z so the two lobes pass over each other
FOR i = 0 TO SEGS
    t = i / SEGS * 2 * PI
    curve t, cx, cy, cz
    curve t + 0.001, ax, ay, az
    tx = ax - cx: ty = ay - cy: tz = az - cz
    l = SQR(tx * tx + ty * ty + tz * tz): tx = tx / l: ty = ty / l: tz = tz / l
    ' binormal = tangent x (0, 0, 1), normal = binormal x tangent
    bx = ty: by = -tx: bz = 0
    l = SQR(bx * bx + by * by): bx = bx / l: by = by / l
    qx = by * tz - bz * ty: qy = bz * tx - bx * tz: qz = bx * ty - by * tx
    FOR j = 0 TO RINGS
        phi = j / RINGS * 2 * PI
        dx = COS(phi) * qx + SIN(phi) * bx: dy = COS(phi) * qy + SIN(phi) * by: dz = COS(phi) * qz + SIN(phi) * bz
        v = i * (RINGS + 1) + j
        px(v) = cx + 0.36 * dx: py(v) = cy + 0.36 * dy: pz(v) = cz + 0.36 * dz
        nx(v) = dx: ny(v) = dy: nz(v) = dz
        tu(v) = i / SEGS
    NEXT
NEXT
n = 0
FOR i = 0 TO SEGS - 1
    FOR j = 0 TO RINGS - 1
        a = i * (RINGS + 1) + j: b = a + RINGS + 1
        ia(n) = a: ib(n) = b: ic(n) = a + 1: n = n + 1
        ia(n) = a + 1: ib(n) = b: ic(n) = b + 1: n = n + 1
    NEXT
NEXT

' the same mesh for the GPU renderer, plus a few orbiting cubes
gpu% = _GPU3D
_MESHBEGIN
FOR v = 0 TO NV - 1
    _MESHVERTEX px(v), py(v), pz(v), tu(v), 0.995
NEXT
FOR k = 0 TO NT - 1
    _MESHTRIANGLE ia(k), ib(k), ic(k)
NEXT
inf& = _MESHEND
_MESHTEXTURE inf&, tex&
cube& = _MESHCUBE(0.3)
_MESHCOLOR cube&, _RGB32(255, 230, 160)
_CAMERA (0, 0, 7) TO (0, 0, 0), 50
_AMBIENT _RGB32(255, 255, 255), 0.25
sun& = _LIGHT(1, -0.4, -0.8, -1, _RGB32(255, 250, 240), 0.9)
lamp& = _LIGHT(2, 0, 2.5, 2, _RGB32(120, 170, 255), 1.5, 9)

cmd$ = UCASE$(COMMAND$)
p = INSTR(cmd$, "FRAMES="): IF p THEN maxframes& = VAL(MID$(cmd$, p + 7))
mode% = 0: IF gpu% = 0 OR INSTR(cmd$, "TRI") > 0 THEN mode% = 1
_PRINTMODE _KEEPBACKGROUND
_DEPTHBUFFER ON
t0# = TIMER: frames& = 0: fps! = 0

DO
    _LIMIT 60
    ang! = ang! + 1
    k$ = INKEY$
    IF k$ = CHR$(27) THEN EXIT DO
    IF UCASE$(k$) = "M" THEN mode% = 1 - mode%
    backdrop
    IF mode% = 0 THEN
        _CLEAR3D 0
        _DRAW3D inf&, 0, 0, 0, ang! * 0.7, ang!, ang! * 0.3
        FOR c = 0 TO 5
            a! = (ang! * 1.5 + c * 60) * PI / 180
            _DRAW3D cube&, 3.2 * COS(a!), 0.6 * SIN(a! * 2), 3.2 * SIN(a!), ang! * 2, ang! * 3, 0
        NEXT
    ELSE
        _CLEAR3D 0
        drawtri ang! * 0.7, ang!, ang! * 0.3
    END IF
    hud
    _DISPLAY
    frames& = frames& + 1
    IF TIMER - t0# >= 1 THEN fps! = frames& / (TIMER - t0#): t0# = TIMER: total& = total& + frames&: frames& = 0
    IF maxframes& > 0 AND total& + frames& >= maxframes& THEN EXIT DO
LOOP
OPEN "infinity3d.log" FOR OUTPUT AS #1
PRINT #1, "gpu3d"; gpu%; "mode"; mode%; "fps"; INT(fps!)
CLOSE #1
SYSTEM

SUB curve (t, x, y, z)
    d = 1 + SIN(t) ^ 2
    x = 2.3 * COS(t) / d
    y = 2.3 * SIN(t) * COS(t) / d
    z = 0.5 * SIN(t)
END SUB

SUB backdrop
    FOR y = 0 TO 599 STEP 4
        c = 10 + y / 30
        LINE (0, y)-(799, y + 3), _RGB32(c, c, c * 2 + 10), BF
    NEXT
END SUB

SUB hud
    SHARED mode%, gpu%, fps!
    COLOR _RGB32(230, 235, 255)
    IF mode% = 0 THEN m$ = "GPU  (_DRAW3D on wgpu)" ELSE m$ = "TRI  (_MAPTRIANGLE 3D)"
    _PRINTSTRING (16, 12), "Renderer: " + m$
    _PRINTSTRING (16, 30), "FPS:" + STR$(INT(fps!)) + "   triangles:" + STR$(NT) + "   _GPU3D:" + STR$(gpu%)
    COLOR _RGB32(150, 160, 190)
    _PRINTSTRING (16, 576), "M = switch renderer   ESC = quit"
    IF gpu% = 0 THEN COLOR _RGB32(255, 180, 120): _PRINTSTRING (420, 576), "no GPU adapter: TRI only"
END SUB

' _MAPTRIANGLE 3D: rotate as _DRAW3D, light, and move away from the eye. Its projection has a
' 67.5 degree vertical field of view here (90 on the shorter side), against 50 for _CAMERA at
' distance 7, so 4.9 gives the same size on screen.
SUB drawtri (axd!, ayd!, azd!)
    SHARED px() AS SINGLE, py() AS SINGLE, pz() AS SINGLE, nx() AS SINGLE, ny() AS SINGLE, nz() AS SINGLE
    SHARED tu() AS SINGLE, rx() AS SINGLE, ry() AS SINGLE, rz() AS SINGLE, lit() AS SINGLE
    SHARED ia() AS LONG, ib() AS LONG, ic() AS LONG, tex&
    sx = SIN(axd! * PI / 180): cxa = COS(axd! * PI / 180)
    sy = SIN(ayd! * PI / 180): cya = COS(ayd! * PI / 180)
    sz = SIN(azd! * PI / 180): cza = COS(azd! * PI / 180)
    ' light from the viewer's upper left
    lx = 0.4: ly = 0.6: lz = 0.7
    FOR v = 0 TO NV - 1
        rotate px(v), py(v), pz(v), sx, cxa, sy, cya, sz, cza, x, y, z
        rx(v) = x: ry(v) = y: rz(v) = z - 4.9
        rotate nx(v), ny(v), nz(v), sx, cxa, sy, cya, sz, cza, x, y, z
        d = x * lx + y * ly + z * lz: IF d < 0 THEN d = 0
        lit(v) = 0.25 + 0.75 * d
    NEXT
    _DEPTHBUFFER _CLEAR
    FOR k = 0 TO NT - 1
        a = ia(k): b = ib(k): c = ic(k)
        row = INT((lit(a) + lit(b) + lit(c)) / 3 * 63)
        _MAPTRIANGLE (tu(a) * 255, row)-(tu(b) * 255, row)-(tu(c) * 255, row), tex& TO(rx(a), ry(a), rz(a))-(rx(b), ry(b), rz(b))-(rx(c), ry(c), rz(c))
    NEXT
END SUB

' the rotation of _DRAW3D: Z first, then X, then Y
SUB rotate (x0, y0, z0, sx, cxa, sy, cya, sz, cza, x, y, z)
    x1 = x0 * cza - y0 * sz: y1 = x0 * sz + y0 * cza
    y = y1 * cxa - z0 * sx: z1 = y1 * sx + z0 * cxa
    x = x1 * cya + z1 * sy: z = -x1 * sy + z1 * cya
END SUB
