' ============================================================================
'  solarsystem.bas  -  the solar system in 3D (for QBNex 2.0, preview.4 or newer)
'
'  The Sun, eight planets with their real orbital periods, the Moon, the four
'  big moons of Jupiter, Titan and the rings of Saturn and Uranus.
'  The planets start roughly where they really are on today's date
'  (circular orbits, mean longitudes from J2000). Distances and sizes are
'  squeezed, otherwise Neptune would be 4500 times farther away than the
'  Earth is big - and nothing would be left to see on the screen.
'
'  Drawn with the 3D statements of QBNex (_MESH*, _CAMERA, _LIGHT, _DRAW3D,
'  _SHADER). The glow shader follows the QBNex example
'  examples/showcase/physics3d.bas.
'
'  Keys:  drag mouse / arrows = turn camera   wheel / PgUp/PgDn = zoom
'         1-8 = follow a planet   0 = Sun     + / - = faster / slower
'         Space = pause   R = backwards   T = today
'         N = names   O = orbits   I = info   H = help   Esc = quit
'
'  Written by Gokipp (https://github.com/Gokipp).
' ============================================================================
CONST PI = 3.14159265
CONST NPL = 8 '        planets
CONST NMO = 6 '        moons
CONST NSTAR = 12000 ' stars (many faint ones in the Milky Way)
DIM SHARED SW, SH
SW = 1200: SH = 720
IF _DESKTOPWIDTH > 0 THEN
    IF _DESKTOPWIDTH - 40 < SW THEN SW = _DESKTOPWIDTH - 40
    IF _DESKTOPHEIGHT - 100 < SH THEN SH = _DESKTOPHEIGHT - 100
END IF
SCREEN _NEWIMAGE(SW, SH, 32)
_TITLE "Solar System 3D"
_PRINTMODE _KEEPBACKGROUND
RANDOMIZE 11

IF _GPU3D = 0 THEN
    COLOR _RGB32(255, 200, 140)
    _PRINTSTRING (40, SH \ 2 - 20), "No GPU for 3D found."
    _PRINTSTRING (40, SH \ 2), "The solar system needs QBNex 2.0 (preview.4 or newer) with a GPU."
    _PRINTSTRING (40, SH \ 2 + 30), "Press a key ..."
    SLEEP
    SYSTEM
END IF

' ---- planet data -----------------------------------------------------------
DIM SHARED pname$(NPL), pau(NPL), pper(NPL), prad(NPL), pkm(NPL), pl0(NPL)
DIM SHARED pdist(NPL), psize(NPL), ptilt(NPL), pday(NPL), pmoons$(NPL)
DIM SHARED px(NPL), py(NPL), pz(NPL), pmesh&(NPL), porbit&(NPL)
' moons: which planet, distance, period in days, size
DIM SHARED mname$(NMO), mhost(NMO), mdist(NMO), mper(NMO), msize(NMO), mmesh&(NMO)
DIM SHARED mx(NMO), my(NMO), mz(NMO)
DIM SHARED starx(NSTAR), stary(NSTAR), starz(NSTAR), starc(NSTAR), startw(NSTAR), starph(NSTAR)
DIM SHARED starr(NSTAR), starg(NSTAR), starb(NSTAR)
DIM SHARED sun&, halo&, satring&, urring&
DIM SHARED glow&, flat&
DIM SHARED camYaw, camPitch, camDist, camFov, focus
DIM SHARED ex, ey, ez, tx, ty, tz
DIM SHARED jd#, speed, paused, backw, showNames, showOrbits, showInfo, showHelp

RESTORE planetdata
FOR i = 1 TO NPL
    READ pname$(i), pau(i), pper(i), pkm(i), pl0(i), ptilt(i), pday(i), pmoons$(i)
    pdist(i) = 2.8 + 7 * LOG(1 + 2.2 * pau(i)) ' squeezed distance
    psize(i) = 0.34 * SQR(pkm(i) / 6371) '   squeezed size
    IF psize(i) < 0.2 THEN psize(i) = 0.2
NEXT
RESTORE moondata
FOR i = 1 TO NMO
    READ mname$(i), mhost(i), mdist(i), mper(i), msize(i)
NEXT

BuildShaders
BuildMeshes
MakeStars

' light: the Sun is a point light in the middle, plus a little ambient light
sunlight& = _LIGHT(2, 0, 0, 0, _RGB32(255, 244, 225), 1.45, 70)
_AMBIENT _RGB32(255, 255, 255), 0.07
_CAMERARANGE 0.05, 400

camYaw = 35: camPitch = 28: camDist = 62: camFov = 45: focus = 0
showNames = -1: showOrbits = -1: showInfo = -1: showHelp = 0
speed = 4 '               step in speeds()
DIM SHARED speeds(10), speedname$(10)
RESTORE speeddata
FOR i = 1 TO 10: READ speeds(i), speedname$(i): NEXT
jd# = TodayJD#

lastT# = TIMER(.001)
DO
    _LIMIT 60
    now# = TIMER(.001): dt# = now# - lastT#: lastT# = now#
    IF dt# < 0 OR dt# > .5 THEN dt# = 1 / 60
    IF HandleInput THEN EXIT DO
    IF paused = 0 THEN
        d# = speeds(speed) * dt#
        IF backw THEN d# = -d#
        jd# = jd# + d#
    END IF
    UpdatePositions
    SetCamera
    DrawBackground
    _CLEAR3D 0
    DrawScene
    DrawLabels
    DrawHud
    _DISPLAY
LOOP
SYSTEM

' name, distance (AU), period (years), radius (km), longitude J2000 (degrees),
' axial tilt (degrees), length of day (hours), moons
planetdata:
DATA "Mercury",0.387,0.2408,2440,252.25,0.03,4223,"none"
DATA "Venus",0.723,0.6152,6052,181.98,177.4,-5832,"none"
DATA "Earth",1.000,1.0000,6371,100.46,23.44,23.93,"1 (the Moon)"
DATA "Mars",1.524,1.8809,3390,355.45,25.19,24.62,"2 (Phobos, Deimos)"
DATA "Jupiter",5.203,11.862,69911,34.40,3.13,9.93,"95 (shown: Io, Europa, Ganymede, Callisto)"
DATA "Saturn",9.537,29.457,58232,49.94,26.73,10.66,"146 (shown: Titan)"
DATA "Uranus",19.19,84.011,25362,313.23,97.77,-17.24,"28"
DATA "Neptune",30.07,164.79,24622,304.88,28.32,16.11,"16"

' moon, planet, distance (in planet sizes), period (days), size
moondata:
DATA "Moon",3,2.3,27.32,0.09
DATA "Io",5,1.55,1.769,0.07
DATA "Europa",5,1.85,3.551,0.065
DATA "Ganymede",5,2.25,7.155,0.09
DATA "Callisto",5,2.75,16.69,0.085
DATA "Titan",6,2.85,15.95,0.09

' days per second
speeddata:
DATA 0.0417,"1 hour/s",0.25,"6 hours/s",1,"1 day/s",5,"5 days/s",20,"20 days/s"
DATA 60,"2 months/s",182.6,"1/2 year/s",365.25,"1 year/s",1826,"5 years/s",7305,"20 years/s"

' ---- building the scene ------------------------------------------------------

SUB BuildShaders
    nl$ = CHR$(10)
    ' glow (Sun, corona): not lit, brighter in the middle;
    ' see-through shells fade out towards the edge.
    s$ = "fn shade(i: ShadeIn) -> vec4<f32> {" + nl$
    s$ = s$ + "    let k = max(dot(i.normal, normalize(i.params[0].xyz - i.world)), 0.0);" + nl$
    s$ = s$ + "    if (i.base.a < 0.99) { return vec4<f32>(i.base.rgb, i.base.a * k * k * k); }" + nl$
    s$ = s$ + "    return vec4<f32>(i.base.rgb * (0.75 + 0.45 * k), 1.0);" + nl$
    s$ = s$ + "}"
    glow& = MakeShader&(s$)
    ' flat: colour without light (orbits, rings)
    s$ = "fn shade(i: ShadeIn) -> vec4<f32> {" + nl$
    s$ = s$ + "    return vec4<f32>(i.base.rgb * i.params[0].x, i.base.a);" + nl$
    s$ = s$ + "}"
    flat& = MakeShader&(s$)
    _SHADERPARAM flat&, 0, 1
END SUB

FUNCTION MakeShader& (src$)
    h& = _SHADER(src$)
    IF h& = -1 THEN
        COLOR _RGB32(255, 160, 120)
        PRINT "Shader error: "; _SHADERERROR$
        _DISPLAY
        SLEEP
        SYSTEM
    END IF
    MakeShader& = h&
END FUNCTION

' sphere with radius 1 and texture coordinates (u around, v from top to bottom)
FUNCTION Sphere& (sl, st, tex&)
    _MESHBEGIN
    FOR i = 0 TO st
        th = i / st * PI
        FOR j = 0 TO sl
            ph = j / sl * 2 * PI
            _MESHVERTEX SIN(th) * COS(ph), COS(th), -SIN(th) * SIN(ph), j / sl, i / st
        NEXT
    NEXT
    FOR i = 0 TO st - 1
        FOR j = 0 TO sl - 1
            a& = i * (sl + 1) + j: b& = a& + 1
            c& = a& + sl + 1: d& = c& + 1
            IF i > 0 THEN _MESHTRIANGLE a&, c&, b&
            IF i < st - 1 THEN _MESHTRIANGLE b&, c&, d&
        NEXT
    NEXT
    m& = _MESHEND
    _MESHCOLOR m&, _RGB32(255, 255, 255)
    IF tex& < -1 THEN _MESHTEXTURE m&, tex&
    Sphere& = m&
END FUNCTION

' flat ring in the XZ plane from r1 to r2
FUNCTION Ring& (r1, r2, n, colour~&)
    _MESHBEGIN
    FOR j = 0 TO n - 1
        a = j / n * 2 * PI
        _MESHVERTEX r1 * COS(a), 0, r1 * SIN(a)
        _MESHVERTEX r2 * COS(a), 0, r2 * SIN(a)
    NEXT
    FOR j = 0 TO n - 1
        j2 = (j + 1) MOD n
        _MESHTRIANGLE 2 * j, 2 * j + 1, 2 * j2
        _MESHTRIANGLE 2 * j + 1, 2 * j2 + 1, 2 * j2
        ' back side, so the ring can be seen from below too
        _MESHTRIANGLE 2 * j, 2 * j2, 2 * j + 1
        _MESHTRIANGLE 2 * j + 1, 2 * j2, 2 * j2 + 1
    NEXT
    m& = _MESHEND
    _MESHCOLOR m&, colour~&
    _MESHSHADER m&, flat&
    Ring& = m&
END FUNCTION

SUB BuildMeshes
    sun& = Sphere&(48, 32, MakeTexture&(0))
    _MESHSHADER sun&, glow&
    halo& = Sphere&(32, 20, 0)
    _MESHCOLOR halo&, _RGBA32(255, 150, 40, 140)
    _MESHSHADER halo&, glow&
    FOR i = 1 TO NPL
        pmesh&(i) = Sphere&(48, 32, MakeTexture&(i))
        porbit&(i) = Ring&(pdist(i) - 0.025, pdist(i) + 0.025, 256, _RGBA32(120, 150, 220, 110))
    NEXT
    _MESHCOLOR porbit&(3), _RGBA32(90, 170, 255, 110)
    FOR i = 1 TO NMO
        mmesh&(i) = Sphere&(24, 16, 0)
        SELECT CASE i
            CASE 1: c~& = _RGB32(200, 200, 200)
            CASE 2: c~& = _RGB32(235, 215, 110)
            CASE 3: c~& = _RGB32(215, 200, 180)
            CASE 4: c~& = _RGB32(170, 160, 150)
            CASE 5: c~& = _RGB32(120, 110, 100)
            CASE ELSE: c~& = _RGB32(225, 170, 90)
        END SELECT
        _MESHCOLOR mmesh&(i), c~&
    NEXT
    satring& = Ring&(1.35, 2.3, 128, _RGBA32(225, 205, 160, 185))
    urring& = Ring&(1.7, 1.85, 96, _RGBA32(170, 200, 210, 90))
END SUB

' surface of every body as a small image (256 x 128)
FUNCTION MakeTexture& (k)
    w = 256: h = 128
    img& = _NEWIMAGE(w, h, 32)
    old& = _DEST: _DEST img&
    SELECT CASE k
        CASE 0 ' Sun: granulation
            LINE (0, 0)-(w - 1, h - 1), _RGB32(255, 170, 40), BF
            FOR n = 1 TO 1400
                c = RND * 60
                r = 1 + RND * 3
                Disc RND * w, RND * h, r, r, _RGB32(255, 150 + c, 30 + c / 2)
            NEXT
        CASE 1 ' Mercury: grey with craters
            LINE (0, 0)-(w - 1, h - 1), _RGB32(150, 145, 140), BF
            Blobs w, h, 160, 2, 8, 110, 105, 100
            Blobs w, h, 80, 1, 3, 190, 185, 180
        CASE 2 ' Venus: yellowish clouds
            FOR y = 0 TO h - 1
                c = 20 * SIN(y / 7) + 10 * SIN(y / 3)
                LINE (0, y)-(w - 1, y), _RGB32(230 + c / 2, 200 + c, 140 + c)
            NEXT
            Blobs w, h, 120, 4, 14, 240, 220, 170
        CASE 3 ' Earth: sea, continents, ice caps, clouds
            LINE (0, 0)-(w - 1, h - 1), _RGB32(30, 80, 170), BF
            Continent 60, 40, 30, 22: Continent 75, 85, 18, 26
            Continent 140, 38, 40, 20: Continent 140, 80, 22, 26
            Continent 190, 45, 35, 18: Continent 215, 95, 16, 10
            LINE (0, 0)-(w - 1, 9), _RGB32(240, 245, 250), BF
            LINE (0, h - 11)-(w - 1, h - 1), _RGB32(240, 245, 250), BF
            FOR n = 1 TO 40
                Disc RND * w, 12 + RND * (h - 24), 3 + RND * 9, 1 + RND * 2, _RGBA32(240, 245, 250, 200)
            NEXT
        CASE 4 ' Mars: red with dark spots and a polar cap
            LINE (0, 0)-(w - 1, h - 1), _RGB32(190, 90, 50), BF
            Blobs w, h, 90, 4, 14, 140, 60, 35
            LINE (0, 0)-(w - 1, 6), _RGB32(240, 235, 230), BF
        CASE 5 ' Jupiter: bands and the Great Red Spot
            Bands w, h, 225, 190, 150, 170, 120, 80, 9
            Disc 170, 82, 15, 7, _RGB32(205, 110, 75)
            Disc 170, 82, 10, 4, _RGB32(190, 90, 60)
        CASE 6 ' Saturn: pale bands
            Bands w, h, 230, 210, 160, 200, 175, 120, 6
        CASE 7 ' Uranus: blue-green
            Bands w, h, 170, 225, 230, 150, 210, 220, 3
        CASE 8 ' Neptune: blue with a dark spot
            Bands w, h, 70, 110, 220, 50, 85, 190, 4
            Disc 90, 55, 11, 6, _RGB32(30, 50, 120)
    END SELECT
    _DEST old&
    MakeTexture& = img&
END FUNCTION

SUB Blobs (w, h, n, r1, r2, r, g, b)
    FOR i = 1 TO n
        x = RND * w: y = RND * h: rr = r1 + RND * (r2 - r1)
        c = RND * 20 - 10
        Disc x, y, rr, rr, _RGB32(r + c, g + c, b + c)
    NEXT
END SUB

' filled ellipse
SUB Disc (x, y, rx, ry, c~&)
    FOR dy = -ry TO ry
        k = 1 - (dy / ry) ^ 2
        IF k > 0 THEN
            dx = rx * SQR(k)
            LINE (x - dx, y + dy)-(x + dx, y + dy), c~&
        END IF
    NEXT
END SUB

SUB Continent (x, y, rx, ry)
    FOR n = 1 TO 26
        a = RND * 2 * PI: d = RND
        cx = x + COS(a) * rx * d: cy = y + SIN(a) * ry * d
        rr = 3 + RND * (rx + ry) / 7
        c = RND * 30
        Disc cx, cy, rr, rr * 0.8, _RGB32(60 + c, 130 + c, 50 + c / 2)
    NEXT
END SUB

SUB Bands (w, h, r1, g1, b1, r2, g2, b2, n)
    FOR y = 0 TO h - 1
        k = 0.5 + 0.5 * SIN(y / h * PI * n + SIN(y / 5) * 0.6)
        LINE (0, y)-(w - 1, y), _RGB32(r1 + (r2 - r1) * k, g1 + (g2 - g1) * k, b1 + (b2 - b1) * k)
    NEXT
END SUB

' ---- time and orbits ------------------------------------------------------------

' Julian date of today (noon)
FUNCTION TodayJD#
    d$ = DATE$ '  mm-dd-yyyy
    TodayJD# = JulianDay#(VAL(MID$(d$, 7, 4)), VAL(LEFT$(d$, 2)), VAL(MID$(d$, 4, 2)))
END FUNCTION

FUNCTION JulianDay# (y, m, d)
    a = INT((14 - m) / 12)
    yy = y + 4800 - a: mm = m + 12 * a - 3
    JulianDay# = d + INT((153 * mm + 2) / 5) + 365 * yy + INT(yy / 4) - INT(yy / 100) + INT(yy / 400) - 32045
END FUNCTION

FUNCTION DateText$ (j#)
    jj& = INT(j# + 0.5)
    a& = jj& + 32044
    b& = (4 * a& + 3) \ 146097
    c& = a& - (146097 * b&) \ 4
    d& = (4 * c& + 3) \ 1461
    e& = c& - (1461 * d&) \ 4
    m& = (5 * e& + 2) \ 153
    dd& = e& - (153 * m& + 2) \ 5 + 1
    mo& = m& + 3 - 12 * (m& \ 10)
    yy& = 100 * b& + d& - 4800 + m& \ 10
    DateText$ = LTRIM$(STR$(dd&)) + " " + MID$("JanFebMarAprMayJunJulAugSepOctNovDec", mo& * 3 - 2, 3) + " " + LTRIM$(STR$(yy&))
END FUNCTION

FUNCTION Two$ (n&)
    Two$ = RIGHT$("0" + LTRIM$(STR$(n&)), 2)
END FUNCTION

SUB UpdatePositions
    t# = (jd# - 2451545#) / 365.25 '  years since J2000
    FOR i = 1 TO NPL
        l# = (pl0(i) + 360# * t# / pper(i)) * PI / 180
        px(i) = pdist(i) * COS(l#): py(i) = 0: pz(i) = -pdist(i) * SIN(l#)
    NEXT
    days# = jd# - 2451545#
    FOR i = 1 TO NMO
        hh = mhost(i)
        a# = 2 * PI * days# / mper(i) + i * 1.3
        r = mdist(i) * psize(hh)
        mx(i) = px(hh) + r * COS(a#)
        mz(i) = pz(hh) - r * SIN(a#)
        my(i) = 0
        IF hh = 6 THEN my(i) = -r * SIN(a#) * SIN(ptilt(6) * PI / 180) * 0.3
    NEXT
END SUB

' ---- camera and input ------------------------------------------------------------

SUB SetCamera
    IF focus = 0 THEN
        tx = 0: ty = 0: tz = 0
    ELSE
        tx = px(focus): ty = py(focus): tz = pz(focus)
    END IF
    yw = camYaw * PI / 180: pt = camPitch * PI / 180
    ex = tx + camDist * COS(pt) * COS(yw)
    ey = ty + camDist * SIN(pt)
    ez = tz + camDist * COS(pt) * SIN(yw)
    _CAMERA (ex, ey, ez)TO(tx, ty, tz), camFov
    _SHADERPARAM glow&, 0, ex: _SHADERPARAM glow&, 1, ey: _SHADERPARAM glow&, 2, ez
END SUB

FUNCTION HandleInput
    STATIC dragX, dragY, dragging
    HandleInput = 0
    wheel = 0
    DO WHILE _MOUSEINPUT
        wheel = wheel + _MOUSEWHEEL
    LOOP
    IF _MOUSEBUTTON(1) THEN
        IF dragging THEN
            camYaw = camYaw + (_MOUSEX - dragX) * 0.4
            camPitch = camPitch + (_MOUSEY - dragY) * 0.3
        END IF
        dragX = _MOUSEX: dragY = _MOUSEY: dragging = -1
    ELSE
        dragging = 0
    END IF
    IF wheel <> 0 THEN camDist = camDist * 1.12 ^ wheel
    IF _KEYDOWN(19200) THEN camYaw = camYaw - 1.5 '   left
    IF _KEYDOWN(19712) THEN camYaw = camYaw + 1.5 '   right
    IF _KEYDOWN(18432) THEN camPitch = camPitch + 1 ' up
    IF _KEYDOWN(20480) THEN camPitch = camPitch - 1 ' down
    IF _KEYDOWN(18688) THEN camDist = camDist / 1.03 ' PgUp
    IF _KEYDOWN(20736) THEN camDist = camDist * 1.03 ' PgDn
    k$ = UCASE$(INKEY$)
    SELECT CASE k$
        CASE CHR$(27): HandleInput = -1
        CASE "+", "=": IF speed < 10 THEN speed = speed + 1
        CASE "-", "_": IF speed > 1 THEN speed = speed - 1
        CASE " ": paused = NOT paused
        CASE "R": backw = NOT backw
        CASE "T": jd# = TodayJD#
        CASE "N": showNames = NOT showNames
        CASE "O": showOrbits = NOT showOrbits
        CASE "I": showInfo = NOT showInfo
        CASE "H", "?": showHelp = NOT showHelp
        CASE "0": focus = 0: camDist = 62
        CASE "1" TO "8"
            focus = VAL(k$)
            camDist = psize(focus) * 9 + 1.5
            IF focus = 5 OR focus = 6 THEN camDist = psize(focus) * 7
    END SELECT
    IF camPitch > 89 THEN camPitch = 89
    IF camPitch < -89 THEN camPitch = -89
    lo = 1.2: IF focus > 0 THEN lo = psize(focus) * 2.2
    IF camDist < lo THEN camDist = lo
    IF camDist > 160 THEN camDist = 160
END FUNCTION

' ---- drawing -------------------------------------------------------------------

' Stars are directions in the sky: they turn with the camera, but stay
' infinitely far away (they do not move when the camera comes closer).
SUB MakeStars
    ' plane of the Milky Way: tilted against the plane of the planets
    nx = 0.52: ny = 0.45: nz = -0.73
    FOR i = 1 TO NSTAR
        IF i > 4000 THEN
            ' Milky Way: many faint stars close to a great circle
            DO
                x = RND * 2 - 1: y = RND * 2 - 1: z = RND * 2 - 1
                l = SQR(x * x + y * y + z * z)
            LOOP UNTIL l > 0.1 AND l <= 1
            x = x / l: y = y / l: z = z / l
            d = x * nx + y * ny + z * nz
            sp = (RND + RND - 1) * 0.16
            x = x - (d - sp) * nx: y = y - (d - sp) * ny: z = z - (d - sp) * nz
            c = 30 + RND * 70
        ELSE
            DO
                x = RND * 2 - 1: y = RND * 2 - 1: z = RND * 2 - 1
                l = SQR(x * x + y * y + z * z)
            LOOP UNTIL l > 0.1 AND l <= 1
            c = RND: c = 70 + 185 * c * c * c
        END IF
        l = SQR(x * x + y * y + z * z)
        starx(i) = x / l: stary(i) = y / l: starz(i) = z / l
        starc(i) = c
        ' colour: mostly white, some bluish, yellowish or reddish
        k = RND
        IF k < 0.15 THEN
            starr(i) = 0.75: starg(i) = 0.85: starb(i) = 1
        ELSEIF k < 0.3 THEN
            starr(i) = 1: starg(i) = 0.9: starb(i) = 0.7
        ELSEIF k < 0.38 THEN
            starr(i) = 1: starg(i) = 0.7: starb(i) = 0.6
        ELSE
            starr(i) = 0.95: starg(i) = 0.95: starb(i) = 1
        END IF
        ' twinkling: every star flickers at its own rate
        startw(i) = 1.5 + RND * 5: starph(i) = RND * 2 * PI
    NEXT
END SUB

SUB DrawBackground
    LINE (0, 0)-(SW - 1, SH - 1), _RGB32(2, 3, 10), BF
    ' camera direction (forward, right, up)
    fx = tx - ex: fy = ty - ey: fz = tz - ez
    l = SQR(fx * fx + fy * fy + fz * fz): fx = fx / l: fy = fy / l: fz = fz / l
    rx = -fz: rz = fx: l = SQR(rx * rx + rz * rz): IF l < 1E-6 THEN l = 1
    rx = rx / l: rz = rz / l
    ux = -rz * fy: uy = rz * fx - rx * fz: uz = rx * fy
    f = (SH / 2) / TAN(camFov * PI / 360)
    t = TIMER
    FOR i = 1 TO NSTAR
        cz = starx(i) * fx + stary(i) * fy + starz(i) * fz
        IF cz > 0.05 THEN
            sx = SW / 2 + (starx(i) * rx + starz(i) * rz) / cz * f
            sy = SH / 2 - (starx(i) * ux + stary(i) * uy + starz(i) * uz) / cz * f
            IF sx >= 0 AND sx < SW AND sy >= 0 AND sy < SH THEN
                c = starc(i)
                IF c > 60 THEN c = c * (0.72 + 0.28 * SIN(t * startw(i) + starph(i)))
                PSET (sx, sy), _RGB32(c * starr(i), c * starg(i), c * starb(i))
                IF c > 170 THEN
                    ' bright stars a little bigger
                    h~& = _RGB32(c * starr(i) * 0.45, c * starg(i) * 0.45, c * starb(i) * 0.45)
                    PSET (sx + 1, sy), h~&: PSET (sx - 1, sy), h~&: PSET (sx, sy + 1), h~&: PSET (sx, sy - 1), h~&
                END IF
            END IF
        END IF
    NEXT
END SUB

SUB DrawScene
    ' first everything solid ...
    sr = 2.2
    _DRAW3D sun&, 0, 0, 0, 0, (jd# - 2451545#) * 360 / 25.4, 7.25, sr, sr, sr
    FOR i = 1 TO NPL
        s = psize(i)
        rot = ((jd# - 2451545#) * 24 / pday(i)) * 360
        rot = rot - INT(rot / 360) * 360
        _DRAW3D pmesh&(i), px(i), py(i), pz(i), 0, rot, ptilt(i), s, s, s
    NEXT
    FOR i = 1 TO NMO
        s = msize(i)
        _DRAW3D mmesh&(i), mx(i), my(i), mz(i), 0, 0, 0, s, s, s
    NEXT
    ' ... then everything see-through
    IF showOrbits THEN
        FOR i = 1 TO NPL
            _DRAW3D porbit&(i)
        NEXT
    END IF
    s = psize(6): _DRAW3D satring&, px(6), py(6), pz(6), 0, 0, ptilt(6), s, s, s
    s = psize(7): _DRAW3D urring&, px(7), py(7), pz(7), 0, 0, ptilt(7), s, s, s
    _DRAW3D halo&, 0, 0, 0, 0, 0, 0, sr * 1.35, sr * 1.35, sr * 1.35
    _DRAW3D halo&, 0, 0, 0, 0, 0, 0, sr * 1.9, sr * 1.9, sr * 1.9
END SUB

' point in space -> screen (the way _CAMERA does it). Result 0 = behind the camera.
FUNCTION Project (x, y, z, sx, sy)
    fx = tx - ex: fy = ty - ey: fz = tz - ez
    l = SQR(fx * fx + fy * fy + fz * fz): fx = fx / l: fy = fy / l: fz = fz / l
    ' right = forward x up(0,1,0)
    rx = -fz: ry = 0: rz = fx
    l = SQR(rx * rx + rz * rz): IF l < 1E-6 THEN l = 1
    rx = rx / l: rz = rz / l
    ' up = right x forward
    ux = ry * fz - rz * fy: uy = rz * fx - rx * fz: uz = rx * fy - ry * fx
    dx = x - ex: dy = y - ey: dz = z - ez
    cz = dx * fx + dy * fy + dz * fz
    IF cz < 0.1 THEN Project = 0: EXIT FUNCTION
    cx = dx * rx + dy * ry + dz * rz
    cy = dx * ux + dy * uy + dz * uz
    f = (SH / 2) / TAN(camFov * PI / 360)
    sx = SW / 2 + cx / cz * f
    sy = SH / 2 - cy / cz * f
    Project = f / cz '  pixels per unit at this distance
END FUNCTION

SUB DrawLabels
    IF showNames = 0 THEN EXIT SUB
    FOR i = 1 TO NPL
        k = Project(px(i), py(i), pz(i), sx, sy)
        IF k > 0 THEN
            n$ = pname$(i)
            yy = sy + psize(i) * k + 4
            IF i = focus THEN COLOR _RGB32(255, 230, 120) ELSE COLOR _RGB32(170, 190, 230)
            _PRINTSTRING (sx - LEN(n$) * 4, yy), n$
        END IF
    NEXT
    IF focus > 0 THEN
        FOR i = 1 TO NMO
            IF mhost(i) = focus THEN
                k = Project(mx(i), my(i), mz(i), sx, sy)
                IF k > 0 THEN
                    COLOR _RGB32(150, 150, 160)
                    _PRINTSTRING (sx - LEN(mname$(i)) * 4, sy + msize(i) * k + 2), mname$(i)
                END IF
            END IF
        NEXT
    END IF
END SUB

SUB DrawHud
    COLOR _RGB32(230, 230, 240)
    t$ = "Date: " + DateText$(jd#) + "    Speed: " + speedname$(speed)
    IF backw THEN t$ = t$ + " backwards"
    IF paused THEN t$ = t$ + "   (paused)"
    _PRINTSTRING (14, 10), t$
    COLOR _RGB32(140, 150, 175)
    _PRINTSTRING (14, SH - 22), "H = help   1-8 = planet   0 = Sun   +/- = speed   Space = pause   Esc = quit"
    IF showInfo AND focus > 0 THEN
        i = focus
        x = 14: y = 40
        LINE (x - 6, y - 6)-(x + 372, y + 118), _RGBA32(10, 20, 50, 200), BF
        LINE (x - 6, y - 6)-(x + 372, y + 118), _RGB32(90, 120, 190), B
        COLOR _RGB32(255, 230, 120): _PRINTSTRING (x, y), pname$(i)
        COLOR _RGB32(220, 225, 235)
        _PRINTSTRING (x, y + 20), "Diameter:      " + Num$(pkm(i) * 2, 0) + " km"
        _PRINTSTRING (x, y + 36), "Distance:      " + Num$(pau(i), 2) + " AU  (" + Num$(pau(i) * 149.6, 0) + " million km)"
        IF pper(i) < 2 THEN
            u$ = Num$(pper(i) * 365.25, 0) + " days"
        ELSE
            u$ = Num$(pper(i), 1) + " years"
        END IF
        _PRINTSTRING (x, y + 52), "Orbit:         " + u$
        IF ABS(pday(i)) > 48 THEN
            d$ = Num$(ABS(pday(i)) / 24, 0) + " days"
        ELSE
            d$ = Num$(ABS(pday(i)), 1) + " hours"
        END IF
        IF pday(i) < 0 THEN d$ = d$ + " (backwards)"
        _PRINTSTRING (x, y + 68), "One day:       " + d$
        _PRINTSTRING (x, y + 84), "Axial tilt:    " + Num$(ptilt(i), 1) + " degrees"
        m$ = pmoons$(i): IF LEN(m$) > 24 THEN m$ = LEFT$(m$, INSTR(m$, " ") - 1)
        _PRINTSTRING (x, y + 100), "Moons:         " + m$
    END IF
    IF showHelp THEN
        x = SW - 380: y = 40
        LINE (x - 8, y - 8)-(x + 360, y + 230), _RGBA32(10, 20, 50, 215), BF
        LINE (x - 8, y - 8)-(x + 360, y + 230), _RGB32(90, 120, 190), B
        COLOR _RGB32(255, 230, 120): _PRINTSTRING (x, y), "Keys"
        COLOR _RGB32(220, 225, 235)
        _PRINTSTRING (x, y + 22), "Drag mouse, arrows    turn the camera"
        _PRINTSTRING (x, y + 38), "Wheel, PgUp / PgDn    closer / farther"
        _PRINTSTRING (x, y + 54), "1 - 8                 follow a planet"
        _PRINTSTRING (x, y + 70), "0                     whole system"
        _PRINTSTRING (x, y + 86), "+ / -                 faster / slower"
        _PRINTSTRING (x, y + 102), "Space                 pause"
        _PRINTSTRING (x, y + 118), "R                     backwards"
        _PRINTSTRING (x, y + 134), "T                     back to today"
        _PRINTSTRING (x, y + 150), "N / O / I             names / orbits / info"
        _PRINTSTRING (x, y + 166), "Esc                   quit"
        COLOR _RGB32(140, 150, 175)
        _PRINTSTRING (x, y + 194), "Distances and sizes are squeezed,"
        _PRINTSTRING (x, y + 210), "periods and positions are real."
    END IF
END SUB

' number with a decimal point and thousands separators
FUNCTION Num$ (v, nk)
    f = 10 ^ nk
    n# = INT(ABS(v) * f + 0.5)
    whole# = INT(n# / f): rest# = n# - whole# * f
    g$ = LTRIM$(STR$(whole#))
    IF INSTR(g$, "E") > 0 OR INSTR(g$, "D") > 0 THEN g$ = LTRIM$(STR$(CLNG(whole#)))
    o$ = ""
    DO WHILE LEN(g$) > 3
        o$ = "," + RIGHT$(g$, 3) + o$: g$ = LEFT$(g$, LEN(g$) - 3)
    LOOP
    o$ = g$ + o$
    IF nk > 0 THEN o$ = o$ + "." + RIGHT$(STRING$(nk, "0") + LTRIM$(STR$(rest#)), nk)
    IF v < 0 THEN o$ = "-" + o$
    Num$ = o$
END FUNCTION
