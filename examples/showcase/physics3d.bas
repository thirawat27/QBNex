' Physics in 3D: five animated scenes drawn with the high-level 3D API
' (_MESH*, _CAMERA, _LIGHT, _DRAW3D) and four small WGSL shaders (_SHADER).
'   1 Pendulum wave      16 pendulums whose periods drift apart and back into step
'   2 Orbits             planets and a moon moving under the Sun's gravity
'   3 Bouncing balls     40 balls with gravity, bounces and ball-to-ball collisions
'   4 Wave interference  two point sources rippling a field of 1,936 columns
'   5 Lorenz attractor   the chaotic Lorenz system traced as a growing tube
' Nothing is written on the picture: the scene name is in the window title.
' The physics advances 1/60 s per frame, so every run looks the same.
' Keys: 1-5 or N = scene, A = automatic tour, Esc = quit.
' Command line: SCENE=k starts with scene k, PAUSE=n holds the picture from
' frame n on (for screenshots), NOLIMIT drops the 60 fps cap.
CONST PI = 3.14159265
CONST SCENES = 5, HUES = 36
CONST SW = 960, SH = 600
CONST NP = 16 '                pendulums
CONST NPL = 6 '                planets
CONST TRAIL = 60 '             trail dots per planet
CONST NBALL = 40, HALF = 4 '   balls, half the width of the arena
CONST GRID = 44 '              wave columns per side
CONST LSTEPS = 6000, CHUNKS = 60, RING = 8 ' Lorenz steps, tube pieces, sides
CONST NSTAR = 260

SCREEN _NEWIMAGE(SW, SH, 32)
_TITLE "QBNex 3D physics"
RANDOMIZE 7
IF _GPU3D = 0 THEN
    COLOR _RGB32(255, 200, 140)
    _PRINTSTRING (SW \ 2 - 160, SH \ 2), "No GPU adapter: the 3D scenes need one."
    SLEEP
    SYSTEM
END IF

DIM SHARED shiny&, glow&, ground&, soft&
DIM SHARED ball&(HUES - 1), water&(HUES - 1), shadow&(7)
DIM SHARED metal&, glass&, floorm&, sun&, halo&, ringm&, moon&, head&
DIM SHARED planet&(NPL - 1), tube&(CHUNKS - 1)
DIM SHARED sunlight&, filllight&, lamp&
DIM SHARED scenename$(SCENES), fr&, tsim
' Lorenz curve and the frame carried along it
DIM SHARED lx(LSTEPS), ly(LSTEPS), lz(LSTEPS)
DIM SHARED ltx(LSTEPS), lty(LSTEPS), ltz(LSTEPS), lqx(LSTEPS), lqy(LSTEPS), lqz(LSTEPS)
' planets: position, velocity, size, trail ring buffer
DIM SHARED px(NPL - 1), py(NPL - 1), pz(NPL - 1), pvx(NPL - 1), pvy(NPL - 1), pvz(NPL - 1), psize(NPL - 1)
DIM SHARED tailx(NPL - 1, TRAIL - 1), taily(NPL - 1, TRAIL - 1), tailz(NPL - 1, TRAIL - 1), tailn&
' balls
DIM SHARED bx(NBALL - 1), by(NBALL - 1), bz(NBALL - 1), bvx(NBALL - 1), bvy(NBALL - 1), bvz(NBALL - 1)
DIM SHARED brad(NBALL - 1), bmass(NBALL - 1), bhue(NBALL - 1) AS INTEGER
DIM SHARED starx(NSTAR - 1), stary(NSTAR - 1), starc(NSTAR - 1)

scenename$(1) = "Pendulum wave"
scenename$(2) = "Orbits"
scenename$(3) = "Bouncing balls"
scenename$(4) = "Wave interference"
scenename$(5) = "Lorenz attractor"

BuildShaders
BuildMeshes
BuildLorenz
FOR i = 0 TO NSTAR - 1
    starx(i) = RND * SW: stary(i) = RND * SH: starc(i) = 60 + RND * 190
NEXT
sunlight& = _LIGHT(1, -0.45, -1, -0.55, _RGB32(255, 248, 235), 0.95)
filllight& = _LIGHT(1, 0.6, -0.3, 0.8, _RGB32(150, 180, 255), 0.25)
lamp& = _LIGHT(2, 0, 0, 0, _RGB32(255, 225, 180), 1.7, 16)

cmd$ = UCASE$(COMMAND$)
scene% = 1
p = INSTR(cmd$, "SCENE="): IF p THEN scene% = VAL(MID$(cmd$, p + 6))
IF scene% < 1 OR scene% > SCENES THEN scene% = 1
pause& = -1
q = INSTR(cmd$, "PAUSE="): IF q THEN pause& = VAL(MID$(cmd$, q + 6))
tour% = -1: IF p OR q THEN tour% = 0
nolimit% = INSTR(cmd$, "NOLIMIT") > 0

StartScene scene%
DO
    IF nolimit% = 0 THEN _LIMIT 60
    k$ = UCASE$(INKEY$)
    IF k$ = CHR$(27) THEN EXIT DO
    IF k$ = "A" THEN tour% = NOT tour%
    IF k$ = "N" THEN scene% = scene% MOD SCENES + 1: StartScene scene%
    IF k$ >= "1" AND k$ <= CHR$(48 + SCENES) THEN scene% = VAL(k$): tour% = 0: StartScene scene%
    IF tour% AND fr& >= 60 * 14 THEN scene% = scene% MOD SCENES + 1: StartScene scene%
    IF pause& < 0 OR fr& < pause& THEN
        tsim = fr& / 60
        Advance scene%
        fr& = fr& + 1
    END IF
    Backdrop scene%
    _CLEAR3D 0
    SELECT CASE scene%
        CASE 1: DrawPendulums
        CASE 2: DrawOrbits
        CASE 3: DrawBalls
        CASE 4: DrawWaves
        CASE 5: DrawLorenz
    END SELECT
    _DISPLAY
LOOP
SYSTEM

' planets: size, colour
planetdata:
DATA 0.16,170,160,150
DATA 0.26,235,190,120
DATA 0.28,70,140,235
DATA 0.22,220,110,70
DATA 0.62,215,170,120
DATA 0.48,230,210,160

' ---- set-up ------------------------------------------------------------------

SUB BuildShaders
    nl$ = CHR$(10)
    ' Blinn-Phong highlight and a rim light on top of the normal lighting.
    ' params: 0 = eye (w: rim), 1 = light direction (w: highlight), 2 = point light (w > 0.5: use it)
    s$ = "fn shade(i: ShadeIn) -> vec4<f32> {" + nl$
    s$ = s$ + "    let v = normalize(i.params[0].xyz - i.world);" + nl$
    s$ = s$ + "    var l = normalize(-i.params[1].xyz);" + nl$
    s$ = s$ + "    if (i.params[2].w > 0.5) { l = normalize(i.params[2].xyz - i.world); }" + nl$
    s$ = s$ + "    let h = normalize(l + v);" + nl$
    s$ = s$ + "    let lit = select(0.0, 1.0, dot(i.normal, l) > 0.0);" + nl$
    s$ = s$ + "    let spec = pow(max(dot(i.normal, h), 0.0), 60.0) * i.params[1].w * lit;" + nl$
    s$ = s$ + "    let rim = pow(1.0 - max(dot(i.normal, v), 0.0), 3.0) * i.params[0].w;" + nl$
    s$ = s$ + "    return vec4<f32>(i.base.rgb * (i.light + vec3<f32>(rim)) + vec3<f32>(spec), i.base.a);" + nl$
    s$ = s$ + "}"
    shiny& = MakeShader(s$)
    ' Glowing: unlit, brighter where the surface faces the eye; a transparent
    ' mesh fades out toward its outline (the Sun's corona).
    s$ = "fn shade(i: ShadeIn) -> vec4<f32> {" + nl$
    s$ = s$ + "    let k = max(dot(i.normal, normalize(i.params[0].xyz - i.world)), 0.0);" + nl$
    s$ = s$ + "    if (i.base.a < 0.99) { return vec4<f32>(i.base.rgb, i.base.a * k * k * k); }" + nl$
    s$ = s$ + "    return vec4<f32>(i.base.rgb * (0.7 + 0.5 * k), 1.0);" + nl$
    s$ = s$ + "}"
    glow& = MakeShader(s$)
    ' Checkered ground fading out at the edge. params 3: square size, radius, fade width
    s$ = "fn shade(i: ShadeIn) -> vec4<f32> {" + nl$
    s$ = s$ + "    let s = i.params[3].x;" + nl$
    s$ = s$ + "    let c = floor(i.world.x / s) + floor(i.world.z / s);" + nl$
    s$ = s$ + "    let k = mix(1.0, 0.8, c - 2.0 * floor(c * 0.5));" + nl$
    s$ = s$ + "    let fade = clamp((i.params[3].y - length(i.world.xz)) / i.params[3].z, 0.0, 1.0);" + nl$
    s$ = s$ + "    return vec4<f32>(i.base.rgb * k * i.light, fade);" + nl$
    s$ = s$ + "}"
    ground& = MakeShader(s$)
    ' Soft round shadow: u runs from 0 at the centre to 1 at the rim.
    s$ = "fn shade(i: ShadeIn) -> vec4<f32> {" + nl$
    s$ = s$ + "    let d = clamp(i.uv.x, 0.0, 1.0);" + nl$
    s$ = s$ + "    return vec4<f32>(0.0, 0.0, 0.0, i.base.a * (1.0 - d * d));" + nl$
    s$ = s$ + "}"
    soft& = MakeShader(s$)
END SUB

FUNCTION MakeShader& (src$)
    h& = _SHADER(src$)
    IF h& = -1 THEN
        _DEST 0: COLOR _RGB32(255, 160, 120)
        PRINT "shader error: "; _SHADERERROR$
        SLEEP
        SYSTEM
    END IF
    MakeShader& = h&
END FUNCTION

' A sphere of radius rr: one vertex at each pole and rings of sl vertices.
SUB SphereGeo (rr, sl, st)
    _MESHVERTEX 0, rr, 0
    FOR i = 1 TO st - 1
        th = i / st * PI
        FOR j = 0 TO sl - 1
            ph = j / sl * 2 * PI
            _MESHVERTEX rr * SIN(th) * COS(ph), rr * COS(th), rr * SIN(th) * SIN(ph)
        NEXT
    NEXT
    _MESHVERTEX 0, -rr, 0
    bottom& = 1 + (st - 1) * sl
    FOR j = 0 TO sl - 1
        j2 = (j + 1) MOD sl
        _MESHTRIANGLE 0, 1 + j2, 1 + j
        _MESHTRIANGLE bottom&, 1 + (st - 2) * sl + j, 1 + (st - 2) * sl + j2
    NEXT
    FOR i = 0 TO st - 3
        FOR j = 0 TO sl - 1
            j2 = (j + 1) MOD sl
            a& = 1 + i * sl + j: b& = 1 + i * sl + j2
            _MESHTRIANGLE a&, b&, a& + sl
            _MESHTRIANGLE b&, b& + sl, a& + sl
        NEXT
    NEXT
END SUB

FUNCTION Sphere& (rr, colour~&, sh&)
    _MESHBEGIN
    SphereGeo rr, 32, 20
    m& = _MESHEND
    _MESHCOLOR m&, colour~&
    _MESHSHADER m&, sh&
    Sphere& = m&
END FUNCTION

FUNCTION Hue~& (h)
    r = 0.5 + 0.5 * COS(2 * PI * h)
    g = 0.5 + 0.5 * COS(2 * PI * (h - 1 / 3))
    b = 0.5 + 0.5 * COS(2 * PI * (h - 2 / 3))
    Hue~& = _RGB32(30 + 225 * r, 30 + 225 * g, 30 + 225 * b)
END FUNCTION

SUB BuildMeshes
    FOR i = 0 TO HUES - 1
        ball&(i) = Sphere&(1, Hue~&(i / HUES), shiny&)
        ' water: deep blue, cyan, then white crests
        h = i / (HUES - 1)
        IF h < 0.6 THEN
            k = h / 0.6: c~& = _RGB32(12 + 30 * k, 40 + 150 * k, 110 + 120 * k)
        ELSE
            k = (h - 0.6) / 0.4: c~& = _RGB32(42 + 200 * k, 190 + 60 * k, 230 + 25 * k)
        END IF
        water&(i) = _MESHCUBE(1)
        _MESHCOLOR water&(i), c~&
        _MESHSHADER water&(i), shiny&
    NEXT
    metal& = _MESHCUBE(1)
    _MESHCOLOR metal&, _RGB32(170, 175, 190)
    _MESHSHADER metal&, shiny&
    glass& = _MESHCUBE(1)
    _MESHCOLOR glass&, _RGBA32(150, 200, 255, 60)
    floorm& = _MESHPLANE(80, 80)
    _MESHCOLOR floorm&, _RGB32(200, 205, 215)
    _MESHSHADER floorm&, ground&
    ' shadow discs, darker for balls close to the ground
    FOR k = 0 TO 7
        _MESHBEGIN
        _MESHVERTEX 0, 0, 0, 0, 0
        FOR j = 0 TO 31
            a = j / 32 * 2 * PI
            _MESHVERTEX COS(a), 0, SIN(a), 1, 0
        NEXT
        FOR j = 0 TO 31
            _MESHTRIANGLE 0, 1 + j, 1 + (j + 1) MOD 32
        NEXT
        shadow&(k) = _MESHEND
        _MESHCOLOR shadow&(k), _RGBA32(0, 0, 0, 150 - k * 17)
        _MESHSHADER shadow&(k), soft&
    NEXT
    ' the Sun, its corona, the planets, a moon and a ring
    sun& = Sphere&(1, _RGB32(255, 205, 90), glow&)
    halo& = Sphere&(1, _RGBA32(255, 160, 50, 150), glow&)
    head& = Sphere&(1, _RGB32(255, 250, 220), glow&)
    RESTORE planetdata
    FOR i = 0 TO NPL - 1
        READ psize(i), r, g, b
        planet&(i) = Sphere&(1, _RGB32(r, g, b), shiny&)
    NEXT
    moon& = Sphere&(1, _RGB32(200, 200, 205), shiny&)
    _MESHBEGIN
    FOR j = 0 TO 63
        a = j / 64 * 2 * PI
        _MESHVERTEX 1.4 * COS(a), 0, 1.4 * SIN(a)
        _MESHVERTEX 2.3 * COS(a), 0, 2.3 * SIN(a)
    NEXT
    FOR j = 0 TO 63
        j2 = (j + 1) MOD 64
        _MESHTRIANGLE 2 * j, 2 * j + 1, 2 * j2
        _MESHTRIANGLE 2 * j + 1, 2 * j2 + 1, 2 * j2
    NEXT
    ringm& = _MESHEND
    _MESHCOLOR ringm&, _RGBA32(225, 200, 150, 190)
END SUB

' The Lorenz system (sigma 10, rho 28, beta 8/3) by Runge-Kutta, then a frame
' carried along the curve without twisting, then the tube in CHUNKS pieces.
SUB BuildLorenz
    x = 1: y = 1: z = 1: dt = 0.006
    FOR i = 0 TO LSTEPS
        lx(i) = x * 0.13: ly(i) = (z - 24) * 0.13: lz(i) = y * 0.13
        LorenzD x, y, z, k1x, k1y, k1z
        LorenzD x + dt / 2 * k1x, y + dt / 2 * k1y, z + dt / 2 * k1z, k2x, k2y, k2z
        LorenzD x + dt / 2 * k2x, y + dt / 2 * k2y, z + dt / 2 * k2z, k3x, k3y, k3z
        LorenzD x + dt * k3x, y + dt * k3y, z + dt * k3z, k4x, k4y, k4z
        x = x + dt / 6 * (k1x + 2 * k2x + 2 * k3x + k4x)
        y = y + dt / 6 * (k1y + 2 * k2y + 2 * k3y + k4y)
        z = z + dt / 6 * (k1z + 2 * k2z + 2 * k3z + k4z)
    NEXT
    FOR i = 0 TO LSTEPS
        i0 = i - 1: IF i0 < 0 THEN i0 = 0
        i1 = i + 1: IF i1 > LSTEPS THEN i1 = LSTEPS
        dx = lx(i1) - lx(i0): dy = ly(i1) - ly(i0): dz = lz(i1) - lz(i0)
        l = SQR(dx * dx + dy * dy + dz * dz): IF l < 1E-9 THEN l = 1
        ltx(i) = dx / l: lty(i) = dy / l: ltz(i) = dz / l
    NEXT
    ux = -lty(0): uy = ltx(0): uz = 0
    l = SQR(ux * ux + uy * uy + uz * uz): lqx(0) = ux / l: lqy(0) = uy / l: lqz(0) = uz / l
    FOR i = 1 TO LSTEPS
        d = lqx(i - 1) * ltx(i) + lqy(i - 1) * lty(i) + lqz(i - 1) * ltz(i)
        ux = lqx(i - 1) - d * ltx(i): uy = lqy(i - 1) - d * lty(i): uz = lqz(i - 1) - d * ltz(i)
        l = SQR(ux * ux + uy * uy + uz * uz)
        IF l < 1E-9 THEN
            lqx(i) = lqx(i - 1): lqy(i) = lqy(i - 1): lqz(i) = lqz(i - 1)
        ELSE
            lqx(i) = ux / l: lqy(i) = uy / l: lqz(i) = uz / l
        END IF
    NEXT
    piece = LSTEPS \ CHUNKS: rad = 0.045
    FOR c = 0 TO CHUNKS - 1
        _MESHBEGIN
        FOR i = c * piece TO c * piece + piece
            bxx = lty(i) * lqz(i) - ltz(i) * lqy(i)
            byy = ltz(i) * lqx(i) - ltx(i) * lqz(i)
            bzz = ltx(i) * lqy(i) - lty(i) * lqx(i)
            FOR j = 0 TO RING - 1
                ph = j / RING * 2 * PI
                ddx = COS(ph) * lqx(i) + SIN(ph) * bxx
                ddy = COS(ph) * lqy(i) + SIN(ph) * byy
                ddz = COS(ph) * lqz(i) + SIN(ph) * bzz
                _MESHVERTEX lx(i) + rad * ddx, ly(i) + rad * ddy, lz(i) + rad * ddz
            NEXT
        NEXT
        FOR i = 0 TO piece - 1
            FOR j = 0 TO RING - 1
                j2 = (j + 1) MOD RING
                a& = i * RING + j: b& = i * RING + j2
                _MESHTRIANGLE a&, a& + RING, b&
                _MESHTRIANGLE b&, a& + RING, b& + RING
            NEXT
        NEXT
        tube&(c) = _MESHEND
        _MESHCOLOR tube&(c), Hue~&(0.62 + 0.75 * c / CHUNKS)
        _MESHSHADER tube&(c), shiny&
    NEXT
END SUB

SUB LorenzD (x, y, z, dx, dy, dz)
    dx = 10 * (y - x): dy = x * (28 - z) - y: dz = x * y - 8 / 3 * z
END SUB

' ---- scenes ------------------------------------------------------------------

SUB StartScene (s%)
    fr& = 0: tsim = 0
    _TITLE "QBNex 3D physics - " + scenename$(s%)
    _LIGHTON sunlight&, s% <> 2
    _LIGHTON filllight&, s% <> 2
    _LIGHTON lamp&, s% = 2
    IF s% = 2 THEN _AMBIENT _RGB32(255, 255, 255), 0.07 ELSE _AMBIENT _RGB32(255, 255, 255), 0.3
    ' highlight from the sunlight, or from the lamp in the Sun
    _SHADERPARAM shiny&, 4, -0.45: _SHADERPARAM shiny&, 5, -1: _SHADERPARAM shiny&, 6, -0.55
    _SHADERPARAM shiny&, 7, 0.55
    _SHADERPARAM shiny&, 11, -(s% = 2)
    _SHADERPARAM shiny&, 3, 0.25
    SELECT CASE s%
        CASE 2
            FOR i = 0 TO NPL - 1
                rr = 2.2 + 1.2 * i
                a = i * 2.1
                v = SQR(40 / rr) * (1 + 0.05 * (i MOD 3))
                px(i) = rr * COS(a): py(i) = 0: pz(i) = rr * SIN(a)
                pvx(i) = -v * SIN(a): pvz(i) = v * COS(a): pvy(i) = v * 0.04 * (i - 2.5)
                FOR j = 0 TO TRAIL - 1
                    tailx(i, j) = px(i): taily(i, j) = py(i): tailz(i, j) = pz(i)
                NEXT
            NEXT
            tailn& = 0
        CASE 3
            FOR i = 0 TO NBALL - 1
                brad(i) = 0.17 + RND * 0.25: bmass(i) = brad(i) ^ 3
                bx(i) = (RND * 2 - 1) * (HALF - 0.6): bz(i) = (RND * 2 - 1) * (HALF - 0.6)
                by(i) = 1.5 + i * 0.16
                bvx(i) = RND * 2 - 1: bvy(i) = 0: bvz(i) = RND * 2 - 1
                bhue(i) = INT(RND * HUES)
            NEXT
        CASE 4
            _SHADERPARAM shiny&, 7, 0.9
    END SELECT
END SUB

SUB Advance (s%)
    SELECT CASE s%
        CASE 2 ' gravity of the Sun (GM = 40), velocity Verlet in four substeps
            h = 1 / 240
            FOR st = 1 TO 4
                FOR i = 0 TO NPL - 1
                    Gravity px(i), py(i), pz(i), ax, ay, az
                    pvx(i) = pvx(i) + ax * h / 2: pvy(i) = pvy(i) + ay * h / 2: pvz(i) = pvz(i) + az * h / 2
                    px(i) = px(i) + pvx(i) * h: py(i) = py(i) + pvy(i) * h: pz(i) = pz(i) + pvz(i) * h
                    Gravity px(i), py(i), pz(i), ax, ay, az
                    pvx(i) = pvx(i) + ax * h / 2: pvy(i) = pvy(i) + ay * h / 2: pvz(i) = pvz(i) + az * h / 2
                NEXT
            NEXT
            IF fr& MOD 3 = 0 THEN
                tailn& = (tailn& + 1) MOD TRAIL
                FOR i = 0 TO NPL - 1
                    tailx(i, tailn&) = px(i): taily(i, tailn&) = py(i): tailz(i, tailn&) = pz(i)
                NEXT
            END IF
        CASE 3
            ' every five seconds the resting balls are thrown up again
            IF fr& MOD 300 = 299 THEN
                FOR i = 0 TO NBALL - 1
                    IF by(i) < brad(i) + 0.1 THEN
                        bvy(i) = 5 + RND * 4: bvx(i) = bvx(i) + RND * 4 - 2: bvz(i) = bvz(i) + RND * 4 - 2
                    END IF
                NEXT
            END IF
            h = 1 / 240
            FOR st = 1 TO 4
                FOR i = 0 TO NBALL - 1
                    bvy(i) = bvy(i) - 9.8 * h
                    bx(i) = bx(i) + bvx(i) * h: by(i) = by(i) + bvy(i) * h: bz(i) = bz(i) + bvz(i) * h
                    IF by(i) < brad(i) THEN
                        by(i) = brad(i)
                        IF bvy(i) < 0 THEN bvy(i) = -bvy(i) * 0.8
                        bvx(i) = bvx(i) * 0.996: bvz(i) = bvz(i) * 0.996
                    END IF
                    w = HALF - brad(i)
                    IF bx(i) > w THEN bx(i) = w: bvx(i) = -ABS(bvx(i)) * 0.9
                    IF bx(i) < -w THEN bx(i) = -w: bvx(i) = ABS(bvx(i)) * 0.9
                    IF bz(i) > w THEN bz(i) = w: bvz(i) = -ABS(bvz(i)) * 0.9
                    IF bz(i) < -w THEN bz(i) = -w: bvz(i) = ABS(bvz(i)) * 0.9
                NEXT
                FOR i = 0 TO NBALL - 2
                    FOR j = i + 1 TO NBALL - 1
                        dx = bx(i) - bx(j): dy = by(i) - by(j): dz = bz(i) - bz(j)
                        rr = brad(i) + brad(j)
                        d2 = dx * dx + dy * dy + dz * dz
                        IF d2 < rr * rr AND d2 > 1E-8 THEN
                            d = SQR(d2): nx = dx / d: ny = dy / d: nz = dz / d
                            wi = 1 / bmass(i): wj = 1 / bmass(j)
                            push = (rr - d) / (wi + wj)
                            bx(i) = bx(i) + nx * push * wi: by(i) = by(i) + ny * push * wi: bz(i) = bz(i) + nz * push * wi
                            bx(j) = bx(j) - nx * push * wj: by(j) = by(j) - ny * push * wj: bz(j) = bz(j) - nz * push * wj
                            vn = (bvx(i) - bvx(j)) * nx + (bvy(i) - bvy(j)) * ny + (bvz(i) - bvz(j)) * nz
                            IF vn < 0 THEN
                                kick = -1.9 * vn / (wi + wj)
                                bvx(i) = bvx(i) + kick * wi * nx: bvy(i) = bvy(i) + kick * wi * ny: bvz(i) = bvz(i) + kick * wi * nz
                                bvx(j) = bvx(j) - kick * wj * nx: bvy(j) = bvy(j) - kick * wj * ny: bvz(j) = bvz(j) - kick * wj * nz
                            END IF
                        END IF
                    NEXT
                NEXT
            NEXT
    END SELECT
END SUB

SUB Gravity (x, y, z, ax, ay, az)
    r2 = x * x + y * y + z * z + 0.01
    f = -40 / (r2 * SQR(r2))
    ax = f * x: ay = f * y: az = f * z
END SUB

' camera and the eye position the shaders need
SUB Look (ex, ey, ez, tx, ty, tz, fov)
    _CAMERA (ex, ey, ez) TO (tx, ty, tz), fov
    _SHADERPARAM shiny&, 0, ex: _SHADERPARAM shiny&, 1, ey: _SHADERPARAM shiny&, 2, ez
    _SHADERPARAM glow&, 0, ex: _SHADERPARAM glow&, 1, ey: _SHADERPARAM glow&, 2, ez
END SUB

SUB DrawGround (size, radius)
    _SHADERPARAM ground&, 12, size: _SHADERPARAM ground&, 13, radius: _SHADERPARAM ground&, 14, radius * 0.4
    _DRAW3D floorm&
END SUB

SUB DrawPendulums
    ' pendulum n swings 16 + n times in 40 s: periods from 2.5 s down to 1.3 s
    a = 0.75 + 0.3 * SIN(tsim * 0.15)
    Look 8.6 * SIN(a), 5.2, 8.6 * COS(a), 0, 1.7, 0, 45
    DrawGround 1, 9
    top = 3.4
    _DRAW3D metal&, 0, top + 0.04, 0, 0, 0, 0, 6.1, 0.08, 0.08
    _DRAW3D metal&, -3, top / 2, 0, 0, 0, 0, 0.1, top + 0.1, 0.1
    _DRAW3D metal&, 3, top / 2, 0, 0, 0, 0, 0.1, top + 0.1, 0.1
    _DRAW3D metal&, 0, 0.02, 0, 0, 0, 0, 6.4, 0.04, 0.5
    FOR n = 0 TO NP - 1
        period = 40 / (16 + n)
        length = 2.6 * (period / 2.5) ^ 2
        th = 0.45 * COS(2 * PI * tsim / period)
        x = (n - (NP - 1) / 2) * 0.36
        cy = top - length * COS(th): cz = -length * SIN(th)
        _DRAW3D metal&, x, (top + cy) / 2, cz / 2, th * 180 / PI, 0, 0, 0.018, length, 0.018
        _DRAW3D ball&((n * HUES) \ NP), x, cy, cz, 0, 0, 0, 0.14, 0.14, 0.14
    NEXT
END SUB

SUB DrawOrbits
    a = 0.4 + tsim * 0.07
    Look 13.5 * COS(a), 7, 13.5 * SIN(a), 0, -0.4, 0, 50
    _SHADERPARAM shiny&, 8, 0: _SHADERPARAM shiny&, 9, 0: _SHADERPARAM shiny&, 10, 0
    _DRAW3D sun&, 0, 0, 0, 0, tsim * 10, 0
    FOR i = 0 TO NPL - 1
        s = psize(i)
        _DRAW3D planet&(i), px(i), py(i), pz(i), 0, tsim * 40, 0, s, s, s
        FOR j = 1 TO TRAIL - 1
            k = (tailn& - j + TRAIL) MOD TRAIL
            d = 0.055 * (1 - j / TRAIL) + 0.01
            _DRAW3D planet&(i), tailx(i, k), taily(i, k), tailz(i, k), 0, 0, 0, d, d, d
        NEXT
    NEXT
    ' a moon around the blue planet
    ma = tsim * 4.2
    _DRAW3D moon&, px(2) + 0.6 * COS(ma), py(2) + 0.12 * SIN(ma), pz(2) + 0.6 * SIN(ma), 0, 0, 0, 0.08, 0.08, 0.08
    ' see-through things last: the ring and the corona
    _DRAW3D ringm&, px(5), py(5), pz(5), 20, 0, 12, 0.48, 0.48, 0.48
    _DRAW3D halo&, 0, 0, 0, 0, 0, 0, 1.45, 1.45, 1.45
    _DRAW3D halo&, 0, 0, 0, 0, 0, 0, 2.1, 2.1, 2.1
END SUB

SUB DrawBalls
    a = 0.6 + tsim * 0.12
    Look 9.8 * COS(a), 6.2, 9.8 * SIN(a), 0, 0.7, 0, 50
    DrawGround 1, 13
    FOR i = 0 TO NBALL - 1
        r = brad(i)
        _DRAW3D ball&(bhue(i)), bx(i), by(i), bz(i), 0, 0, 0, r, r, r
    NEXT
    FOR i = 0 TO NBALL - 1
        hgt = by(i) - brad(i)
        k = INT(hgt / 0.7): IF k > 7 THEN k = 7
        s = brad(i) * (1.1 + 0.3 * hgt)
        _DRAW3D shadow&(k), bx(i), 0.005 + i * 0.0002, bz(i), 0, 0, 0, s, 1, s
    NEXT
    w = HALF + 0.06
    _DRAW3D glass&, 0, 0.3, w, 0, 0, 0, 2 * w + 0.12, 0.6, 0.12
    _DRAW3D glass&, 0, 0.3, -w, 0, 0, 0, 2 * w + 0.12, 0.6, 0.12
    _DRAW3D glass&, w, 0.3, 0, 0, 0, 0, 0.12, 0.6, 2 * w
    _DRAW3D glass&, -w, 0.3, 0, 0, 0, 0, 0.12, 0.6, 2 * w
END SUB

SUB DrawWaves
    a = 0.55 + tsim * 0.1
    Look 9 * COS(a), 6.8, 9 * SIN(a), 0, -0.3, 0, 48
    k = 2 * PI / 1.1: w = 2 * PI * 0.7: sp = 0.2
    FOR i = 0 TO GRID - 1
        x = (i - (GRID - 1) / 2) * sp
        FOR j = 0 TO GRID - 1
            z = (j - (GRID - 1) / 2) * sp
            hgt = Wave(x, z, k, w)
            ci = INT((hgt + 0.75) / 1.5 * (HUES - 1)): IF ci < 0 THEN ci = 0
            IF ci > HUES - 1 THEN ci = HUES - 1
            hh = 0.9 + hgt
            _DRAW3D water&(ci), x, hh / 2 - 0.5, z, 0, 0, 0, sp * 0.9, hh, sp * 0.9
        NEXT
    NEXT
    ' the two sources
    FOR s = -1 TO 1 STEP 2
        hgt = Wave(s * 1.6, 0, k, w)
        _DRAW3D ball&(3), s * 1.6, hgt + 0.75, 0, 0, 0, 0, 0.16, 0.16, 0.16
    NEXT
END SUB

' height of the water at (x, z): two circular waves that fade with distance
FUNCTION Wave (x, z, k, w)
    r1 = SQR((x - 1.6) ^ 2 + z * z): r2 = SQR((x + 1.6) ^ 2 + z * z)
    Wave = 0.38 * (SIN(k * r1 - w * tsim) / (1 + 0.3 * r1) + SIN(k * r2 - w * tsim) / (1 + 0.3 * r2))
END FUNCTION

SUB DrawLorenz
    a = PI / 2 + 0.55 * SIN(tsim * 0.22)
    Look 10.5 * COS(a), 1.8, 10.5 * SIN(a), 0, 0.1, 0, 46
    piece = LSTEPS \ CHUNKS
    shown = 1 + fr& \ 7: IF shown > CHUNKS THEN shown = CHUNKS
    FOR c = 0 TO shown - 1
        _DRAW3D tube&(c)
    NEXT
    IF shown < CHUNKS THEN p = shown * piece ELSE p = (fr& * 12) MOD LSTEPS
    _DRAW3D head&, lx(p), ly(p), lz(p), 0, 0, 0, 0.11, 0.11, 0.11
END SUB

' ---- the picture behind the 3D layer -----------------------------------------

SUB Backdrop (s%)
    SELECT CASE s%
        CASE 1: r1 = 40: g1 = 46: b1 = 70: r2 = 140: g2 = 150: b2 = 175
        CASE 2: r1 = 2: g1 = 3: b1 = 10: r2 = 12: g2 = 14: b2 = 34
        CASE 3: r1 = 70: g1 = 90: b1 = 130: r2 = 200: g2 = 205: b2 = 215
        CASE 4: r1 = 6: g1 = 12: b1 = 30: r2 = 30: g2 = 60: b2 = 100
        CASE ELSE: r1 = 8: g1 = 6: b1 = 18: r2 = 40: g2 = 24: b2 = 60
    END SELECT
    FOR y = 0 TO SH - 1 STEP 4
        k = y / SH
        LINE (0, y)-(SW - 1, y + 3), _RGB32(r1 + (r2 - r1) * k, g1 + (g2 - g1) * k, b1 + (b2 - b1) * k), BF
    NEXT
    IF s% = 2 OR s% = 5 THEN
        FOR i = 0 TO NSTAR - 1
            c = starc(i): IF s% = 5 THEN c = c * 0.5
            PSET (starx(i), stary(i)), _RGB32(c, c, c + 20)
        NEXT
    END IF
END SUB
