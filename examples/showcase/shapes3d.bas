' Six mathematical shapes spinning in 3D: an infinity sign, a trefoil knot, a
' (3,7) torus knot, a Moebius strip, a Klein bottle and a seashell. Two
' renderers draw the same meshes:
'   GPU  - the high-level 3D API (_MESH*, _CAMERA, _LIGHT, _DRAW3D) on wgpu
'   TRI  - _MAPTRIANGLE 3D with a depth buffer, transformed and lit in BASIC
' Keys: 1-6 or N = shape, M = renderer, A = automatic tour, ESC = quit.
' Command line: SHAPE=k starts with shape k, TRI with the _MAPTRIANGLE renderer,
' FRAMES=n stops after n frames (for tests).
CONST PI = 3.14159265
CONST SHAPES = 6
CONST MAXV = 50000, MAXT = 90000, MAXC = 700
SCREEN _NEWIMAGE(800, 600, 32)
_TITLE "QBNex 3D shapes"

' all meshes in one set of arrays: shape s uses vertices vs(s).. and triangles ts(s)..
DIM SHARED px(MAXV) AS SINGLE, py(MAXV) AS SINGLE, pz(MAXV) AS SINGLE
DIM SHARED nx(MAXV) AS SINGLE, ny(MAXV) AS SINGLE, nz(MAXV) AS SINGLE, tu(MAXV) AS SINGLE
DIM SHARED rx(MAXV) AS SINGLE, ry(MAXV) AS SINGLE, rz(MAXV) AS SINGLE, lit(MAXV) AS SINGLE
DIM SHARED ia(MAXT) AS LONG, ib(MAXT) AS LONG, ic(MAXT) AS LONG
DIM SHARED vs(SHAPES) AS LONG, vn(SHAPES) AS LONG, ts(SHAPES) AS LONG, tn(SHAPES) AS LONG
DIM SHARED shapename$(SHAPES), mesh&(SHAPES)
DIM SHARED nv AS LONG, nt AS LONG
' a tube's centre line and its frame
DIM SHARED cx(MAXC) AS SINGLE, cy(MAXC) AS SINGLE, cz(MAXC) AS SINGLE
DIM SHARED tx(MAXC) AS SINGLE, ty(MAXC) AS SINGLE, tz(MAXC) AS SINGLE
DIM SHARED qx(MAXC) AS SINGLE, qy(MAXC) AS SINGLE, qz(MAXC) AS SINGLE
DIM SHARED tex&

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

LOCATE 18, 30: PRINT "building the meshes..."
_DISPLAY
shapename$(1) = "Infinity (lemniscate tube)": buildtube 1, 160, 16, 0.36
shapename$(2) = "Trefoil knot": buildtube 2, 240, 16, 0.32
shapename$(3) = "Torus knot (3, 7)": buildtube 3, 600, 10, 0.16
shapename$(4) = "Moebius strip": buildsurface 4, 160, 12
shapename$(5) = "Klein bottle (figure-8)": buildsurface 5, 140, 48
shapename$(6) = "Seashell": buildsurface 6, 200, 24

' the same meshes for the GPU renderer, plus a few orbiting cubes
gpu% = _GPU3D
FOR s = 1 TO SHAPES
    _MESHBEGIN
    FOR v = vs(s) TO vs(s) + vn(s) - 1
        _MESHVERTEX px(v), py(v), pz(v), tu(v), 0.995
    NEXT
    FOR k = ts(s) TO ts(s) + tn(s) - 1
        _MESHTRIANGLE ia(k) - vs(s), ib(k) - vs(s), ic(k) - vs(s)
    NEXT
    mesh&(s) = _MESHEND
    _MESHTEXTURE mesh&(s), tex&
NEXT
cube& = _MESHCUBE(0.3)
_MESHCOLOR cube&, _RGB32(255, 230, 160)
_CAMERA (0, 0, 7) TO (0, 0, 0), 50
_AMBIENT _RGB32(255, 255, 255), 0.25
sun& = _LIGHT(1, -0.4, -0.8, -1, _RGB32(255, 250, 240), 0.9)
lamp& = _LIGHT(2, 0, 2.5, 2, _RGB32(120, 170, 255), 1.5, 9)

cmd$ = UCASE$(COMMAND$)
p = INSTR(cmd$, "FRAMES="): IF p THEN maxframes& = VAL(MID$(cmd$, p + 7))
shape% = 1: p = INSTR(cmd$, "SHAPE="): IF p THEN shape% = VAL(MID$(cmd$, p + 6))
IF shape% < 1 OR shape% > SHAPES THEN shape% = 1
mode% = 0: IF gpu% = 0 OR INSTR(cmd$, "TRI") > 0 THEN mode% = 1
auto% = -1: IF p THEN auto% = 0
_PRINTMODE _KEEPBACKGROUND
_DEPTHBUFFER ON
t0# = TIMER: frames& = 0: fps! = 0: shown# = TIMER

DO
    _LIMIT 60
    ang! = ang! + 1
    k$ = UCASE$(INKEY$)
    IF k$ = CHR$(27) THEN EXIT DO
    IF k$ = "M" THEN mode% = 1 - mode%
    IF k$ = "A" THEN auto% = NOT auto%: shown# = TIMER
    IF k$ = "N" THEN shape% = shape% MOD SHAPES + 1: shown# = TIMER
    IF k$ >= "1" AND k$ <= CHR$(48 + SHAPES) THEN shape% = VAL(k$): auto% = 0
    IF auto% AND TIMER - shown# > 8 THEN shape% = shape% MOD SHAPES + 1: shown# = TIMER
    backdrop
    _CLEAR3D 0
    IF mode% = 0 THEN
        _DRAW3D mesh&(shape%), 0, 0, 0, ang! * 0.7, ang!, ang! * 0.3
        FOR c = 0 TO 5
            a! = (ang! * 1.5 + c * 60) * PI / 180
            _DRAW3D cube&, 3.4 * COS(a!), 0.6 * SIN(a! * 2), 3.4 * SIN(a!), ang! * 2, ang! * 3, 0
        NEXT
    ELSE
        drawtri shape%, ang! * 0.7, ang!, ang! * 0.3
    END IF
    hud shape%, mode%, gpu%, fps!, auto%
    _DISPLAY
    frames& = frames& + 1
    IF TIMER - t0# >= 1 THEN fps! = frames& / (TIMER - t0#): t0# = TIMER: total& = total& + frames&: frames& = 0
    IF maxframes& > 0 AND total& + frames& >= maxframes& THEN EXIT DO
LOOP
OPEN "shapes3d.log" FOR OUTPUT AS #1
PRINT #1, "gpu3d"; gpu%; "shape"; shape%; "mode"; mode%; "fps"; INT(fps!); "triangles"; tn(shape%)
CLOSE #1
SYSTEM

' ---- shapes -----------------------------------------------------------------

' centre line of tube `kind` at t (0 .. 2 pi)
SUB curve (kind, t, x, y, z)
    SELECT CASE kind
        CASE 1 ' lemniscate of Bernoulli, lifted so the lobes pass over each other
            d = 1 + SIN(t) ^ 2
            x = 2.3 * COS(t) / d: y = 2.3 * SIN(t) * COS(t) / d: z = 0.5 * SIN(t)
        CASE 2 ' trefoil knot
            x = 0.75 * (SIN(t) + 2 * SIN(2 * t)): y = 0.75 * (COS(t) - 2 * COS(2 * t)): z = -0.75 * SIN(3 * t)
        CASE 3 ' (3, 7) torus knot
            r = 1.9 + 0.7 * COS(7 * t)
            x = r * COS(3 * t): y = r * SIN(3 * t): z = 0.7 * SIN(7 * t)
    END SELECT
END SUB

' surface `kind` at (u, v), both 0 .. 1
SUB surface (kind, u, v, x, y, z)
    SELECT CASE kind
        CASE 4 ' Moebius strip
            a = u * 2 * PI: w = (v - 0.5) * 1.8
            r = 2 + w * COS(a / 2)
            x = r * COS(a): y = r * SIN(a): z = w * SIN(a / 2)
        CASE 5 ' Klein bottle, figure-8 immersion
            a = u * 2 * PI: b = v * 2 * PI
            r = 2 + COS(a / 2) * SIN(b) - SIN(a / 2) * SIN(2 * b)
            x = 0.85 * r * COS(a): y = 0.85 * r * SIN(a): z = 0.85 * (SIN(a / 2) * SIN(b) + COS(a / 2) * SIN(2 * b))
        CASE 6 ' seashell: a shrinking tube along a rising spiral
            k = 1 - 0.85 * u: a = u * 3 * 2 * PI: b = v * 2 * PI
            r = k * (1.15 + COS(b))
            x = r * COS(a): y = 3.2 * u - 1.8 + k * SIN(b): z = r * SIN(a)
    END SELECT
END SUB

' a closed tube of radius rad around curve `kind`, its frame carried along the
' curve without twisting (and the leftover twist spread over the loop)
SUB buildtube (kind, segs, ring, rad)
    vs(kind) = nv: ts(kind) = nt
    FOR i = 0 TO segs
        t = i / segs * 2 * PI
        curve kind, t, cx(i), cy(i), cz(i)
        curve kind, t + 0.0005, ax, ay, az
        curve kind, t - 0.0005, bx, by, bz
        dx = ax - bx: dy = ay - by: dz = az - bz: l = SQR(dx * dx + dy * dy + dz * dz)
        tx(i) = dx / l: ty(i) = dy / l: tz(i) = dz / l
    NEXT
    ' first normal: perpendicular to the tangent
    ux = -ty(0): uy = tx(0): uz = 0
    IF ABS(tz(0)) > 0.9 THEN ux = 0: uy = -tz(0): uz = ty(0)
    l = SQR(ux * ux + uy * uy + uz * uz): qx(0) = ux / l: qy(0) = uy / l: qz(0) = uz / l
    FOR i = 1 TO segs
        d = qx(i - 1) * tx(i) + qy(i - 1) * ty(i) + qz(i - 1) * tz(i)
        ux = qx(i - 1) - d * tx(i): uy = qy(i - 1) - d * ty(i): uz = qz(i - 1) - d * tz(i)
        l = SQR(ux * ux + uy * uy + uz * uz): qx(i) = ux / l: qy(i) = uy / l: qz(i) = uz / l
    NEXT
    ' twist between the last frame and the first
    bx = ty(0) * qz(0) - tz(0) * qy(0): by = tz(0) * qx(0) - tx(0) * qz(0): bz = tx(0) * qy(0) - ty(0) * qx(0)
    twist = _ATAN2(qx(segs) * bx + qy(segs) * by + qz(segs) * bz, qx(segs) * qx(0) + qy(segs) * qy(0) + qz(segs) * qz(0))
    FOR i = 0 TO segs
        ' turn the normal back by its share of the twist, around the tangent
        a = -twist * i / segs
        bx = ty(i) * qz(i) - tz(i) * qy(i): by = tz(i) * qx(i) - tx(i) * qz(i): bz = tx(i) * qy(i) - ty(i) * qx(i)
        mx = qx(i) * COS(a) + bx * SIN(a): my = qy(i) * COS(a) + by * SIN(a): mz = qz(i) * COS(a) + bz * SIN(a)
        bx = ty(i) * mz - tz(i) * my: by = tz(i) * mx - tx(i) * mz: bz = tx(i) * my - ty(i) * mx
        FOR j = 0 TO ring
            phi = j / ring * 2 * PI
            dx = COS(phi) * mx + SIN(phi) * bx: dy = COS(phi) * my + SIN(phi) * by: dz = COS(phi) * mz + SIN(phi) * bz
            vertex cx(i) + rad * dx, cy(i) + rad * dy, cz(i) + rad * dz, dx, dy, dz, i / segs
        NEXT
    NEXT
    grid vs(kind), segs, ring, 0
    vn(kind) = nv - vs(kind): tn(kind) = nt - ts(kind)
END SUB

' an open surface, both sides: the front lifted along the normal, the back
' below it, wound the other way
SUB buildsurface (kind, nu, nvv)
    vs(kind) = nv: ts(kind) = nt
    e = 0.0005
    FOR side = 0 TO 1
        first = nv
        FOR i = 0 TO nu
            FOR j = 0 TO nvv
                u = i / nu: v = j / nvv
                surface kind, u, v, x, y, z
                surface kind, u + e, v, ax, ay, az
                surface kind, u - e, v, bx, by, bz
                surface kind, u, v + e, cx2, cy2, cz2
                surface kind, u, v - e, dx2, dy2, dz2
                ux = ax - bx: uy = ay - by: uz = az - bz
                wx = cx2 - dx2: wy = cy2 - dy2: wz = cz2 - dz2
                mx = uy * wz - uz * wy: my = uz * wx - ux * wz: mz = ux * wy - uy * wx
                l = SQR(mx * mx + my * my + mz * mz): IF l < 1E-12 THEN l = 1
                mx = mx / l: my = my / l: mz = mz / l
                IF side = 1 THEN mx = -mx: my = -my: mz = -mz
                vertex x + 0.012 * mx, y + 0.012 * my, z + 0.012 * mz, mx, my, mz, u
            NEXT
        NEXT
        grid first, nu, nvv, side
    NEXT
    vn(kind) = nv - vs(kind): tn(kind) = nt - ts(kind)
END SUB

SUB vertex (x, y, z, mx, my, mz, u)
    px(nv) = x: py(nv) = y: pz(nv) = z
    nx(nv) = mx: ny(nv) = my: nz(nv) = mz: tu(nv) = u
    nv = nv + 1
END SUB

' two triangles per cell of a (nu + 1) x (nvv + 1) vertex grid starting at first
SUB grid (first AS LONG, nu, nvv, flip)
    FOR i = 0 TO nu - 1
        FOR j = 0 TO nvv - 1
            a& = first + i * (nvv + 1) + j: b& = a& + nvv + 1
            IF flip THEN
                ia(nt) = a&: ib(nt) = a& + 1: ic(nt) = b&
                ia(nt + 1) = a& + 1: ib(nt + 1) = b& + 1: ic(nt + 1) = b&
            ELSE
                ia(nt) = a&: ib(nt) = b&: ic(nt) = a& + 1
                ia(nt + 1) = a& + 1: ib(nt + 1) = b&: ic(nt + 1) = b& + 1
            END IF
            nt = nt + 2
        NEXT
    NEXT
END SUB

' ---- drawing ----------------------------------------------------------------

SUB backdrop
    FOR y = 0 TO 599 STEP 4
        c = 10 + y / 30
        LINE (0, y)-(799, y + 3), _RGB32(c, c, c * 2 + 10), BF
    NEXT
END SUB

SUB hud (shape%, mode%, gpu%, fps!, auto%)
    COLOR _RGB32(255, 255, 255)
    _PRINTSTRING (16, 12), STR$(shape%) + ". " + shapename$(shape%)
    COLOR _RGB32(200, 210, 240)
    IF mode% = 0 THEN m$ = "GPU  (_DRAW3D on wgpu)" ELSE m$ = "TRI  (_MAPTRIANGLE 3D)"
    _PRINTSTRING (16, 30), "Renderer: " + m$
    _PRINTSTRING (16, 48), "FPS:" + STR$(INT(fps!)) + "   triangles:" + STR$(tn(shape%)) + "   _GPU3D:" + STR$(gpu%)
    COLOR _RGB32(150, 160, 190)
    a$ = "": IF auto% THEN a$ = " (tour on)"
    _PRINTSTRING (16, 576), "1-6/N = shape   M = renderer   A = tour" + a$ + "   ESC = quit"
    IF gpu% = 0 THEN COLOR _RGB32(255, 180, 120): _PRINTSTRING (560, 12), "no GPU adapter: TRI only"
END SUB

' _MAPTRIANGLE 3D: rotate as _DRAW3D, light, and move away from the eye. Its
' projection has a 67.5 degree vertical field of view here (90 on the shorter
' side), against 50 for _CAMERA at distance 7, so 4.9 gives the same size.
SUB drawtri (s, axd!, ayd!, azd!)
    sx = SIN(axd! * PI / 180): cxa = COS(axd! * PI / 180)
    sy = SIN(ayd! * PI / 180): cya = COS(ayd! * PI / 180)
    sz = SIN(azd! * PI / 180): cza = COS(azd! * PI / 180)
    ' light from the viewer's upper left
    lx = 0.4: ly = 0.6: lz = 0.7
    FOR v = vs(s) TO vs(s) + vn(s) - 1
        rotate px(v), py(v), pz(v), sx, cxa, sy, cya, sz, cza, x, y, z
        rx(v) = x: ry(v) = y: rz(v) = z - 4.9
        rotate nx(v), ny(v), nz(v), sx, cxa, sy, cya, sz, cza, x, y, z
        d = x * lx + y * ly + z * lz: IF d < 0 THEN d = 0
        lit(v) = 0.25 + 0.75 * d
    NEXT
    _DEPTHBUFFER _CLEAR
    FOR k = ts(s) TO ts(s) + tn(s) - 1
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
