' Ray tracer on the GPU: six animated scenes traced live, every pixel of every
' frame, by a WGSL shader (_SHADER) that the BASIC program feeds with the
' camera and the clock.
'   1 Bouncing spheres   a mirror ball, eight bouncing balls, gold moons, glass marbles
'   2 Glass              a big glass sphere bending the balls and boxes around it
'   3 Newton's cradle    five chrome balls swinging on strings in a chrome frame
'   4 Light show         red, green and blue lamps circling white shapes: coloured shadows
'   5 Double helix       32 spheres and 16 chrome rungs turning over a mirror floor at dusk
'   6 Sphere wave        64 balls riding a ripple under a floating glass sphere
' Every ray bounces up to six times: mirror reflections, refraction through glass
' with Fresnel reflection, soft shadows from the sun and from point lamps, and
' four rays per pixel for anti-aliasing.
' How it works: a cube is drawn around the camera with _DRAW3D, so every pixel
' of the window runs the shader once; the shader takes the ray from the camera
' through that pixel and traces the whole scene itself.
' Nothing is written on the picture: the scene name and the frame rate are in
' the window title.
' Keys: 1-6 or N = scene, A = automatic tour, Q = anti-aliasing on/off,
' Space = pause, Esc = quit.
' Command line: SCENE=k starts with scene k, TIME=t holds the picture at t
' seconds (for screenshots), AA=0 starts without anti-aliasing, NOLIMIT drops
' the 60 fps cap.
CONST PI = 3.14159265
CONST SCENES = 6
CONST SW = 960, SH = 600

SCREEN _NEWIMAGE(SW, SH, 32)
_TITLE "QBNex GPU ray tracer"
IF _GPU3D = 0 THEN
    COLOR _RGB32(255, 200, 140)
    _PRINTSTRING (SW \ 2 - 180, SH \ 2), "No GPU adapter: the ray tracer needs one."
    SLEEP
    SYSTEM
END IF

DIM SHARED scenename$(SCENES)
scenename$(1) = "Bouncing spheres"
scenename$(2) = "Glass"
scenename$(3) = "Newton's cradle"
scenename$(4) = "Light show"
scenename$(5) = "Double helix"
scenename$(6) = "Sphere wave"

' the tracer: WGSL source, one line per DATA string
nl$ = CHR$(10)
src$ = ""
RESTORE tracer
DO
    READ l$
    IF l$ = "~" THEN EXIT DO
    src$ = src$ + l$ + nl$
LOOP
tracer& = _SHADER(src$)
IF tracer& = -1 THEN
    COLOR _RGB32(255, 160, 120)
    PRINT "shader error: "; _SHADERERROR$
    SLEEP
    SYSTEM
END IF
dome& = _MESHCUBE(1)
_MESHSHADER dome&, tracer&
_CAMERARANGE 0.1, 100

cmd$ = UCASE$(COMMAND$)
scene% = 1
p = INSTR(cmd$, "SCENE="): IF p THEN scene% = VAL(MID$(cmd$, p + 6))
IF scene% < 1 OR scene% > SCENES THEN scene% = 1
hold = -1
q = INSTR(cmd$, "TIME="): IF q THEN hold = VAL(MID$(cmd$, q + 5))
tour% = -1: IF p OR q THEN tour% = 0
aa% = -1: IF INSTR(cmd$, "AA=0") THEN aa% = 0
nolimit% = INSTR(cmd$, "NOLIMIT") > 0
paused% = 0

tsim = 0: IF hold >= 0 THEN tsim = hold
shown = 0: frames& = 0
last# = TIMER(.001): fpsclock# = last#
SetTitle scene%, 0
DO
    IF nolimit% = 0 THEN _LIMIT 60
    k$ = UCASE$(INKEY$)
    IF k$ = CHR$(27) THEN EXIT DO
    was% = scene%
    IF k$ = "A" THEN tour% = NOT tour%
    IF k$ = "Q" THEN aa% = NOT aa%
    IF k$ = " " THEN paused% = NOT paused%
    IF k$ = "N" THEN scene% = scene% MOD SCENES + 1
    IF k$ >= "1" AND k$ <= CHR$(48 + SCENES) THEN scene% = VAL(k$): tour% = 0
    IF tour% AND shown >= 15 THEN scene% = scene% MOD SCENES + 1
    IF scene% <> was% THEN
        shown = 0
        IF hold < 0 THEN tsim = 0
        SetTitle scene%, 0
    END IF

    ' the clock follows real time, so a slow GPU shows fewer frames, not slow motion
    now# = TIMER(.001)
    dt = now# - last#: last# = now#
    IF dt < 0 OR dt > 0.25 THEN dt = 1 / 60 ' midnight, or a stall
    shown = shown + dt
    IF hold < 0 AND paused% = 0 THEN tsim = tsim + dt
    frames& = frames& + 1
    IF now# - fpsclock# >= 1 OR now# < fpsclock# THEN
        SetTitle scene%, frames& / (now# - fpsclock# + 1E-06)
        frames& = 0: fpsclock# = now#
    END IF

    Camera scene%, tsim, ex, ey, ez, tx, ty, tz, fov
    _CAMERA (ex, ey, ez) TO (tx, ty, tz), fov
    _SHADERPARAM tracer&, 0, ex: _SHADERPARAM tracer&, 1, ey: _SHADERPARAM tracer&, 2, ez
    _SHADERPARAM tracer&, 3, tsim
    _SHADERPARAM tracer&, 7, scene%
    _SHADERPARAM tracer&, 8, -aa%
    _CLEAR3D 0
    _DRAW3D dome&, ex, ey, ez, 0, 0, 0, 20, 20, 20
    _DISPLAY
LOOP
SYSTEM

' ---- the tracer (WGSL) ---------------------------------------------------------
' params: 0-2 eye, 3 time, 7 scene, 8 anti-aliasing on (1) or off (0)
tracer:
DATA "const TAU = 6.2831853;"
DATA "const FAR = 1e4;"
DATA "const MAXS = 72;"
DATA "const MAXB = 8;"
DATA "const MAXR = 24;"
DATA ""
DATA "// the scene, rebuilt for each pixel from the clock"
DATA "var<private> sp: array<vec4f, MAXS>;  // spheres: centre, radius"
DATA "var<private> sm: array<vec4f, MAXS>;  //   colour, material"
DATA "var<private> ns: i32;"
DATA "var<private> bp: array<vec4f, MAXB>;  // boxes: centre, turn around y"
DATA "var<private> bs: array<vec4f, MAXB>;  //   half size, tilt around x"
DATA "var<private> bm: array<vec4f, MAXB>;  //   colour, material"
DATA "var<private> nb: i32;"
DATA "var<private> ra: array<vec4f, MAXR>;  // rods (capsules): one end, radius"
DATA "var<private> rb: array<vec4f, MAXR>;  //   other end"
DATA "var<private> rm: array<vec4f, MAXR>;  //   colour, material"
DATA "var<private> nr: i32;"
DATA "var<private> lp: array<vec4f, 3>;     // lights: direction (w 0) or position (w 1)"
DATA "var<private> lc: array<vec4f, 3>;     //   colour, size (how soft the shadows are)"
DATA "var<private> nl: i32;"
DATA "var<private> bnd: vec4f;              // a sphere around all spheres (radius 0: none)"
DATA "var<private> skyT: vec3f;"
DATA "var<private> skyH: vec3f;"
DATA "var<private> flA: vec3f;"
DATA "var<private> flB: vec3f;"
DATA "var<private> flR: f32;"
DATA "var<private> amb: vec3f;"
DATA ""
DATA "// materials: 0 plastic, 1 metal, 2 glass, 3 lamp, 4 the floor"
DATA "struct Hit { t: f32, n: vec3f, col: vec3f, mat: f32, kind: i32, idx: i32 };"
DATA ""
DATA "fn srgb(c: vec3f) -> vec3f { return pow(c, vec3f(2.2)); }"
DATA "fn hue(h: f32) -> vec3f {"
DATA "    return srgb(0.1 + 0.9 * (0.5 + 0.5 * cos(TAU * (h + vec3f(0.0, -0.3333, -0.6667)))));"
DATA "}"
DATA "fn ball(c: vec3f, r: f32, col: vec3f, m: f32) {"
DATA "    if (ns < MAXS) { sp[ns] = vec4f(c, r); sm[ns] = vec4f(col, m); ns++; }"
DATA "}"
DATA "fn block(c: vec3f, h: vec3f, ay: f32, ax: f32, col: vec3f, m: f32) {"
DATA "    if (nb < MAXB) { bp[nb] = vec4f(c, ay); bs[nb] = vec4f(h, ax); bm[nb] = vec4f(col, m); nb++; }"
DATA "}"
DATA "fn rod(a: vec3f, b: vec3f, r: f32, col: vec3f, m: f32) {"
DATA "    if (nr < MAXR) { ra[nr] = vec4f(a, r); rb[nr] = vec4f(b, 0.0); rm[nr] = vec4f(col, m); nr++; }"
DATA "}"
DATA "fn sun(d: vec3f, col: vec3f, size: f32) {"
DATA "    if (nl < 3) { lp[nl] = vec4f(normalize(d), 0.0); lc[nl] = vec4f(col, size); nl++; }"
DATA "}"
DATA "fn lamp(p: vec3f, col: vec3f, size: f32) {"
DATA "    if (nl < 3) { lp[nl] = vec4f(p, 1.0); lc[nl] = vec4f(col, size); nl++; }"
DATA "}"
DATA "fn rotY(v: vec3f, a: f32) -> vec3f {"
DATA "    let c = cos(a); let s = sin(a);"
DATA "    return vec3f(c * v.x + s * v.z, v.y, c * v.z - s * v.x);"
DATA "}"
DATA "fn rotX(v: vec3f, a: f32) -> vec3f {"
DATA "    let c = cos(a); let s = sin(a);"
DATA "    return vec3f(v.x, c * v.y - s * v.z, s * v.y + c * v.z);"
DATA "}"
DATA ""
DATA "// ---- scenes ----"
DATA "fn scBounce(t: f32) {"
DATA "    sun(vec3f(-0.55, 0.75, -0.4), vec3f(1.1, 1.06, 1.0), 0.05);"
DATA "    ball(vec3f(0.0, 1.3, 0.0), 1.3, srgb(vec3f(0.92, 0.93, 0.96)), 1.0);"
DATA "    for (var i = 0; i < 8; i++) {"
DATA "        let f = f32(i);"
DATA "        let a = f * TAU / 8.0 + t * 0.3;"
DATA "        let u = fract(t * 0.8 + f / 8.0);"
DATA "        ball(vec3f(3.4 * cos(a), 0.5 + 9.6 * u * (1.0 - u), 3.4 * sin(a)), 0.5, hue(f / 8.0), 0.0);"
DATA "    }"
DATA "    for (var i = 0; i < 3; i++) {"
DATA "        let f = f32(i);"
DATA "        let a = t * 1.1 + f * TAU / 3.0;"
DATA "        ball(vec3f(2.05 * cos(a), 1.3 + 0.7 * sin(a + f), 2.05 * sin(a)), 0.22, srgb(vec3f(1.0, 0.78, 0.34)), 1.0);"
DATA "    }"
DATA "    for (var i = 0; i < 2; i++) {"
DATA "        let a = -t * 0.25 + f32(i) * 3.14159;"
DATA "        ball(vec3f(5.6 * cos(a), 0.6, 5.6 * sin(a)), 0.6, srgb(vec3f(0.85, 0.95, 1.0)), 2.0);"
DATA "    }"
DATA "}"
DATA "fn scGlass(t: f32) {"
DATA "    sun(vec3f(0.5, 0.8, -0.35), vec3f(1.1, 1.05, 0.98), 0.04);"
DATA "    flA = srgb(vec3f(0.95)); flB = srgb(vec3f(0.08, 0.08, 0.1)); flR = 0.15;"
DATA "    ball(vec3f(0.0, 1.5, 0.0), 1.5, vec3f(0.97, 0.99, 1.0), 2.0);"
DATA "    for (var i = 0; i < 7; i++) {"
DATA "        let f = f32(i);"
DATA "        let a = f * TAU / 7.0 - t * 0.45;"
DATA "        ball(vec3f(3.0 * cos(a), 0.45 + 0.6 * abs(sin(t * 2.0 + f)), 3.0 * sin(a)), 0.45, hue(f / 7.0 + 0.1), 0.0);"
DATA "    }"
DATA "    for (var i = 0; i < 3; i++) {"
DATA "        let f = f32(i);"
DATA "        let a = f * TAU / 3.0 + t * 0.3;"
DATA "        ball(vec3f(4.7 * cos(a), 0.75, 4.7 * sin(a)), 0.75, mix(vec3f(1.0), hue(f / 3.0 + 0.5), 0.45), 2.0);"
DATA "    }"
DATA "    for (var i = 0; i < 4; i++) {"
DATA "        let f = f32(i);"
DATA "        let a = f * TAU / 4.0 - t * 0.12;"
DATA "        block(vec3f(6.8 * cos(a), 0.55, 6.8 * sin(a)), vec3f(0.55), t * 0.8 + f, 0.0, hue(f / 4.0 + 0.05), 0.0);"
DATA "    }"
DATA "}"
DATA "fn scCradle(t: f32) {"
DATA "    sun(vec3f(-0.4, 0.8, 0.45), vec3f(1.05, 1.0, 0.95), 0.06);"
DATA "    lamp(vec3f(3.5, 4.5, 3.0), vec3f(5.0, 3.6, 2.2), 0.1);"
DATA "    skyT = srgb(vec3f(0.2, 0.22, 0.3)); skyH = srgb(vec3f(0.62, 0.58, 0.58));"
DATA "    flA = srgb(vec3f(0.85)); flB = srgb(vec3f(0.25, 0.26, 0.3)); flR = 0.25;"
DATA "    let top = 3.2;"
DATA "    let len = 2.15;"
DATA "    let th = 0.62 * sin(t * TAU / 1.7);"
DATA "    for (var i = 0; i < 5; i++) {"
DATA "        var a = 0.0;"
DATA "        if (i == 0) { a = min(th, 0.0); }"
DATA "        if (i == 4) { a = max(th, 0.0); }"
DATA "        let x = (f32(i) - 2.0) * 0.8;"
DATA "        let c = vec3f(x + len * sin(a), top - len * cos(a), 0.0);"
DATA "        ball(c, 0.4, srgb(vec3f(0.9, 0.92, 0.95)), 1.0);"
DATA "        rod(vec3f(x, top, 0.75), c, 0.015, vec3f(0.5), 0.0);"
DATA "        rod(vec3f(x, top, -0.75), c, 0.015, vec3f(0.5), 0.0);"
DATA "    }"
DATA "    let chrome = srgb(vec3f(0.85, 0.86, 0.9));"
DATA "    for (var j = 0; j < 2; j++) {"
DATA "        let z = select(-0.75, 0.75, j == 1);"
DATA "        rod(vec3f(-2.6, top, z), vec3f(2.6, top, z), 0.05, chrome, 1.0);"
DATA "        rod(vec3f(-2.6, top, z), vec3f(-2.6, 0.24, z), 0.05, chrome, 1.0);"
DATA "        rod(vec3f(2.6, top, z), vec3f(2.6, 0.24, z), 0.05, chrome, 1.0);"
DATA "    }"
DATA "    block(vec3f(0.0, 0.12, 0.0), vec3f(3.0, 0.12, 1.15), 0.0, 0.0, srgb(vec3f(0.06, 0.06, 0.07)), 0.0);"
DATA "    ball(vec3f(-3.8, 0.5, 2.0), 0.5, hue(0.0), 0.0);"
DATA "    ball(vec3f(3.9, 0.7, 1.6), 0.7, hue(0.6), 0.0);"
DATA "    ball(vec3f(1.8, 0.35, 2.6), 0.35, vec3f(0.9, 1.0, 0.95), 2.0);"
DATA "}"
DATA "fn scLights(t: f32) {"
DATA "    skyT = srgb(vec3f(0.01, 0.01, 0.03)); skyH = srgb(vec3f(0.06, 0.05, 0.1));"
DATA "    flA = srgb(vec3f(0.85)); flB = srgb(vec3f(0.62)); flR = 0.2;"
DATA "    amb = vec3f(0.015);"
DATA "    for (var i = 0; i < 3; i++) {"
DATA "        let f = f32(i);"
DATA "        let a = t * 0.6 + f * TAU / 3.0;"
DATA "        let p = vec3f(3.3 * cos(a), 2.4 + 0.9 * sin(t * 1.3 + f * 2.0), 3.3 * sin(a));"
DATA "        var c = vec3f(1.0, 0.22, 0.12);"
DATA "        if (i == 1) { c = vec3f(0.2, 1.0, 0.25); }"
DATA "        if (i == 2) { c = vec3f(0.22, 0.35, 1.0); }"
DATA "        lamp(p, c * 5.5, 0.12);"
DATA "        ball(p, 0.12, c, 3.0);"
DATA "    }"
DATA "    let white = srgb(vec3f(0.92));"
DATA "    block(vec3f(0.0, 1.5, 0.0), vec3f(0.75), t * 0.7, 0.6 * sin(t * 0.45), white, 0.0);"
DATA "    for (var i = 0; i < 6; i++) {"
DATA "        let a = f32(i) * TAU / 6.0 + 0.3;"
DATA "        ball(vec3f(2.0 * cos(a), 0.45, 2.0 * sin(a)), 0.45, white, 0.0);"
DATA "    }"
DATA "    for (var i = 0; i < 4; i++) {"
DATA "        let a = f32(i) * TAU / 4.0 + t * 0.15 + 0.785;"
DATA "        block(vec3f(5.0 * cos(a), 1.2, 5.0 * sin(a)), vec3f(0.3, 1.2, 0.3), -a, 0.0, white, 0.0);"
DATA "    }"
DATA "}"
DATA "fn scHelix(t: f32) {"
DATA "    skyT = srgb(vec3f(0.03, 0.04, 0.16)); skyH = srgb(vec3f(0.95, 0.5, 0.32));"
DATA "    sun(vec3f(0.4, 0.5, 0.75), srgb(vec3f(1.0, 0.85, 0.68)) * 1.4, 0.05);"
DATA "    amb = vec3f(0.06, 0.05, 0.1);"
DATA "    flA = srgb(vec3f(0.3, 0.3, 0.36)); flB = srgb(vec3f(0.06, 0.06, 0.09)); flR = 0.45;"
DATA "    let rr = 1.2 + 0.25 * sin(t * 0.8);"
DATA "    for (var k = 0; k < 16; k++) {"
DATA "        let f = f32(k);"
DATA "        let y = 0.7 + f * 0.27;"
DATA "        let a = f * 0.42 + t * 0.9;"
DATA "        let pa = vec3f(rr * cos(a), y, rr * sin(a));"
DATA "        let pb = vec3f(-rr * cos(a), y, -rr * sin(a));"
DATA "        ball(pa, 0.22, hue(0.55 + f / 40.0), 0.0);"
DATA "        ball(pb, 0.22, hue(0.02 + f / 40.0), 0.0);"
DATA "        rod(pa, pb, 0.05, srgb(vec3f(0.9, 0.9, 0.95)), 1.0);"
DATA "    }"
DATA "    ball(vec3f(0.0, 5.9 + 0.15 * sin(t * 2.0), 0.0), 0.5, srgb(vec3f(1.0, 0.78, 0.34)), 1.0);"
DATA "    bnd = vec4f(0.0, 3.45, 0.0, 3.7);"
DATA "}"
DATA "fn scWave(t: f32) {"
DATA "    sun(vec3f(-0.5, 0.8, -0.3), vec3f(1.1, 1.06, 1.0), 0.05);"
DATA "    for (var ix = 0; ix < 8; ix++) {"
DATA "        for (var iz = 0; iz < 8; iz++) {"
DATA "            let x = (f32(ix) - 3.5) * 0.9;"
DATA "            let z = (f32(iz) - 3.5) * 0.9;"
DATA "            let d = sqrt(x * x + z * z);"
DATA "            let y = 1.0 + 0.6 * sin(d * 1.7 - t * 2.6);"
DATA "            ball(vec3f(x, y, z), 0.36, hue(0.58 - 0.13 * d + 0.15 * sin(t * 0.5)), f32((ix + iz) % 2));"
DATA "        }"
DATA "    }"
DATA "    ball(vec3f(0.0, 3.4 + 0.25 * sin(t * 1.3), 0.0), 0.95, vec3f(0.96, 0.98, 1.0), 2.0);"
DATA "    bnd = vec4f(0.0, 1.9, 0.0, 5.4);"
DATA "}"
DATA "fn build(s: i32, t: f32) {"
DATA "    ns = 0; nb = 0; nr = 0; nl = 0;"
DATA "    bnd = vec4f(0.0);"
DATA "    skyT = srgb(vec3f(0.24, 0.47, 1.0)); skyH = srgb(vec3f(0.98, 0.93, 1.0));"
DATA "    flA = srgb(vec3f(0.92)); flB = srgb(vec3f(0.16, 0.18, 0.24)); flR = 0.3;"
DATA "    amb = vec3f(0.1, 0.11, 0.14);"
DATA "    switch s {"
DATA "        case 2: { scGlass(t); }"
DATA "        case 3: { scCradle(t); }"
DATA "        case 4: { scLights(t); }"
DATA "        case 5: { scHelix(t); }"
DATA "        case 6: { scWave(t); }"
DATA "        default: { scBounce(t); }"
DATA "    }"
DATA "}"
DATA ""
DATA "// ---- intersections ----"
DATA "// distances squared are taken from the point nearest the centre, not as"
DATA "// b*b - c, which loses the small spheres far away to rounding"
DATA "fn hitSph(ro: vec3f, rd: vec3f, s: vec4f) -> f32 {"
DATA "    let oc = ro - s.xyz;"
DATA "    let b = dot(oc, rd);"
DATA "    let q = oc - b * rd;"
DATA "    let h = s.w * s.w - dot(q, q);"
DATA "    if (h < 0.0) { return -1.0; }"
DATA "    return -b - sqrt(h);"
DATA "}"
DATA "// can the ray reach any sphere at all?"
DATA "fn inBound(ro: vec3f, rd: vec3f) -> bool {"
DATA "    if (bnd.w <= 0.0) { return true; }"
DATA "    let oc = ro - bnd.xyz;"
DATA "    let b = dot(oc, rd);"
DATA "    let c = dot(oc, oc) - bnd.w * bnd.w;"
DATA "    return c < 0.0 || (b < 0.0 && b * b > c);"
DATA "}"
DATA "fn hitBox(ro: vec3f, rd: vec3f, k: i32, nrm: ptr<function, vec3f>) -> f32 {"
DATA "    let o = rotX(rotY(ro - bp[k].xyz, -bp[k].w), -bs[k].w);"
DATA "    var d = rotX(rotY(rd, -bp[k].w), -bs[k].w);"
DATA "    d = select(d, vec3f(1e-7), abs(d) < vec3f(1e-7));"
DATA "    let m = 1.0 / d;"
DATA "    let n = m * o;"
DATA "    let kk = abs(m) * bs[k].xyz;"
DATA "    let t1 = -n - kk;"
DATA "    let t2 = -n + kk;"
DATA "    let tn = max(max(t1.x, t1.y), t1.z);"
DATA "    let tf = min(min(t2.x, t2.y), t2.z);"
DATA "    if (tn > tf || tn < 1e-4) { return -1.0; }"
DATA "    let ln = -sign(d) * step(t1.yzx, t1.xyz) * step(t1.zxy, t1.xyz);"
DATA "    *nrm = rotY(rotX(ln, bs[k].w), bp[k].w);"
DATA "    return tn;"
DATA "}"
DATA "// a rod: a cylinder with round ends; worked across the axis so thin, long rods stay exact"
DATA "fn hitRod(ro: vec3f, rd: vec3f, k: i32) -> f32 {"
DATA "    let pa = ra[k].xyz; let pb = rb[k].xyz; let r = ra[k].w;"
DATA "    let len = length(pb - pa);"
DATA "    let u = (pb - pa) / len;"
DATA "    let oa = ro - pa;"
DATA "    let y0 = dot(oa, u);"
DATA "    let ry = dot(rd, u);"
DATA "    let w = oa - y0 * u;"
DATA "    let v = rd - ry * u;"
DATA "    let a = dot(v, v);"
DATA "    if (a > 1e-7) {"
DATA "        let tc = -dot(w, v) / a;"
DATA "        let q = w + tc * v;"
DATA "        let h = r * r - dot(q, q);"
DATA "        if (h < 0.0) { return -1.0; }"
DATA "        let t = tc - sqrt(h / a);"
DATA "        let y = y0 + t * ry;"
DATA "        if (y > 0.0 && y < len && t > 1e-4) { return t; }"
DATA "    }"
DATA "    var best = -1.0;"
DATA "    let t0 = hitSph(ro, rd, vec4f(pa, r));"
DATA "    if (t0 > 1e-4) { best = t0; }"
DATA "    let t1 = hitSph(ro, rd, vec4f(pb, r));"
DATA "    if (t1 > 1e-4 && (best < 0.0 || t1 < best)) { best = t1; }"
DATA "    return best;"
DATA "}"
DATA "fn rodNormal(p: vec3f, k: i32) -> vec3f {"
DATA "    let ba = rb[k].xyz - ra[k].xyz; let pa = p - ra[k].xyz;"
DATA "    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);"
DATA "    return (pa - h * ba) / ra[k].w;"
DATA "}"
DATA "fn checker(p: vec3f, t: f32) -> vec3f {"
DATA "    let q = floor(p.xz);"
DATA "    let c = q.x + q.y;"
DATA "    let odd = c - 2.0 * floor(c * 0.5);"
DATA "    let blur = clamp(t / 35.0, 0.0, 1.0);"
DATA "    return mix(mix(flA, flB, odd), 0.5 * (flA + flB), blur * blur);"
DATA "}"
DATA "fn trace(ro: vec3f, rd: vec3f) -> Hit {"
DATA "    var h: Hit;"
DATA "    h.t = FAR; h.kind = 0; h.idx = 0; h.n = vec3f(0.0, 1.0, 0.0); h.col = vec3f(0.0); h.mat = 4.0;"
DATA "    if (rd.y < 0.0) {"
DATA "        let t = -ro.y / rd.y;"
DATA "        if (t > 1e-4) { h.t = t; h.kind = 1; }"
DATA "    }"
DATA "    if (inBound(ro, rd)) {"
DATA "        for (var i = 0; i < ns; i++) {"
DATA "            let t = hitSph(ro, rd, sp[i]);"
DATA "            if (t > 1e-4 && t < h.t) { h.t = t; h.kind = 2; h.idx = i; }"
DATA "        }"
DATA "    }"
DATA "    for (var i = 0; i < nb; i++) {"
DATA "        var nn = vec3f(0.0);"
DATA "        let t = hitBox(ro, rd, i, &nn);"
DATA "        if (t > 0.0 && t < h.t) { h.t = t; h.kind = 3; h.idx = i; h.n = nn; }"
DATA "    }"
DATA "    for (var i = 0; i < nr; i++) {"
DATA "        let t = hitRod(ro, rd, i);"
DATA "        if (t > 0.0 && t < h.t) { h.t = t; h.kind = 4; h.idx = i; }"
DATA "    }"
DATA "    let p = ro + rd * h.t;"
DATA "    switch h.kind {"
DATA "        case 1: { h.col = checker(p, h.t); }"
DATA "        case 2: { h.n = (p - sp[h.idx].xyz) / sp[h.idx].w; h.col = sm[h.idx].rgb; h.mat = sm[h.idx].w; }"
DATA "        case 3: { h.col = bm[h.idx].rgb; h.mat = bm[h.idx].w; }"
DATA "        case 4: { h.n = rodNormal(p, h.idx); h.col = rm[h.idx].rgb; h.mat = rm[h.idx].w; }"
DATA "        default: {}"
DATA "    }"
DATA "    return h;"
DATA "}"
DATA ""
DATA "// ---- light ----"
DATA "// a soft shadow from a sphere: how far the ray passes from it, over how far it went"
DATA "fn softSph(ro: vec3f, rd: vec3f, s: vec4f, tmax: f32, size: f32) -> f32 {"
DATA "    let oc = ro - s.xyz;"
DATA "    let b = dot(oc, rd);"
DATA "    let q = oc - b * rd;"
DATA "    let h = s.w * s.w - dot(q, q);"
DATA "    let d = length(q) - s.w;"
DATA "    let t = -b - sqrt(max(h, 0.0));"
DATA "    if (t < 0.0 || t > tmax) { return 1.0; }"
DATA "    return smoothstep(0.0, 1.0, d / (t * size));"
DATA "}"
DATA "fn shadow(ro: vec3f, rd: vec3f, tmax: f32, size: f32) -> f32 {"
DATA "    var s = 1.0;"
DATA "    if (inBound(ro, rd)) {"
DATA "        for (var i = 0; i < ns; i++) {"
DATA "            let m = sm[i].w;"
DATA "            if (m > 2.5) { continue; }"
DATA "            var v = softSph(ro, rd, sp[i], tmax, size);"
DATA "            if (m > 1.5) { v = mix(1.0, v, 0.4); }"
DATA "            s = min(s, v);"
DATA "        }"
DATA "    }"
DATA "    for (var i = 0; i < nb; i++) {"
DATA "        var nn = vec3f(0.0);"
DATA "        let t = hitBox(ro, rd, i, &nn);"
DATA "        if (t > 0.0 && t < tmax) { return 0.0; }"
DATA "    }"
DATA "    for (var i = 0; i < nr; i++) {"
DATA "        let t = hitRod(ro, rd, i);"
DATA "        if (t > 0.0 && t < tmax) { return 0.0; }"
DATA "    }"
DATA "    return s;"
DATA "}"
DATA "fn sky(d: vec3f) -> vec3f {"
DATA "    var c = mix(skyH, skyT, sqrt(clamp(d.y, 0.0, 1.0)));"
DATA "    for (var l = 0; l < nl; l++) {"
DATA "        if (lp[l].w < 0.5) {"
DATA "            let k = max(dot(d, lp[l].xyz), 0.0);"
DATA "            c += lc[l].rgb * (0.12 * pow(k, 16.0) + 0.8 * pow(k, 600.0));"
DATA "        }"
DATA "    }"
DATA "    return c;"
DATA "}"
DATA "// sun and lamps at point p: Lambert, Blinn-Phong highlight, soft shadows"
DATA "fn direct(p: vec3f, n: vec3f, rd: vec3f, col: vec3f, ks: f32, pw: f32) -> vec3f {"
DATA "    var c = amb * col * (0.75 + 0.25 * n.y);"
DATA "    for (var l = 0; l < nl; l++) {"
DATA "        var dir = lp[l].xyz;"
DATA "        var dist = FAR;"
DATA "        var att = 1.0;"
DATA "        if (lp[l].w > 0.5) {"
DATA "            let v = lp[l].xyz - p;"
DATA "            dist = length(v); dir = v / dist;"
DATA "            att = 1.0 / (1.0 + 0.12 * dist * dist);"
DATA "        }"
DATA "        let nd = dot(n, dir);"
DATA "        if (nd <= 0.0) { continue; }"
DATA "        let s = shadow(p + n * 2e-3, dir, dist, lc[l].w);"
DATA "        if (s <= 0.0) { continue; }"
DATA "        let hv = normalize(dir - rd);"
DATA "        let spec = ks * pow(max(dot(n, hv), 0.0), pw);"
DATA "        c += lc[l].rgb * (att * s) * (col * nd + vec3f(spec));"
DATA "    }"
DATA "    return c;"
DATA "}"
DATA "// what glass reflects: the sky and the floor only (cheaper than a second ray tree)"
DATA "fn env(ro: vec3f, rd: vec3f) -> vec3f {"
DATA "    if (rd.y >= 0.0) { return sky(rd); }"
DATA "    let t = -ro.y / rd.y;"
DATA "    let lit = amb + lc[0].rgb * max(lp[0].y, 0.0) * select(0.3, 1.0, lp[0].w < 0.5);"
DATA "    return mix(checker(ro + rd * t, t) * lit, skyH, smoothstep(15.0, 60.0, t));"
DATA "}"
DATA ""
DATA "// ---- one ray, up to six bounces ----"
DATA "fn render(ro0: vec3f, rd0: vec3f) -> vec3f {"
DATA "    var ro = ro0; var rd = rd0;"
DATA "    var acc = vec3f(0.0); var thr = vec3f(1.0);"
DATA "    for (var depth = 0; depth < 6; depth++) {"
DATA "        let h = trace(ro, rd);"
DATA "        if (h.kind == 0) { acc += thr * sky(rd); break; }"
DATA "        let p = ro + rd * h.t;"
DATA "        let n = h.n;"
DATA "        let fog = 1.0 - exp(-max(h.t - 18.0, 0.0) * 0.025);"
DATA "        acc += thr * fog * sky(rd);"
DATA "        thr *= 1.0 - fog;"
DATA "        let fr = pow(1.0 - max(-dot(rd, n), 0.0), 5.0);"
DATA "        if (h.mat > 2.5 && h.mat < 3.5) { acc += thr * h.col * 3.0; break; }"
DATA "        if (h.mat > 1.5 && h.mat < 2.5) {"
DATA "            // glass sphere: reflect a little, refract in, cross, refract out"
DATA "            let s = sp[h.idx];"
DATA "            let f = 0.04 + 0.96 * fr;"
DATA "            acc += thr * f * env(p, reflect(rd, n));"
DATA "            var q = p;"
DATA "            var d = refract(rd, n, 1.0 / 1.5);"
DATA "            var tint = vec3f(1.0);"
DATA "            for (var k = 0; k < 3; k++) {"
DATA "                let oc = q - s.xyz;"
DATA "                let b = dot(oc, d);"
DATA "                let tt = -b + sqrt(max(b * b - dot(oc, oc) + s.w * s.w, 0.0));"
DATA "                q += d * tt;"
DATA "                tint *= exp(-(1.0 - h.col) * tt * 0.9);"
DATA "                let nn = (q - s.xyz) / s.w;"
DATA "                let o = refract(d, -nn, 1.5);"
DATA "                if (dot(o, o) > 0.0) { d = o; break; }"
DATA "                d = reflect(d, -nn);"
DATA "            }"
DATA "            thr *= (1.0 - f) * tint;"
DATA "            ro = q + d * 2e-3; rd = d;"
DATA "            continue;"
DATA "        }"
DATA "        if (h.mat > 0.5 && h.mat < 1.5) {"
DATA "            acc += thr * direct(p, n, rd, h.col * 0.12, 1.2, 250.0);"
DATA "            thr *= mix(h.col, vec3f(1.0), fr) * 0.85;"
DATA "        } else {"
DATA "            var refl = 0.12 + 0.6 * fr;"
DATA "            var surf: vec3f;"
DATA "            if (h.mat > 3.5) {"
DATA "                refl = flR + (1.0 - flR) * 0.6 * fr;"
DATA "                surf = direct(p, n, rd, h.col, 0.25, 40.0);"
DATA "            } else {"
DATA "                surf = direct(p, n, rd, h.col, 0.7, 90.0);"
DATA "            }"
DATA "            acc += thr * (1.0 - refl) * surf;"
DATA "            thr *= refl;"
DATA "        }"
DATA "        if (max(thr.x, max(thr.y, thr.z)) < 0.02) { break; }"
DATA "        rd = reflect(rd, n);"
DATA "        ro = p + n * 2e-3;"
DATA "    }"
DATA "    return acc;"
DATA "}"
DATA ""
DATA "// one pixel: the ray from the eye through this point of the cube around it"
DATA "fn shade(i: ShadeIn) -> vec4<f32> {"
DATA "    let dx = dpdx(i.world);"
DATA "    let dy = dpdy(i.world);"
DATA "    let eye = i.params[0].xyz;"
DATA "    build(i32(i.params[1].w + 0.5), i.params[0].w);"
DATA "    var n = 1;"
DATA "    if (i.params[2].x > 0.5) { n = 4; }"
DATA "    var c = vec3f(0.0);"
DATA "    for (var k = 0; k < n; k++) {"
DATA "        var o = vec2f(0.0);"
DATA "        if (n == 4) {"
DATA "            let a = f32(k) * 1.5707963 + 0.4636476;"
DATA "            o = vec2f(cos(a), sin(a)) * 0.3953;"
DATA "        }"
DATA "        c += render(eye, normalize(i.world + o.x * dx + o.y * dy - eye));"
DATA "    }"
DATA "    c = c / f32(n);"
DATA "    c = c / (1.0 + 0.08 * c);"
DATA "    return vec4<f32>(pow(clamp(c, vec3f(0.0), vec3f(1.0)), vec3f(1.0 / 2.2)), 1.0);"
DATA "}"
DATA "~"

SUB SetTitle (s%, fps)
    t$ = "QBNex GPU ray tracer - " + scenename$(s%)
    IF fps > 0 THEN t$ = t$ + " - " + LTRIM$(STR$(INT(fps + 0.5))) + " fps"
    _TITLE t$
END SUB

' Where the camera is at time t: each scene circles or sways around its subject.
SUB Camera (s%, t, ex, ey, ez, tx, ty, tz, fov)
    tx = 0: tz = 0: fov = 50
    SELECT CASE s%
        CASE 1: a = 0.5 + t * 0.12: r = 9.5: ey = 3.4: ty = 1.2
        CASE 2: a = 2 - t * 0.1: r = 8.8: ey = 3 + 0.6 * SIN(t * 0.3): ty = 1.2
        CASE 3: a = PI / 2 + 0.6 * SIN(t * 0.18): r = 7.4: ey = 3.1: ty = 1.9: fov = 42
        CASE 4: a = 0.7 + t * 0.08: r = 8.5: ey = 4.6: ty = 1.1
        CASE 5: a = 1 + t * 0.1: r = 10: ey = 3 + 1.5 * SIN(t * 0.25): ty = 2.9: fov = 46
        CASE ELSE: a = 0.6 + t * 0.1: r = 9.2: ey = 5: ty = 1: fov = 48
    END SELECT
    ex = r * COS(a): ez = r * SIN(a)
END SUB
