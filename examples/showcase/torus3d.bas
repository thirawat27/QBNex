' ============================================================================
'  torus3d.bas  -  the classic QBasic TORUS demo, in real 3D for QBNex 2.0
'                  (preview.4 or newer, needs the GPU 3D layer)
'
'  Like Microsoft's TORUS.BAS it builds a torus out of coloured tiles and
'  rotates the colours like the old palette trick. But here the torus is a
'  real 3D mesh: it spins, is lit by a lamp and can be turned with the keys.
'  The colour cycling and the tile borders are done by a small GPU shader.
'
'  Menu:  Up/Down = choose   Left/Right = change   Enter = draw   Esc = quit
'  Torus: Arrows = tilt   +/- = spin speed   < > = colour speed
'         B = tile border   Space = pause   Esc = back to the menu
'
'  Written by Gokipp (https://github.com/Gokipp).
' ============================================================================
CONST PI = 3.14159265
DIM SHARED SW, SH
SW = 960: SH = 640
SCREEN _NEWIMAGE(SW, SH, 32)
_TITLE "Torus 3D"
_PRINTMODE _KEEPBACKGROUND

IF _GPU3D = 0 THEN
    PRINT "No GPU for 3D found - TORUS3D needs QBNex 2.0 (preview.4 or newer)."
    PRINT "Press a key ...": SLEEP: SYSTEM
END IF

' Menu values (the defaults are the ones of the old TORUS.BAS)
CONST NOPT = 7
DIM SHARED optName$(NOPT), optVal(NOPT), optMin(NOPT), optMax(NOPT), optStep(NOPT)
RESTORE menudata
FOR i = 1 TO NOPT
    READ optName$(i), optVal(i), optMin(i), optMax(i), optStep(i)
NEXT
DIM SHARED shTorus&, torus&, lamp&

menudata:
DATA "Thickness",3,1,9,1
DATA "Panels per section",8,3,30,1
DATA "Sections per torus",14,4,60,1
DATA "Tilt around horizontal axis",30,0,90,5
DATA "Tilt around vertical axis",30,0,90,5
DATA "Tile border (1 = yes, 0 = no)",1,0,1,1
DATA "Number of colours",15,2,15,1

MakeShader
lamp& = _LIGHT(1, -0.6, -1, -0.8, _RGB32(255, 255, 255), 1.1)
_AMBIENT _RGB32(255, 255, 255), 0.25

DO
    IF Menu = 0 THEN EXIT DO
    ShowTorus
LOOP
SYSTEM

' ---------------------------------------------------------------------------
FUNCTION Menu
    STATIC sel
    IF sel = 0 THEN sel = 1
    DO
        _LIMIT 30
        LINE (0, 0)-(SW - 1, SH - 1), _RGB32(0, 0, 40), BF
        x = SW \ 2 - 240: y = 120
        LINE (x - 20, y - 70)-(x + 500, y + 330), _RGB32(170, 170, 170), B
        LINE (x - 16, y - 66)-(x + 496, y + 326), _RGB32(170, 170, 170), B
        COLOR _RGB32(255, 255, 85)
        _PRINTSTRING (SW \ 2 - 4 * 16, y - 44), "T O R U S   3 D"
        FOR i = 1 TO NOPT
            yy = y + (i - 1) * 32
            IF i = sel THEN
                LINE (x - 6, yy - 6)-(x + 486, yy + 20), _RGB32(0, 0, 170), BF
                COLOR _RGB32(255, 255, 255)
            ELSE
                COLOR _RGB32(170, 170, 170)
            END IF
            _PRINTSTRING (x, yy), optName$(i)
            v$ = LTRIM$(STR$(optVal(i)))
            IF i = 6 THEN IF optVal(i) THEN v$ = "YES" ELSE v$ = "NO"
            _PRINTSTRING (x + 400, yy), "<" + SPACE$(4 - LEN(v$)) + v$ + " >"
        NEXT
        COLOR _RGB32(85, 255, 255)
        _PRINTSTRING (x, y + 250), "Up/Down = choose, Left/Right = change"
        _PRINTSTRING (x, y + 270), "Enter = draw the torus,  Esc = quit"
        COLOR _RGB32(120, 120, 160)
        _PRINTSTRING (x, y + 300), "After the old Microsoft QBasic TORUS.BAS demo"
        _CLEAR3D 0
        _DISPLAY
        k$ = INKEY$
        SELECT CASE k$
            CASE CHR$(27): Menu = 0: EXIT FUNCTION
            CASE CHR$(13): Menu = -1: EXIT FUNCTION
            CASE CHR$(0) + "H": sel = sel - 1: IF sel < 1 THEN sel = NOPT
            CASE CHR$(0) + "P": sel = sel + 1: IF sel > NOPT THEN sel = 1
            CASE CHR$(0) + "K": optVal(sel) = optVal(sel) - optStep(sel)
            CASE CHR$(0) + "M": optVal(sel) = optVal(sel) + optStep(sel)
        END SELECT
        IF optVal(sel) < optMin(sel) THEN optVal(sel) = optMin(sel)
        IF optVal(sel) > optMax(sel) THEN optVal(sel) = optMax(sel)
    LOOP
END FUNCTION

' ---------------------------------------------------------------------------
' The torus: every tile has its own four corners, so the tiles stay flat
' (faceted like in the old demo). u runs around the big ring, v around the
' tube; the shader finds the tile number from u and v.
FUNCTION MakeTorus& (thick, panels, sections)
    rBig = 1: rTube = 0.07 + thick * 0.055
    _MESHBEGIN
    n& = 0
    FOR i = 0 TO sections - 1
        a0 = i / sections * 2 * PI: a1 = (i + 1) / sections * 2 * PI
        FOR j = 0 TO panels - 1
            b0 = j / panels * 2 * PI: b1 = (j + 1) / panels * 2 * PI
            TorusVertex rBig, rTube, a0, b0, i / sections, j / panels
            TorusVertex rBig, rTube, a1, b0, (i + 1) / sections, j / panels
            TorusVertex rBig, rTube, a1, b1, (i + 1) / sections, (j + 1) / panels
            TorusVertex rBig, rTube, a0, b1, i / sections, (j + 1) / panels
            _MESHTRIANGLE n&, n& + 2, n& + 1
            _MESHTRIANGLE n&, n& + 3, n& + 2
            n& = n& + 4
        NEXT
    NEXT
    m& = _MESHEND
    _MESHCOLOR m&, _RGB32(255, 255, 255)
    _MESHSHADER m&, shTorus&
    MakeTorus& = m&
END FUNCTION

SUB TorusVertex (rBig, rTube, a, b, u, v)
    d = rBig + rTube * COS(b)
    _MESHVERTEX d * COS(a), rTube * SIN(b), d * SIN(a), u, v
END SUB

SUB MakeShader
    nl$ = CHR$(10)
    ' params: 0 = sections, 1 = panels, 2 = colour shift, 3 = border on/off,
    '         4 = number of colours
    s$ = "fn shade(i: ShadeIn) -> vec4<f32> {" + nl$
    s$ = s$ + "    var pal = array<vec3<f32>, 15>(" + nl$
    s$ = s$ + "        vec3<f32>(0.0, 0.0, 0.67), vec3<f32>(0.0, 0.67, 0.0), vec3<f32>(0.0, 0.67, 0.67)," + nl$
    s$ = s$ + "        vec3<f32>(0.67, 0.0, 0.0), vec3<f32>(0.67, 0.0, 0.67), vec3<f32>(0.67, 0.33, 0.0)," + nl$
    s$ = s$ + "        vec3<f32>(0.67, 0.67, 0.67), vec3<f32>(0.33, 0.33, 0.33), vec3<f32>(0.33, 0.33, 1.0)," + nl$
    s$ = s$ + "        vec3<f32>(0.33, 1.0, 0.33), vec3<f32>(0.33, 1.0, 1.0), vec3<f32>(1.0, 0.33, 0.33)," + nl$
    s$ = s$ + "        vec3<f32>(1.0, 0.33, 1.0), vec3<f32>(1.0, 1.0, 0.33), vec3<f32>(1.0, 1.0, 1.0));" + nl$
    s$ = s$ + "    let su = i.uv.x * i.params[0].x;" + nl$
    s$ = s$ + "    let sv = i.uv.y * i.params[0].y;" + nl$
    s$ = s$ + "    let n = max(i.params[1].x, 1.0);" + nl$
    s$ = s$ + "    let tile = floor(su) + floor(sv) + floor(i.params[0].z);" + nl$
    s$ = s$ + "    let k = i32(tile - n * floor(tile / n));" + nl$
    s$ = s$ + "    var c = pal[clamp(k, 0, 14)];" + nl$
    s$ = s$ + "    let fu = fract(su); let fv = fract(sv);" + nl$
    s$ = s$ + "    let edge = min(min(fu, 1.0 - fu) / max(fwidth(su), 0.0001), min(fv, 1.0 - fv) / max(fwidth(sv), 0.0001));" + nl$
    s$ = s$ + "    if (i.params[0].w > 0.5 && edge < 1.2) { c = vec3<f32>(0.05, 0.05, 0.08); }" + nl$
    s$ = s$ + "    return vec4<f32>(c * i.light, 1.0);" + nl$
    s$ = s$ + "}"
    shTorus& = _SHADER(s$)
    IF shTorus& = -1 THEN
        PRINT "Shader error: "; _SHADERERROR$
        _DISPLAY: SLEEP: SYSTEM
    END IF
END SUB

' ---------------------------------------------------------------------------
SUB ShowTorus
    IF torus& THEN _FREEMESH torus&
    torus& = MakeTorus&(optVal(1), optVal(2), optVal(3))
    _SHADERPARAM shTorus&, 0, optVal(3)
    _SHADERPARAM shTorus&, 1, optVal(2)
    _SHADERPARAM shTorus&, 3, optVal(6)
    _SHADERPARAM shTorus&, 4, optVal(7)
    tiltX = optVal(4): tiltZ = optVal(5)
    spin = 0: spinSpeed = 30: colSpeed = 6: shift = 0: paused = 0
    border = optVal(6)
    _CAMERA (0, 0, 3.6)TO(0, 0, 0), 45
    last# = TIMER(.001)
    DO
        _LIMIT 60
        now# = TIMER(.001): dt = now# - last#: last# = now#
        IF dt < 0 OR dt > 0.5 THEN dt = 1 / 60
        k$ = INKEY$
        SELECT CASE UCASE$(k$)
            CASE CHR$(27): EXIT DO
            CASE " ": paused = NOT paused
            CASE "+", "=": spinSpeed = spinSpeed + 10
            CASE "-", "_": spinSpeed = spinSpeed - 10
            CASE ">", ".": colSpeed = colSpeed + 2
            CASE "<", ",": colSpeed = colSpeed - 2
            CASE "B": border = 1 - border: _SHADERPARAM shTorus&, 3, border
        END SELECT
        IF _KEYDOWN(18432) THEN tiltX = tiltX - 60 * dt
        IF _KEYDOWN(20480) THEN tiltX = tiltX + 60 * dt
        IF _KEYDOWN(19200) THEN tiltZ = tiltZ + 60 * dt
        IF _KEYDOWN(19712) THEN tiltZ = tiltZ - 60 * dt
        IF NOT paused THEN
            spin = spin + spinSpeed * dt
            shift = shift + colSpeed * dt
        END IF
        _SHADERPARAM shTorus&, 2, shift

        ' background like the old demo: dark blue with a frame
        LINE (0, 0)-(SW - 1, SH - 1), _RGB32(0, 0, 0), BF
        COLOR _RGB32(120, 120, 170)
        t$ = "Arrows = tilt   +/- = spin   < > = colours   B = border   Space = pause   Esc = menu"
        _PRINTSTRING ((SW - LEN(t$) * 8) \ 2, SH - 24), t$
        COLOR _RGB32(255, 255, 85)
        _PRINTSTRING (16, 12), "TORUS 3D   Panels:" + STR$(optVal(2)) + "   Sections:" + STR$(optVal(3)) + "   Tiles:" + STR$(optVal(2) * optVal(3))

        _CLEAR3D 0
        _DRAW3D torus&, 0, 0, 0, tiltX, spin, tiltZ
        _DISPLAY
    LOOP
    _KEYCLEAR
END SUB
