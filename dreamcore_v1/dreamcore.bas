' ============================================================================
'  DREAMCORE  -  15-minute psychological dreamcore / weirdcore / liminal tour
'  for the QBNex compiler (QBasic / QB64-compatible syntax, one file, no assets)
'
'  - 1920 x 1080, all 3D on the GPU: a WGSL shader (_SHADER) raymarches signed
'    distance fields for every pixel and ray traces each one: the primary
'    ray, a soft shadow ray toward the sun, ambient occlusion and one
'    reflection bounce through the scene (water, chrome, the wet eyes)
'  - 50 worlds of 18 seconds each, built from separate pieces (one per cell,
'    with a gap on every side) around a clear tube along the flight path, so
'    nothing touches or passes through anything else
'  - "The Sentient Watcher": a living eye that follows the camera, turns its
'    iris toward it and dilates and glows while the voice speaks; from
'    minute 10.5 rings of smaller eyes circle it
'  - melting transitions (smoothstep + 3D sine noise) between worlds
'  - robotic English voice synthesised sample by sample with _SNDRAW
'    (formant filters), with subtitles
'  - automatic Lissajous camera flight with roll and screen shake
'  - ESC quits at any time; after 900 seconds the screen fades to white,
'    the voice says goodbye and the program ends
'
'  Build:  qb dreamcore.bas     Run:  dreamcore
'  Options: /W window, /T=seconds start later, /FPS details in the HUD,
'           /SHADERLOG write shader errors to dreamcore_shader.log
'  Subtitles and HUD use a monospace TrueType font of the system (Consolas,
'  DejaVu Sans Mono, Liberation Mono, Courier New, ...), else the built-in font.
' ============================================================================
DEFSNG A-Z

CONST scrW = 1920, scrH = 1080
CONST PI = 3.14159265
CONST WLEN = 18
CONST ENDT = 909
CONST BLK = 128
CONST SEGMAX = 700
CONST LEAD = .3

' ---- screen, GPU, overlay -----------------------------------------------------
DIM SHARED scr AS LONG, uiK
DIM SHARED tracer AS LONG, dome AS LONG, gpuQ
DIM SHARED camX, camY, camZ, camTX, camTY, camTZ, camRoll
DIM SHARED cfX, cfY, cfZ, rbX, rbZ, ubX, ubY, ubZ
DIM SHARED ovl AS LONG, quadSh AS LONG, quadM AS LONG
DIM SHARED fontEn AS LONG, fontHud AS LONG, fontOK AS LONG
DIM SHARED showFps AS LONG, fpsVal
DIM SHARED hud1$, hud2$, hudA, introA, ovlKey$, ovlDirty AS LONG
DIM SHARED subOn, subS$, subFull$, subAl, subGl$, glTick, glLine

' ---- world state -------------------------------------------------------------
DIM SHARED tNow, horror, act, wCur
DIM SHARED kindA, kindB, blendB, melt, meltT
DIM SHARED glitchAmt, staticAmt, voiceEnv
DIM SHARED wKind(49)

' ---- voice synthesiser --------------------------------------------------------
DIM SHARED sr AS LONG, audioOn AS LONG, genS AS LONG
DIM SHARED segN AS LONG, segIdx AS LONG, segPos AS LONG, segTot
DIM SHARED sgDur(SEGMAX), sgF1(SEGMAX), sgF2(SEGMAX), sgF3(SEGMAX)
DIM SHARED sgAmp(SEGMAX), sgNz(SEGMAX), sgNF(SEGMAX), sgP0(SEGMAX), sgP1(SEGMAX)
DIM SHARED sgV(SEGMAX) AS LONG
DIM SHARED cF1, cF2, cF3, cAmp, cNz, cNF, curPit
DIM SHARED ph, y11, y12, y21, y22, y31, y32, yn1, yn2
DIM SHARED a1, b11, b12, a2, b21, b22, a3, b31, b32, an, bn1, bn2
DIM SHARED rmS, rmC, h1S, h1C, h1K, h2S, h2C, h2K, h3S, h3C, h3K
DIM SHARED echoBuf(32767), ep AS LONG, echoLen AS LONG
DIM SHARED envRing(4095)
DIM SHARED nextLine, subLine
DIM SHARED enL$(50), phL$(50), lineAt(50), lineStart(50), lineDur(50)
' music, stingers, events
DIM SHARED sinTab(4095), chordN(4, 3, 3), pPh(5), pInc(5), lastHB
DIM SHARED mbP, mbI, mbE, hbP, hbI, hbE, stE, stF, stP, stingW, fgP, rmb
DIM SHARED evList(49), evType, evPh, evTxt$, fontBig AS LONG

DIM t0 AS DOUBLE, tn AS DOUBLE, prevT AS DOUBLE, dayAdd AS DOUBLE

' ============================================================================
'  start
' ============================================================================
RANDOMIZE TIMER
cmd$ = UCASE$(COMMAND$)
uiK = scrH / 450
scr = _NEWIMAGE(scrW, scrH, 32)
SCREEN scr
_TITLE "DREAMCORE"
IF _GPU3D = 0 THEN
    COLOR _RGB32(255, 200, 200)
    _FONT 16
    _PRINTSTRING (40, 40), "DREAMCORE draws all of its 3D on the GPU, and no GPU adapter was found."
    _PRINTSTRING (40, 60), "Press any key."
    _DISPLAY
    SLEEP
    SYSTEM
END IF
IF InitGPU = 0 THEN
    COLOR _RGB32(255, 200, 200)
    _FONT 16
    _PRINTSTRING (40, 40), "The GPU shader did not compile: " + _SHADERERROR$
    _PRINTSTRING (40, 60), "Press any key."
    _DISPLAY
    SLEEP
    SYSTEM
END IF
IF INSTR(cmd$, "/W") = 0 THEN _FULLSCREEN _SQUAREPIXELS
_PRINTMODE _KEEPBACKGROUND

InitData
LoadFonts
InitAudio

_DEST scr
CLS
_DISPLAY

' /T=seconds starts the tour later (for example /T=630 starts at minute 10.5)
startAt = 0
IF INSTR(cmd$, "/T=") > 0 THEN startAt = Cl(VAL(MID$(cmd$, INSTR(cmd$, "/T=") + 3)), 0, 900)
IF INSTR(cmd$, "/FPS") > 0 THEN showFps = 1
genS = CLNG(startAt * sr)
DO WHILE nextLine <= 50
    IF lineAt(nextLine) >= startAt THEN EXIT DO
    nextLine = nextLine + 1
LOOP

t0 = TIMER(.001) - startAt: prevT = TIMER(.001): dayAdd = 0
lastEl = startAt: fpsT = startAt
DO
    tn = TIMER(.001)
    IF tn < prevT THEN dayAdd = dayAdd + 86400
    prevT = tn
    el = tn + dayAdd - t0
    IF el > ENDT THEN EXIT DO
    dt = el - lastEl: lastEl = el
    fcount = fcount + 1
    IF el - fpsT >= 1 THEN fpsVal = fcount / (el - fpsT): fcount = 0: fpsT = el

    UpdateWorld el, dt
    AudioPump el
    ' a cube around the camera: every pixel of the screen runs the shader once
    _CAMERA (camX, camY, camZ) TO (camTX, camTY, camTZ), 60
    _SHADERPARAM tracer, 0, camX: _SHADERPARAM tracer, 1, camY
    _SHADERPARAM tracer, 2, camZ: _SHADERPARAM tracer, 3, el
    _SHADERPARAM tracer, 4, kindA: _SHADERPARAM tracer, 5, kindB
    _SHADERPARAM tracer, 6, blendB: _SHADERPARAM tracer, 7, melt
    _SHADERPARAM tracer, 8, camRoll: _SHADERPARAM tracer, 9, voiceEnv
    _SHADERPARAM tracer, 10, evType + evPh * .999: _SHADERPARAM tracer, 11, glitchAmt
    _SHADERPARAM tracer, 12, wCur: _SHADERPARAM tracer, 13, staticAmt
    _SHADERPARAM tracer, 14, scrW / scrH: _SHADERPARAM tracer, 15, gpuQ
    _CLEAR3D 0
    _DRAW3D dome, camX, camY, camZ, 0, 0, 0, 20, 20, 20
    ' the 3D layer covers the screen page, so text and HUD go on a
    ' transparent overlay drawn in 3D right in front of the camera; it is
    ' redrawn only when what it shows changes
    OverlayPrep el
    IF ovlKey$ <> lastKey$ THEN
        DrawOverlay el
        lastKey$ = ovlKey$
        ovlDirty = 1
    END IF
    OverlayQuad
    _DISPLAY

    k$ = INKEY$
    IF k$ = CHR$(27) THEN SYSTEM

    ' keep the frame rate up: fewer or more raymarch steps per pixel, judged
    ' on the frame time averaged over a few frames
    ftAvg = ftAvg * .85 + dt * .15
    IF ftAvg > 1 / 30 THEN
        gpuQ = gpuQ - 2
        IF gpuQ < 60 THEN gpuQ = 60
    ELSEIF ftAvg < 1 / 45 THEN
        gpuQ = gpuQ + 1
        IF gpuQ > 180 THEN gpuQ = 180
    END IF
LOOP
SYSTEM

' ============================================================================
'  data
' ============================================================================
WorldKinds:
' 1 pool, 2 playground, 3 doors in a field, 4 EXIT hall, 5 spiral stairs,
' 6 dense corridor, 7 hallway, 8 floating classroom, 9 arch tunnel,
' 10 upside-down, 11 melting columns, 12 fractal, 13 eye temple,
' 14 red void, 15 glitch, 16 light
DATA 1,2,3,4,1,2,3,4,1,3
DATA 5,6,5,6,5,6,5,6,5,6
DATA 7,8,9,7,8,9,7,8,9,7
DATA 10,11,12,10,11,12,10,11,12,11
DATA 13,14,15,13,14,15,13,14,15,16

VoiceLines:
' each line: the subtitle, then how it is said (ARPAbet; 1 marks the stressed
' vowel, / separates words)
DATA "Welcome back home.","W EH1 L K AH M / B AE1 K / HH OW1 M ."
DATA "The water was never this cold.","DH AH / W AO1 T ER / W AH Z / N EH1 V ER / DH IH S / K OW1 L D ."
DATA "Nobody comes to play anymore.","N OW1 B AA D IY / K AH1 M Z / T UW / P L EY1 / EH N IY M AO1 R ."
DATA "This door opens to anywhere.","DH IH S / D AO1 R / OW1 P AH N Z / T UW / EH1 N IY W EH R ."
DATA "The exit is always so close.","DH IY / EH1 G Z IH T / IH Z / AO1 L W EY Z / S OW / K L OW1 S ."
DATA "You were here as a child.","Y UW / W ER / HH IY1 R / AE Z / AH / CH AY1 L D ."
DATA "The swing still moves by itself.","DH AH / S W IH1 NG / S T IH L / M UW1 V Z / B AY / IH T S EH1 L F ."
DATA "Do not open the third door.","D UW / N AA1 T / OW1 P AH N / DH AH / TH ER1 D / D AO1 R ."
DATA "The exit sign is lying.","DH IY / EH1 G Z IH T / S AY1 N / IH Z / L AY1 IH NG ."
DATA "I can see you now.","AY / K AE N / S IY1 / Y UW / N AW1 ."
DATA "These stairs never end.","DH IY Z / S T EH1 R Z / N EH1 V ER / EH1 N D ."
DATA "The walls are breathing.","DH AH / W AO1 L Z / AA R / B R IY1 DH IH NG ."
DATA "Keep climbing. Do not stop.","K IY1 P / K L AY1 M IH NG . D UW / N AA1 T / S T AA1 P ."
DATA "This fog smells like memory.","DH IH S / F AO1 G / S M EH1 L Z / L AY K / M EH1 M ER IY ."
DATA "You counted the steps wrong.","Y UW / K AW1 N T IH D / DH AH / S T EH1 P S / R AO1 NG ."
DATA "The hall shrinks every time you blink.","DH AH / HH AO1 L / SH R IH1 NG K S / EH1 V R IY / T AY1 M / Y UW / B L IH1 NG K ."
DATA "Up is down here.","AH1 P / IH Z / D AW1 N / HH IY1 R ."
DATA "This pink is not real.","DH IH S / P IH1 NG K / IH Z / N AA1 T / R IY1 L ."
DATA "I am always behind you.","AY / AE M / AO1 L W EY Z / B IH HH AY1 N D / Y UW ."
DATA "Do not look back.","D UW / N AA1 T / L UH1 K / B AE1 K ."
DATA "This hallway is longer than you think.","DH IH S / HH AO1 L W EY / IH Z / L AO1 NG G ER / DH AE N / Y UW / TH IH1 NG K ."
DATA "The school bell rang thirty years ago.","DH AH / S K UW1 L / B EH1 L / R AE1 NG / TH ER1 T IY / Y IH1 R Z / AH G OW1 ."
DATA "Every arch is a closed eye.","EH1 V R IY / AA1 R CH / IH Z / AH / K L OW1 Z D / AY1 ."
DATA "Room fourteen does not exist.","R UW1 M / F AO R T IY1 N / D AH Z / N AA1 T / IH G Z IH1 S T ."
DATA "The teacher is still waiting for you.","DH AH / T IY1 CH ER / IH Z / S T IH1 L / W EY1 T IH NG / F AO R / Y UW ."
DATA "Doors inside doors. Dreams inside dreams.","D AO1 R Z / IH N S AY1 D / D AO1 R Z . D R IY1 M Z / IH N S AY1 D / D R IY1 M Z ."
DATA "The lights whisper your name.","DH AH / L AY1 T S / W IH1 S P ER / Y AO R / N EY1 M ."
DATA "That desk was yours.","DH AE T / D EH1 S K / W AH Z / Y AO1 R Z ."
DATA "We are back where we started.","W IY / AA R / B AE1 K / W EH1 R / W IY / S T AA1 R T IH D ."
DATA "I remember your face.","AY / R IH M EH1 M B ER / Y AO R / F EY1 S ."
DATA "The world is upside down.","DH AH / W ER1 L D / IH Z / AH1 P S AY D / D AW1 N ."
DATA "These pillars are melting.","DH IY Z / P IH1 L ER Z / AA R / M EH1 L T IH NG ."
DATA "This shape should not exist.","DH IH S / SH EY1 P / SH UH D / N AA1 T / IH G Z IH1 S T ."
DATA "The sky is under your feet.","DH AH / S K AY1 / IH Z / AH1 N D ER / Y AO R / F IY1 T ."
DATA "Time is folding into itself.","T AY1 M / IH Z / F OW1 L D IH NG / IH N T UW / IH T S EH1 L F ."
DATA "We are awake now.","W IY / AA R / AH W EY1 K / N AW1 ."
DATA "So many eyes are watching you.","S OW / M EH1 N IY / AY1 Z / AA R / W AA1 CH IH NG / Y UW ."
DATA "Gravity has forgotten you.","G R AE1 V IH T IY / HH AE Z / F ER G AA1 T AH N / Y UW ."
DATA "Do not fear. We are you.","D UW / N AA1 T / F IY1 R . W IY / AA R / Y UW1 ."
DATA "You never left this place.","Y UW / N EH1 V ER / L EH1 F T / DH IH S / P L EY1 S ."
DATA "Welcome to the temple.","W EH1 L K AH M / T UW / DH AH / T EH1 M P AH L ."
DATA "The sky is turning red.","DH AH / S K AY1 / IH Z / T ER1 N IH NG / R EH1 D ."
DATA "Signal lost.","S IH1 G N AH L / L AO1 S T ."
DATA "Every eye is your memory.","EH1 V R IY / AY1 / IH Z / Y AO R / M EH1 M ER IY ."
DATA "Do not adjust your signal.","D UW / N AA1 T / AH JH AH1 S T / Y AO R / S IH1 G N AH L ."
DATA "Reality is breaking apart.","R IY AE1 L IH T IY / IH Z / B R EY1 K IH NG / AH P AA1 R T ."
DATA "Can you hear me?","K AE N / Y UW / HH IY1 R / M IY1 ?"
DATA "Almost there.","AO1 L M OW S T / DH EH1 R ."
DATA "Let it all melt away.","L EH1 T / IH T / AO1 L / M EH1 L T / AH W EY1 ."
DATA "It is time to wake up.","IH T / IH Z / T AY1 M / T UW / W EY1 K / AH1 P ."
DATA "Goodbye. See you again in the dream.","G UH D B AY1 . S IY1 / Y UW / AH G EH1 N / IH N / DH AH / D R IY1 M ."

' one scare event per world: 0 none, 1 the eye blinks, 2 blackout, 3 the camera
' turns away and snaps back, 4 a figure stands on the path, 5 the eye lunges
Events:
DATA 0,1,0,4,1,2,0,3,1,4
DATA 1,2,4,5,1,3,2,4,1,5
DATA 2,4,3,1,5,2,4,3,1,5
DATA 2,4,3,5,1,2,4,5,3,2
DATA 5,4,2,3,5,4,2,5,3,0

' chords of each act (MIDI notes): dreamy C, pastel E flat, melancholic A minor,
' uneasy D minor, then a chromatic, dissonant last act
Chords:
DATA 48,52,55,59, 45,48,52,55, 41,45,48,52, 43,47,50,55
DATA 51,55,58,62, 48,51,55,58, 44,48,51,55, 46,50,53,57
DATA 45,48,52,55, 41,45,48,52, 48,52,55,59, 40,44,47,50
DATA 50,53,57,60, 46,50,53,57, 43,46,50,53, 45,49,52,55
DATA 48,51,55,59, 49,53,56,60, 48,51,54,58, 47,51,54,58

GpuShader:
' the GPU raymarcher in WGSL, one source line per DATA string ('~' ends it)
DATA "// DREAMCORE raymarcher (GPU). Every world is built from separate pieces: one"
DATA "// piece per cell of a 9 x 9 x 10 grid with a gap on every side, and nothing"
DATA "// inside a clear tube around the flight path, so the watcher, its tentacles,"
DATA "// its satellite eyes and the camera never touch or pass through anything."
DATA "// params: 0-2 camera, 3 time, 4 world kind A, 5 kind B, 6 blend, 7 melt,"
DATA "// 8 roll, 9 voice level, 10 dread, 11 glitch, 12 world index, 13 static,"
DATA "// 14 screen aspect, 15 raymarch steps"
DATA "var<private> T: f32;"
DATA "var<private> kA: i32;"
DATA "var<private> kB: i32;"
DATA "var<private> bl: f32;"
DATA "var<private> mlt: f32;"
DATA "var<private> hor: f32;"
DATA "var<private> gli: f32;"
DATA "var<private> venv: f32;"
DATA "var<private> pup: f32;"
DATA "var<private> stat: f32;"
DATA "var<private> qual: f32;"
DATA "var<private> act: i32;"
DATA "var<private> cam: vec3f;"
DATA "var<private> eyeP: vec3f;"
DATA "var<private> eyeR: f32;"
DATA "var<private> gz: vec3f;"
DATA "var<private> tb1: vec3f;"
DATA "var<private> tb2: vec3f;"
DATA "var<private> sat1: f32;"
DATA "var<private> sat2: f32;"
DATA "var<private> fogC: vec3f;"
DATA "var<private> fogD: f32;"
DATA "var<private> gMat: i32;"
DATA "var<private> gSub: i32;"
DATA "var<private> gEyeD: f32;"
DATA "var<private> gLoc: vec3f;"
DATA "var<private> gCtr: vec3f;"
DATA "var<private> oMat: i32;"
DATA "var<private> oLoc: vec3f;"
DATA "var<private> oCtr: vec3f;"
DATA "var<private> cMat: i32;"
DATA "var<private> cLoc: vec3f;"
DATA "var<private> cCtr: vec3f;"
DATA "// false while AO and shadows sample the scene: empty cells then report no"
DATA "// surface at all instead of the distance to their walls"
DATA "var<private> useBnd: bool = true;"
DATA "// direction of the ray being marched: a cell is left after (wall distance /"
DATA "// speed toward that wall), often far more than the plain wall distance"
DATA "var<private> rdir: vec3f = vec3f(0.0, 0.0, 1.0);"
DATA ""
DATA "// world kinds: floor, cell mode, object A, object B, share of A, share of"
DATA "// empty cells, first cell column used, upper floating row object, special"
DATA "// structure, sky, fog colour, fog density"
DATA "struct KI { fl: i32, md: i32, oa: i32, ob: i32, mx: f32, em: f32, mi: f32, up: i32, sp: i32, sky: i32, fog: vec3f, fd: f32 };"
DATA ""
DATA "// events: type (0 none, 1 blinks, 2 blackout, 3 the camera turns away, 4 a"
DATA "// figure stands on the path, 5 lunge) and phase 0..1, from the BASIC code"
DATA "var<private> evT: i32;"
DATA "var<private> evP: f32;"
DATA "var<private> lid: f32;"
DATA "var<private> dark: f32;"
DATA "var<private> figOn: bool;"
DATA "var<private> dissolve: f32;"
DATA ""
DATA "var<private> KIa: KI;"
DATA "var<private> KIb: KI;"
DATA ""
DATA "// fog colour of each sky: 0 pastel day, 1 violet dusk, 2 red void, 3 white,"
DATA "// 4 night with a moon, 5 sunset, 6 aqua dawn, 7 iridescent, 8 hazy sun with a"
DATA "// giant eye, 9 dark violet"
DATA "fn skyFog(sky: i32) -> vec3f {"
DATA "    var c = vec3f(225.0, 180.0, 235.0);"
DATA "    switch sky {"
DATA "        case 1: { c = vec3f(150.0, 110.0, 170.0); }"
DATA "        case 2: { c = vec3f(40.0, 2.0, 12.0); }"
DATA "        case 3: { c = vec3f(255.0, 255.0, 255.0); }"
DATA "        case 4: { c = vec3f(45.0, 42.0, 90.0); }"
DATA "        case 5: { c = vec3f(240.0, 150.0, 140.0); }"
DATA "        case 6: { c = vec3f(190.0, 228.0, 228.0); }"
DATA "        case 7: { c = vec3f(215.0, 195.0, 240.0); }"
DATA "        case 8: { c = vec3f(225.0, 195.0, 175.0); }"
DATA "        case 9: { c = vec3f(60.0, 35.0, 92.0); }"
DATA "        default: { }"
DATA "    }"
DATA "    return c;"
DATA "}"
DATA ""
DATA "// floor (0 none, 1 pink tiles, 2 grass, 3 carpet, 4 marble, 5 pool tiles,"
DATA "// 6 water, 7 asphalt, 8 checkerboard, 9 sand), cell mode (-1 none, 0 standing, 1 floating rows,"
DATA "// 2 hanging), piece A, piece B, share of A, share of empty cells, cell"
DATA "// columns kept free (1, 2 beside arches), upper floating row piece, special"
DATA "// (1 hall, 2 arches, 3 ceiling floor), sky"
DATA "fn mk(fl: i32, md: i32, oa: i32, ob: i32, mx: f32, em: f32, mi: f32, up: i32, sp: i32, sky: i32) -> KI {"
DATA "    var fd = 0.02;"
DATA "    if (sp == 1 || sky == 2) { fd = 0.03; }"
DATA "    return KI(fl, md, oa, ob, mx, em, mi, up, sp, sky, skyFog(sky), fd);"
DATA "}"
DATA ""
DATA "// the 50 worlds; pieces: 1 pool, 2 swings, 3 slide, 4 door, 5 EXIT sign,"
DATA "// 6 spiral stair, 7 corridor, 8 desk, 9 chalkboard, 10 column, 11 melting"
DATA "// arch, 12 fractal cube, 13 eye pillar, 14 shard, 15 glitch blocks, 16 big"
DATA "// eye, 17 old TV, 18 street lamp, 19 small house, 20 bed, 21 straight stairs,"
DATA "// 22 aqueduct, 23 door frame, 24 spiral, 25 mirror sphere, 26 wire cubes,"
DATA "// 27 glowing ring, 28 fractal tower, 29 liquid chrome, 30 wire crystal,"
DATA "// 31 tiled wall with doors and EXIT signs"
DATA "fn winfo(w: i32) -> KI {"
DATA "    // 49: everything turns to light"
DATA "    var q = mk(1, 0, 1, 10, 0.6, 0.25, 1.0, 21, 0, 3);"
DATA "    switch w {"
DATA "        case 0: { q = mk(2,0,1,10,0.5,0.3,1.0,21,0,0); }  // meadow, pools, columns, floating stairs"
DATA "        case 1: { q = mk(8,0,31,1,0.5,0.2,1.0,0,0,0); }  // checker floor, EXIT walls, pools"
DATA "        case 2: { q = mk(2,0,2,3,0.5,0.3,1.0,0,0,0); }  // playground"
DATA "        case 3: { q = mk(2,0,4,18,0.6,0.3,1.0,0,0,5); }  // doors and street lamps at sunset"
DATA "        case 4: { q = mk(6,0,10,11,0.5,0.3,1.0,0,0,6); }  // water, columns, melting arches"
DATA "        case 5: { q = mk(2,0,19,18,0.55,0.25,1.0,0,0,4); }  // houses at night"
DATA "        case 6: { q = mk(3,0,17,20,0.5,0.25,1.0,0,0,1); }  // TVs and beds"
DATA "        case 7: { q = mk(5,0,31,3,0.5,0.3,1.0,21,0,0); }  // pool tiles, EXIT walls, slides"
DATA "        case 8: { q = mk(2,0,2,4,0.5,0.3,1.0,0,0,5); }  // swings and doors at sunset"
DATA "        case 9: { q = mk(8,0,1,31,0.5,0.25,1.0,23,0,0); }  // checker pools, floating door frames"
DATA "        case 10: { q = mk(0,1,6,22,0.5,0.45,1.0,0,0,0); }  // spiral stairs and aqueducts"
DATA "        case 11: { q = mk(0,1,23,24,0.6,0.45,1.0,0,0,0); }  // pastel void: door frames, spirals"
DATA "        case 12: { q = mk(0,1,22,21,0.5,0.45,1.0,0,0,6); }  // aqueducts and straight stairs"
DATA "        case 13: { q = mk(0,1,7,17,0.5,0.4,1.0,0,0,4); }  // corridors and TVs at night"
DATA "        case 14: { q = mk(0,1,6,10,0.5,0.45,1.0,0,0,5); }  // stairs and columns at sunset"
DATA "        case 15: { q = mk(0,1,23,4,0.5,0.4,1.0,0,0,1); }  // door frames and doors"
DATA "        case 16: { q = mk(0,1,21,19,0.6,0.45,1.0,0,0,0); }  // stairs and houses"
DATA "        case 17: { q = mk(0,1,22,11,0.6,0.45,1.0,0,0,6); }  // aqueducts and melting arches"
DATA "        case 18: { q = mk(0,1,7,18,0.6,0.4,1.0,0,0,4); }  // corridors and lamps"
DATA "        case 19: { q = mk(0,1,24,20,0.6,0.45,1.0,0,0,1); }  // spirals and beds"
DATA "        case 20: { q = mk(3,-1,0,0,0.5,0.0,1.0,0,1,1); }  // hallway"
DATA "        case 21: { q = mk(0,1,8,9,0.65,0.4,1.0,0,0,0); }  // classroom"
DATA "        case 22: { q = mk(1,0,10,11,0.5,0.3,2.0,0,2,0); }  // arches"
DATA "        case 23: { q = mk(0,1,26,25,0.55,0.4,1.0,0,0,7); }  // inverted chromatic: wire cubes, mirror spheres"
DATA "        case 24: { q = mk(0,1,8,17,0.6,0.4,1.0,0,0,1); }  // desks and TVs"
DATA "        case 25: { q = mk(6,0,11,10,0.5,0.3,2.0,0,2,6); }  // arches over water"
DATA "        case 26: { q = mk(7,-1,0,0,0.5,0.0,1.0,0,1,4); }  // night hallway"
DATA "        case 27: { q = mk(0,1,25,26,0.5,0.4,1.0,0,0,7); }  // mirror spheres and wire cubes"
DATA "        case 28: { q = mk(2,0,18,10,0.5,0.3,2.0,0,2,5); }  // arches on grass"
DATA "        case 29: { q = mk(0,1,9,20,0.55,0.4,1.0,0,0,4); }  // chalkboards and beds"
DATA "        case 30: { q = mk(0,2,8,10,0.5,0.25,1.0,0,3,0); }  // upside down"
DATA "        case 31: { q = mk(0,1,29,6,0.6,0.4,1.0,0,0,0); }  // melting void: liquid chrome"
DATA "        case 32: { q = mk(9,0,28,27,0.65,0.25,1.0,0,0,8); }  // fractal cathedral, rings"
DATA "        case 33: { q = mk(0,2,18,17,0.5,0.25,1.0,0,3,6); }  // upside-down lamps and TVs"
DATA "        case 34: { q = mk(4,0,29,11,0.5,0.25,1.0,12,0,5); }  // chrome blobs, melting arches"
DATA "        case 35: { q = mk(0,1,12,27,0.6,0.35,1.0,0,0,8); }  // fractal cubes, rings"
DATA "        case 36: { q = mk(0,2,19,4,0.5,0.25,1.0,0,3,4); }  // upside-down houses"
DATA "        case 37: { q = mk(6,0,10,11,0.5,0.25,1.0,12,0,0); }  // water, columns, fractal cubes"
DATA "        case 38: { q = mk(0,1,29,25,0.55,0.4,1.0,0,0,7); }  // chrome blobs, mirror spheres"
DATA "        case 39: { q = mk(9,0,28,13,0.6,0.2,1.0,27,0,8); }  // fractal cathedral, eye pillars"
DATA "        case 40: { q = mk(4,0,13,13,0.5,0.1,1.0,16,0,2); }  // eye temple"
DATA "        case 41: { q = mk(0,1,30,16,0.6,0.35,1.0,0,0,9); }  // wire crystals, giant eyes"
DATA "        case 42: { q = mk(0,1,15,7,0.6,0.3,1.0,0,0,2); }  // glitch"
DATA "        case 43: { q = mk(7,0,13,17,0.5,0.15,1.0,16,0,9); }  // eye pillars and TVs"
DATA "        case 44: { q = mk(0,1,16,15,0.5,0.3,1.0,0,0,2); }  // giant eyes, glitch"
DATA "        case 45: { q = mk(6,0,13,4,0.6,0.15,1.0,30,0,9); }  // eye pillars in dark water"
DATA "        case 46: { q = mk(0,1,17,30,0.5,0.3,1.0,0,0,9); }  // TVs and crystals"
DATA "        case 47: { q = mk(0,2,13,17,0.5,0.2,1.0,0,3,2); }  // upside-down eye pillars"
DATA "        case 48: { q = mk(0,1,14,15,0.5,0.25,1.0,0,0,9); }  // dissolution"
DATA "        default: { }"
DATA "    }"
DATA "    return q;"
DATA "}"
DATA ""
DATA "fn pathX(z: f32) -> f32 { return 2.2 * sin(z * 0.045) + 0.9 * sin(z * 0.11 + 1.3); }"
DATA "fn pathY(z: f32) -> f32 { return 4.0 + 0.4 * sin(z * 0.07 + 0.4); }"
DATA "fn rp(v: f32, c: f32) -> f32 { return v - c * floor(v / c + 0.5); }"
DATA "fn hs(n: f32) -> f32 { return fract(sin(n * 12.9898 + 78.233) * 43758.5453); }"
DATA "fn hash3(p: vec3f) -> f32 { return fract(sin(dot(p, vec3f(127.1, 311.7, 74.7))) * 43758.5453); }"
DATA "fn vnoise(p: vec3f) -> f32 {"
DATA "    let i = floor(p);"
DATA "    let f = fract(p);"
DATA "    let u = f * f * (3.0 - 2.0 * f);"
DATA "    let a = mix(mix(hash3(i), hash3(i + vec3f(1.0, 0.0, 0.0)), u.x), mix(hash3(i + vec3f(0.0, 1.0, 0.0)), hash3(i + vec3f(1.0, 1.0, 0.0)), u.x), u.y);"
DATA "    let b = mix(mix(hash3(i + vec3f(0.0, 0.0, 1.0)), hash3(i + vec3f(1.0, 0.0, 1.0)), u.x), mix(hash3(i + vec3f(0.0, 1.0, 1.0)), hash3(i + vec3f(1.0, 1.0, 1.0)), u.x), u.y);"
DATA "    return mix(a, b, u.z);"
DATA "}"
DATA "fn fbm(p: vec3f) -> f32 {"
DATA "    var s = 0.0;"
DATA "    var a = 0.5;"
DATA "    var q = p;"
DATA "    for (var i = 0; i < 3; i++) {"
DATA "        s = s + a * vnoise(q);"
DATA "        q = q * 2.03 + vec3f(1.7, 9.2, 4.1);"
DATA "        a = a * 0.5;"
DATA "    }"
DATA "    return s;"
DATA "}"
DATA "fn box3(p: vec3f, b: vec3f) -> f32 {"
DATA "    let q = abs(p) - b;"
DATA "    return length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);"
DATA "}"
DATA "fn capsule(p: vec3f, a: vec3f, b: vec3f, r: f32) -> f32 {"
DATA "    let pa = p - a;"
DATA "    let ba = b - a;"
DATA "    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);"
DATA "    return length(pa - ba * h) - r;"
DATA "}"
DATA "fn rot2(v: vec2f, a: f32) -> vec2f {"
DATA "    let c = cos(a);"
DATA "    let s = sin(a);"
DATA "    return vec2f(v.x * c - v.y * s, v.x * s + v.y * c);"
DATA "}"
DATA ""
DATA "// ---- pieces, each inside |x| < 3.6, |y| < 3.6, |z| < 4.4 of its cell ----"
DATA "// grounded pieces stand on the floor at y = -3.6"
DATA ""
DATA "fn oPool(p: vec3f) -> f32 {"
DATA "    let g = -3.6;"
DATA "    var d = max(box3(p - vec3f(0.0, g + 0.45, 0.0), vec3f(2.9, 0.45, 3.9)), -box3(p - vec3f(0.0, g + 0.9, 0.0), vec3f(2.5, 0.6, 3.5)));"
DATA "    oMat = 23;"
DATA "    oLoc = p;"
DATA "    let w = box3(p - vec3f(0.0, g + 0.5, 0.0), vec3f(2.5, 0.2, 3.5));"
DATA "    if (w < d) { d = w; oMat = 4; }"
DATA "    let lq = vec3f(abs(p.x - 1.2) - 0.32, p.y, p.z);"
DATA "    var r = capsule(lq, vec3f(0.0, g + 0.4, -3.2), vec3f(0.0, g + 1.6, -3.2), 0.045);"
DATA "    r = min(r, capsule(lq, vec3f(0.0, g + 1.6, -3.2), vec3f(0.0, g + 1.6, -3.7), 0.045));"
DATA "    r = min(r, capsule(lq, vec3f(0.0, g + 1.6, -3.7), vec3f(0.0, g + 1.0, -3.7), 0.045));"
DATA "    if (r < d) { d = r; oMat = 9; }"
DATA "    return d;"
DATA "}"
DATA ""
DATA "fn oSwing(p: vec3f, h: f32) -> f32 {"
DATA "    let g = -3.6;"
DATA "    let top = g + 3.2;"
DATA "    let lq = vec3f(abs(p.x) - 2.0, p.y, abs(p.z));"
DATA "    var d = capsule(lq, vec3f(0.0, top, 0.0), vec3f(0.0, g, 1.1), 0.07);"
DATA "    d = min(d, capsule(p, vec3f(-2.0, top, 0.0), vec3f(2.0, top, 0.0), 0.07));"
DATA "    oMat = 13;"
DATA "    oLoc = p;"
DATA "    for (var s = 0; s < 2; s++) {"
DATA "        let sx = select(-0.8, 0.8, s == 1);"
DATA "        let a = 0.5 * sin(T * 1.3 + h * 6.0 + f32(s) * 2.1);"
DATA "        let sy = top - 2.4 * cos(a);"
DATA "        let sz = 2.4 * sin(a);"
DATA "        let seat = box3(p - vec3f(sx, sy, sz), vec3f(0.32, 0.04, 0.16));"
DATA "        if (seat < d) { d = seat; oMat = 12; }"
DATA "        let cq = vec3f(abs(p.x - sx) - 0.26, p.y, p.z);"
DATA "        let ch = capsule(cq, vec3f(0.0, top, 0.0), vec3f(0.0, sy, sz), 0.018);"
DATA "        if (ch < d) { d = ch; oMat = 9; }"
DATA "    }"
DATA "    return d;"
DATA "}"
DATA ""
DATA "fn oSlide(p: vec3f) -> f32 {"
DATA "    let g = -3.6;"
DATA "    var d = box3(p - vec3f(0.0, g + 2.2, 0.0), vec3f(0.6, 0.06, 0.6));"
DATA "    let lq = vec3f(abs(p.x) - 0.5, p.y, abs(p.z) - 0.5);"
DATA "    d = min(d, capsule(lq, vec3f(0.0, g, 0.0), vec3f(0.0, g + 2.2, 0.0), 0.06));"
DATA "    oMat = 13;"
DATA "    oLoc = p;"
DATA "    let q = p - vec3f(0.0, g + 1.15, 2.1);"
DATA "    let rr = rot2(vec2f(q.z, q.y), 0.611);"
DATA "    var s = box3(vec3f(q.x, rr.y, rr.x), vec3f(0.42, 0.04, 1.75));"
DATA "    s = min(s, box3(vec3f(abs(q.x) - 0.42, rr.y - 0.1, rr.x), vec3f(0.04, 0.12, 1.75)));"
DATA "    if (s < d) { d = s; oMat = 9; }"
DATA "    let rq = vec3f(abs(p.x) - 0.35, p.y, p.z);"
DATA "    var lad = capsule(rq, vec3f(0.0, g, -1.1), vec3f(0.0, g + 2.2, -0.62), 0.04);"
DATA "    let ry = clamp(round((p.y - g) / 0.45), 1.0, 4.0);"
DATA "    lad = min(lad, capsule(p, vec3f(-0.35, g + ry * 0.45, -1.1 + ry * 0.098), vec3f(0.35, g + ry * 0.45, -1.1 + ry * 0.098), 0.03));"
DATA "    if (lad < d) { d = lad; oMat = 9; }"
DATA "    return d;"
DATA "}"
DATA ""
DATA "fn exitSign(p: vec3f, c: vec3f) -> f32 {"
DATA "    return box3(p - c, vec3f(0.5, 0.17, 0.06));"
DATA "}"
DATA ""
DATA "fn oDoor(p: vec3f, h: f32) -> f32 {"
DATA "    let g = -3.6;"
DATA "    var d = max(box3(p - vec3f(0.0, g + 1.45, 0.0), vec3f(1.0, 1.45, 0.12)), -box3(p - vec3f(0.0, g + 1.3, 0.0), vec3f(0.8, 1.32, 0.3)));"
DATA "    oMat = 15;"
DATA "    oLoc = p;"
DATA "    let hq = p - vec3f(-0.78, g + 1.3, 0.0);"
DATA "    let rr = rot2(hq.xz, -1.1 - 0.15 * sin(T * 0.4 + h * 7.0));"
DATA "    let panel = box3(vec3f(rr.x - 0.78, hq.y, rr.y), vec3f(0.76, 1.28, 0.04));"
DATA "    if (panel < d) { d = panel; oMat = 15; oLoc = p; }"
DATA "    let knob = length(vec3f(rr.x - 1.35, hq.y, abs(rr.y) - 0.08)) - 0.06;"
DATA "    if (knob < d) { d = knob; oMat = 9; }"
DATA "    if (h > 0.45) {"
DATA "        let sc = vec3f(0.0, g + 3.2, 0.0);"
DATA "        let s = exitSign(p, sc);"
DATA "        if (s < d) { d = s; oMat = 7; oLoc = p - sc; }"
DATA "    }"
DATA "    return d;"
DATA "}"
DATA ""
DATA "fn oExitPost(p: vec3f) -> f32 {"
DATA "    let g = -3.6;"
DATA "    var d = capsule(p, vec3f(0.0, g, 0.0), vec3f(0.0, g + 2.85, 0.0), 0.06);"
DATA "    oMat = 9;"
DATA "    oLoc = p;"
DATA "    let sc = vec3f(0.0, g + 3.0, 0.0);"
DATA "    let s = exitSign(p, sc);"
DATA "    if (s < d) { d = s; oMat = 7; oLoc = p - sc; }"
DATA "    return d;"
DATA "}"
DATA ""
DATA "fn oStair(p: vec3f, h: f32) -> f32 {"
DATA "    var d = max(length(p.xz) - 0.22, abs(p.y) - 3.5);"
DATA "    oMat = 9;"
DATA "    oLoc = p;"
DATA "    let k = clamp(round(p.y / 0.3), -11.0, 11.0);"
DATA "    for (var n = 0; n < 2; n++) {"
DATA "        var kk = k;"
DATA "        if (n == 1) {"
DATA "            if (p.y / 0.3 < k) { kk = max(k - 1.0, -11.0); } else { kk = min(k + 1.0, 11.0); }"
DATA "        }"
DATA "        let rr = rot2(p.xz, -(kk * 0.42 + h * 6.0));"
DATA "        d = min(d, box3(vec3f(rr.x - 1.25, p.y - kk * 0.3, rr.y), vec3f(1.0, 0.055, 0.3)));"
DATA "    }"
DATA "    return d * 0.9;"
DATA "}"
DATA ""
DATA "fn oCorridor(p: vec3f) -> f32 {"
DATA "    var d = max(box3(p, vec3f(2.0, 1.7, 3.8)), -box3(p, vec3f(1.8, 1.5, 4.2)));"
DATA "    oMat = 21;"
DATA "    oLoc = p;"
DATA "    let rec = box3(vec3f(abs(p.x) - 1.9, p.y + 0.35, p.z - 1.2), vec3f(0.12, 1.1, 0.5));"
DATA "    d = max(d, -rec);"
DATA "    let l = box3(p - vec3f(0.0, 1.47, -1.5), vec3f(0.5, 0.03, 0.25));"
DATA "    if (l < d) { d = l; oMat = 3; }"
DATA "    return d;"
DATA "}"
DATA ""
DATA "fn oDesk(pin: vec3f, h: f32) -> f32 {"
DATA "    var p = pin / 1.6;"
DATA "    let a1 = rot2(p.xy, T * 0.11 + h * 6.0);"
DATA "    p = vec3f(a1.x, a1.y, p.z);"
DATA "    let a2 = rot2(p.xz, T * 0.07 + h * 3.0);"
DATA "    p = vec3f(a2.x, p.y, a2.y);"
DATA "    var d = box3(p - vec3f(0.0, 0.3, 0.0), vec3f(0.6, 0.035, 0.4));"
DATA "    oMat = 12;"
DATA "    oLoc = p;"
DATA "    let lg = box3(vec3f(abs(p.x) - 0.52, p.y + 0.12, abs(p.z) - 0.32), vec3f(0.03, 0.42, 0.03));"
DATA "    if (lg < d) { d = lg; oMat = 9; }"
DATA "    let seat = box3(p - vec3f(0.0, -0.05, 0.7), vec3f(0.26, 0.03, 0.24));"
DATA "    if (seat < d) { d = seat; oMat = 12; }"
DATA "    let back = box3(p - vec3f(0.0, 0.25, 0.92), vec3f(0.26, 0.28, 0.025));"
DATA "    if (back < d) { d = back; oMat = 12; }"
DATA "    let cl = box3(vec3f(abs(p.x) - 0.22, p.y + 0.3, abs(p.z - 0.7) - 0.2), vec3f(0.025, 0.27, 0.025));"
DATA "    if (cl < d) { d = cl; oMat = 9; }"
DATA "    return d * 1.6;"
DATA "}"
DATA ""
DATA "fn oBoard(pin: vec3f, h: f32) -> f32 {"
DATA "    let r = rot2(pin.xz, h * 6.0 + T * 0.05);"
DATA "    let p = vec3f(r.x, pin.y, r.y);"
DATA "    var d = box3(p, vec3f(2.2, 1.3, 0.05));"
DATA "    oMat = 16;"
DATA "    oLoc = p;"
DATA "    let fr = max(box3(p, vec3f(2.36, 1.46, 0.08)), -box3(p, vec3f(2.2, 1.3, 0.2)));"
DATA "    if (fr < d) { d = fr; oMat = 12; }"
DATA "    let tray = box3(p - vec3f(0.0, -1.5, 0.12), vec3f(2.2, 0.03, 0.08));"
DATA "    if (tray < d) { d = tray; oMat = 12; }"
DATA "    return d;"
DATA "}"
DATA ""
DATA "fn oColumn(pin: vec3f, h: f32) -> f32 {"
DATA "    let a1 = rot2(pin.xy, (h - 0.5) * 1.6 + 0.1 * sin(T * 0.2 + h * 9.0));"
DATA "    var p = vec3f(a1.x, a1.y, pin.z);"
DATA "    let a2 = rot2(p.zy, (hs(h * 7.0) - 0.5) * 1.0);"
DATA "    p = vec3f(p.x, a2.y, a2.x);"
DATA "    let ang = atan2(p.z, p.x);"
DATA "    var rad = 0.5 + 0.035 * cos(16.0 * ang);"
DATA "    rad = rad + 0.06 * mlt * sin(p.y * 4.0 + T);"
DATA "    var d = max((length(p.xz) - rad) * 0.8, abs(p.y) - 2.5);"
DATA "    oMat = 9;"
DATA "    oLoc = p;"
DATA "    let caps = box3(vec3f(p.x, abs(p.y) - 2.62, p.z), vec3f(0.72, 0.13, 0.72));"
DATA "    if (caps < d) { d = caps; oMat = 22; }"
DATA "    let ring = box3(vec3f(p.x, abs(p.y) - 2.42, p.z), vec3f(0.6, 0.07, 0.6));"
DATA "    if (ring < d) { d = ring; oMat = 22; }"
DATA "    return d;"
DATA "}"
DATA ""
DATA "fn oMeltArch(p: vec3f, h: f32) -> f32 {"
DATA "    let g = -3.6;"
DATA "    let cy = g + 2.4;"
DATA "    let opening = min(box3(p - vec3f(0.0, g + 1.1, 0.0), vec3f(1.4, 1.3, 1.0)), length(p.xy - vec2f(0.0, cy)) - 1.4);"
DATA "    var d = max(box3(p - vec3f(0.0, g + 2.2, 0.0), vec3f(2.3, 2.2, 0.5)), -opening);"
DATA "    oMat = 22;"
DATA "    oLoc = p;"
DATA "    // liquid chrome dripping from the inside of the arch"
DATA "    let i = clamp(round(p.x / 0.4), -3.0, 3.0);"
DATA "    let xi = i * 0.4;"
DATA "    let ye = cy + sqrt(max(1.96 - xi * xi, 0.0));"
DATA "    let len = 0.25 + 0.7 * (0.5 + 0.5 * sin(T * 0.7 + xi * 3.0 + h * 9.0)) * (0.4 + mlt);"
DATA "    var dr = capsule(p, vec3f(xi, ye + 0.05, 0.0), vec3f(xi, ye - len, 0.0), 0.075 + 0.02 * sin(xi * 7.0));"
DATA "    let top = box3(p - vec3f(0.0, g + 4.45, 0.0), vec3f(2.3, 0.06, 0.52));"
DATA "    dr = min(dr, top);"
DATA "    if (dr < d) { d = dr; oMat = 9; }"
DATA "    return d;"
DATA "}"
DATA ""
DATA "fn oMenger(pin: vec3f, h: f32) -> f32 {"
DATA "    let a1 = rot2(pin.xz, T * 0.1 + h * 6.0);"
DATA "    var p = vec3f(a1.x, pin.y, a1.y);"
DATA "    let a2 = rot2(p.xy, T * 0.07 + h * 3.0);"
DATA "    p = vec3f(a2.x, a2.y, p.z) / 2.0;"
DATA "    var d = box3(p, vec3f(1.0));"
DATA "    var s = 1.0;"
DATA "    for (var m = 0; m < 3; m++) {"
DATA "        let a = p * s - 2.0 * floor(p * s / 2.0) - 1.0;"
DATA "        s = s * 3.0;"
DATA "        let rr = abs(1.0 - 3.0 * abs(a));"
DATA "        let da = max(rr.x, rr.y);"
DATA "        let db = max(rr.y, rr.z);"
DATA "        let dc = max(rr.z, rr.x);"
DATA "        d = max(d, (min(da, min(db, dc)) - 1.0) / s);"
DATA "    }"
DATA "    oMat = 9;"
DATA "    oLoc = p;"
DATA "    return d * 2.0;"
DATA "}"
DATA ""
DATA "fn oEyePillar(p: vec3f, side: f32) -> f32 {"
DATA "    let g = -3.6;"
DATA "    var d = max(length(p.xz) - 0.8, abs(p.y - (g + 3.25)) - 3.25);"
DATA "    let cap = box3(p - vec3f(0.0, g + 6.6, 0.0), vec3f(1.05, 0.12, 1.05));"
DATA "    d = min(d, cap);"
DATA "    let k = clamp(round((p.y - g - 1.6) / 1.6), 0.0, 2.0);"
DATA "    let ec = vec3f(side * 0.62, g + 1.6 + k * 1.6, 0.0);"
DATA "    d = max(d, -(length(p - ec) - 0.56));"
DATA "    oMat = 24;"
DATA "    oLoc = p;"
DATA "    let e = length(p - ec) - 0.42;"
DATA "    if (e < d) { d = e; oMat = 18; oCtr = ec; }"
DATA "    return d;"
DATA "}"
DATA ""
DATA "fn oShard(pin: vec3f, h: f32) -> f32 {"
DATA "    let a1 = rot2(pin.xy, h * 6.28 + T * 0.4);"
DATA "    var p = vec3f(a1.x, a1.y, pin.z);"
DATA "    let a2 = rot2(p.zx, h * 4.4 + T * 0.28);"
DATA "    p = vec3f(a2.y, p.y, a2.x);"
DATA "    oMat = 19;"
DATA "    oLoc = p;"
DATA "    return box3(p, vec3f(1.2 + h * 0.6, 0.08, 0.9)) * 0.8;"
DATA "}"
DATA ""
DATA "fn oGlitch(p: vec3f, h: f32) -> f32 {"
DATA "    let id = clamp(round(p / 2.2), vec3f(-1.0), vec3f(1.0));"
DATA "    let q = p - id * 2.2;"
DATA "    let hh = hs(dot(id, vec3f(3.1, 7.7, 1.3)) + h * 17.0 + floor(T * 4.0) * 0.37);"
DATA "    let s = 0.12 + 0.3 * hh;"
DATA "    oMat = 20;"
DATA "    oLoc = vec3f(hh, 0.0, 0.0);"
DATA "    if (hh < 0.35) { return max(box3(q, vec3f(1.0)), 0.1); }"
DATA "    return box3(q, vec3f(s));"
DATA "}"
DATA ""
DATA "fn oBigEye(p: vec3f) -> f32 {"
DATA "    oMat = 18;"
DATA "    oLoc = p;"
DATA "    oCtr = vec3f(0.0);"
DATA "    return length(p) - 1.3;"
DATA "}"
DATA ""
DATA "fn smin(a: f32, b: f32, k: f32) -> f32 {"
DATA "    let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);"
DATA "    return mix(b, a, h) - k * h * (1.0 - h);"
DATA "}"
DATA ""
DATA "fn boxFrame(p: vec3f, b: vec3f, e: f32) -> f32 {"
DATA "    let q = abs(p) - b;"
DATA "    let w = abs(q + e) - e;"
DATA "    let a = length(max(vec3f(q.x, w.y, w.z), vec3f(0.0))) + min(max(q.x, max(w.y, w.z)), 0.0);"
DATA "    let c = length(max(vec3f(w.x, q.y, w.z), vec3f(0.0))) + min(max(w.x, max(q.y, w.z)), 0.0);"
DATA "    let d = length(max(vec3f(w.x, w.y, q.z), vec3f(0.0))) + min(max(w.x, max(w.y, q.z)), 0.0);"
DATA "    return min(a, min(c, d));"
DATA "}"
DATA ""
DATA "fn spinY(p: vec3f, a: f32) -> vec3f {"
DATA "    let r = rot2(p.xz, a);"
DATA "    return vec3f(r.x, p.y, r.y);"
DATA "}"
DATA ""
DATA "fn spinX(p: vec3f, a: f32) -> vec3f {"
DATA "    let r = rot2(p.yz, a);"
DATA "    return vec3f(p.x, r.x, r.y);"
DATA "}"
DATA ""
DATA "// an old TV showing static, screen toward the path"
DATA "fn oTV(pin: vec3f, side: f32) -> f32 {"
DATA "    let p = pin / 1.6;"
DATA "    var d = box3(p, vec3f(0.62, 0.5, 0.55)) - 0.06;"
DATA "    oMat = 35;"
DATA "    oLoc = p;"
DATA "    let scr = box3(p - vec3f(side * 0.66, 0.02, 0.0), vec3f(0.04, 0.38, 0.44));"
DATA "    if (scr < d) { d = scr; oMat = 26; }"
DATA "    let aq = vec3f(p.x, p.y, abs(p.z));"
DATA "    let ant = capsule(aq, vec3f(0.0, 0.55, 0.0), vec3f(0.0, 1.05, 0.38), 0.015);"
DATA "    if (ant < d) { d = ant; oMat = 9; }"
DATA "    return d * 1.6;"
DATA "}"
DATA ""
DATA "fn oLamp(p: vec3f, side: f32) -> f32 {"
DATA "    let g = -3.6;"
DATA "    var d = capsule(p, vec3f(0.0, g, 0.0), vec3f(0.0, g + 5.6, 0.0), 0.08);"
DATA "    d = min(d, capsule(p, vec3f(0.0, g + 5.6, 0.0), vec3f(side * 1.3, g + 5.8, 0.0), 0.06));"
DATA "    oMat = 19;"
DATA "    oLoc = p;"
DATA "    let head = box3(p - vec3f(side * 1.3, g + 5.72, 0.0), vec3f(0.35, 0.1, 0.2));"
DATA "    if (head < d) { d = head; oMat = 19; }"
DATA "    let bulb = box3(p - vec3f(side * 1.3, g + 5.58, 0.0), vec3f(0.24, 0.04, 0.13));"
DATA "    if (bulb < d) { d = bulb; oMat = 3; }"
DATA "    return d;"
DATA "}"
DATA ""
DATA "fn oHouse(p: vec3f, side: f32) -> f32 {"
DATA "    let g = -3.6;"
DATA "    var d = box3(p - vec3f(0.0, g + 1.4, 0.0), vec3f(2.0, 1.4, 2.6));"
DATA "    oMat = 22;"
DATA "    oLoc = p;"
DATA "    let q = p - vec3f(0.0, g + 2.8, 0.0);"
DATA "    let roof = max(abs(q.z) - 2.85, max(0.6 * abs(q.x) + 0.8 * q.y - 1.3, -q.y));"
DATA "    if (roof < d) { d = roof; oMat = 13; }"
DATA "    let win = box3(vec3f(p.x - side * 2.0, p.y - (g + 1.7), abs(p.z) - 1.4), vec3f(0.05, 0.42, 0.42));"
DATA "    if (win < d) { d = win; oMat = 3; }"
DATA "    let door = box3(p - vec3f(side * 2.0, g + 0.8, 0.0), vec3f(0.05, 0.8, 0.45));"
DATA "    if (door < d) { d = door; oMat = 33; }"
DATA "    return d;"
DATA "}"
DATA ""
DATA "fn oBed(p: vec3f) -> f32 {"
DATA "    let g = -3.6;"
DATA "    var d = box3(p - vec3f(0.0, g + 0.25, 0.0), vec3f(1.1, 0.25, 2.0));"
DATA "    oMat = 12;"
DATA "    oLoc = p;"
DATA "    let head = box3(p - vec3f(0.0, g + 0.9, -2.0), vec3f(1.1, 0.9, 0.08));"
DATA "    d = min(d, head);"
DATA "    var m = box3(p - vec3f(0.0, g + 0.62, 0.05), vec3f(1.0, 0.13, 1.9)) - 0.04;"
DATA "    m = min(m, box3(p - vec3f(0.0, g + 0.84, -1.45), vec3f(0.62, 0.08, 0.28)) - 0.05);"
DATA "    if (m < d) { d = m; oMat = 15; }"
DATA "    return d;"
DATA "}"
DATA ""
DATA "fn oStraightStairs(pin: vec3f, h: f32) -> f32 {"
DATA "    let p = spinY(pin, (h - 0.5) * 1.2);"
DATA "    let k0 = clamp(floor((p.z + 3.3) / 0.6), 0.0, 11.0);"
DATA "    var d = 100.0;"
DATA "    for (var n = 0; n < 2; n++) {"
DATA "        let k = min(k0 + f32(n), 11.0);"
DATA "        d = min(d, box3(p - vec3f(0.0, -1.8 + k * 0.3, -3.3 + k * 0.6), vec3f(0.9, 0.16, 0.3)));"
DATA "    }"
DATA "    oMat = 22;"
DATA "    oLoc = p;"
DATA "    return d;"
DATA "}"
DATA ""
DATA "fn oAqueduct(p: vec3f) -> f32 {"
DATA "    var d = box3(p - vec3f(0.0, 1.3, 0.0), vec3f(0.95, 0.22, 4.4));"
DATA "    let wall = box3(p - vec3f(0.0, -1.0, 0.0), vec3f(0.6, 2.1, 4.4));"
DATA "    let qz = p.z - 2.2 * clamp(round(p.z / 2.2), -1.0, 1.0);"
DATA "    let opening = min(length(vec2f(qz, p.y + 0.5)) - 0.75, box3(vec3f(0.0, p.y + 2.3, qz), vec3f(1.0, 1.8, 0.75)));"
DATA "    d = min(d, max(wall, -opening));"
DATA "    oMat = 22;"
DATA "    oLoc = p;"
DATA "    return d;"
DATA "}"
DATA ""
DATA "fn oFrame(pin: vec3f, h: f32) -> f32 {"
DATA "    var p = spinY(pin, h * 6.28 + T * 0.1);"
DATA "    p = spinX(p, (hs(h * 3.0) - 0.5) * 0.8);"
DATA "    oMat = 22;"
DATA "    oLoc = p;"
DATA "    return max(box3(p, vec3f(0.95, 1.55, 0.08)), -box3(p, vec3f(0.75, 1.38, 0.2)));"
DATA "}"
DATA ""
DATA "fn oSpiral(pin: vec3f, h: f32) -> f32 {"
DATA "    let p = spinY(pin, h * 6.28);"
DATA "    let r = length(p.xy);"
DATA "    let a = 2.6 / (6.2831853 * 3.5);"
DATA "    let th = atan2(p.y, p.x) + 3.14159265 + T * 0.6;"
DATA "    let k = clamp(round((r / a - th) / 6.2831853), 0.0, 3.0);"
DATA "    let dr = r - a * (th + 6.2831853 * k);"
DATA "    oMat = 9;"
DATA "    oLoc = p;"
DATA "    return max(length(vec2f(dr, p.z)) - 0.13, r - 2.75) * 0.7;"
DATA "}"
DATA ""
DATA "fn oSphere(p: vec3f, h: f32) -> f32 {"
DATA "    oMat = 29;"
DATA "    oLoc = p;"
DATA "    return length(p - vec3f(0.0, 0.6 * sin(T * 0.7 + h * 9.0), 0.0)) - (1.1 + 0.6 * h);"
DATA "}"
DATA ""
DATA "fn oWireCubes(pin: vec3f, h: f32) -> f32 {"
DATA "    var d = 100.0;"
DATA "    for (var i = 0; i < 3; i++) {"
DATA "        let s = 2.0 - f32(i) * 0.6;"
DATA "        var p = spinY(pin, T * (0.2 + 0.1 * f32(i)) + h * 5.0 + f32(i));"
DATA "        p = spinX(p, T * (0.13 - 0.05 * f32(i)) + f32(i) * 0.7);"
DATA "        d = min(d, boxFrame(p, vec3f(s), 0.07));"
DATA "    }"
DATA "    oMat = 9;"
DATA "    oLoc = pin;"
DATA "    return d;"
DATA "}"
DATA ""
DATA "fn oPortal(pin: vec3f, h: f32) -> f32 {"
DATA "    let p = pin - vec3f(0.0, 0.4 * sin(T * 0.5 + h * 7.0), 0.0);"
DATA "    let ring = length(vec2f(length(p.yz) - 1.6, p.x)) - 0.14;"
DATA "    oMat = 30;"
DATA "    oLoc = p;"
DATA "    var d = ring;"
DATA "    let disk = max(length(p.yz) - 1.45, abs(p.x) - 0.015);"
DATA "    if (disk < d) { d = disk; oMat = 28; }"
DATA "    return d;"
DATA "}"
DATA ""
DATA "fn mengerAt(p: vec3f, s: f32) -> f32 {"
DATA "    let q = p / s;"
DATA "    var d = box3(q, vec3f(1.0));"
DATA "    var sc = 1.0;"
DATA "    for (var m = 0; m < 3; m++) {"
DATA "        let a = q * sc - 2.0 * floor(q * sc / 2.0) - 1.0;"
DATA "        sc = sc * 3.0;"
DATA "        let rr = abs(1.0 - 3.0 * abs(a));"
DATA "        let da = max(rr.x, rr.y);"
DATA "        let db = max(rr.y, rr.z);"
DATA "        let dc = max(rr.z, rr.x);"
DATA "        d = max(d, (min(da, min(db, dc)) - 1.0) / sc);"
DATA "    }"
DATA "    return d * s;"
DATA "}"
DATA ""
DATA "fn oTower(p: vec3f) -> f32 {"
DATA "    let g = -3.6;"
DATA "    let k = clamp(round((p.y - (g + 1.8)) / 3.6), 0.0, 1.0);"
DATA "    oMat = 31;"
DATA "    oLoc = p;"
DATA "    return mengerAt(p - vec3f(0.0, g + 1.8 + k * 3.6, 0.0), 1.8);"
DATA "}"
DATA ""
DATA "fn oBlob(pin: vec3f, h: f32) -> f32 {"
DATA "    var d = 100.0;"
DATA "    for (var i = 0; i < 4; i++) {"
DATA "        let fi = f32(i);"
DATA "        let c = 1.1 * vec3f(sin(T * 0.5 + fi * 2.0 + h * 9.0), 0.6 * cos(T * 0.4 + fi * 1.3), sin(T * 0.3 + fi * 2.7));"
DATA "        d = smin(d, length(pin - c) - (0.75 + 0.15 * fi), 0.6);"
DATA "    }"
DATA "    for (var j = 0; j < 4; j++) {"
DATA "        let x = -0.9 + f32(j) * 0.6;"
DATA "        let len = 1.4 + 1.2 * (0.5 + 0.5 * sin(T * 0.6 + f32(j) * 2.1 + h * 5.0));"
DATA "        d = smin(d, capsule(pin, vec3f(x, -0.3, 0.2 * f32(j) - 0.3), vec3f(x, -0.3 - len, 0.2 * f32(j) - 0.3), 0.13), 0.35);"
DATA "    }"
DATA "    oMat = 29;"
DATA "    oLoc = pin;"
DATA "    return d * 0.85;"
DATA "}"
DATA ""
DATA "fn oCrystal(pin: vec3f, h: f32) -> f32 {"
DATA "    var p = spinY(pin, T * 0.3 + h * 6.0);"
DATA "    p = spinX(p, T * 0.17 + h * 3.0);"
DATA "    let q = abs(p);"
DATA "    let s = 2.2;"
DATA "    var d = capsule(q, vec3f(s, 0.0, 0.0), vec3f(0.0, s, 0.0), 0.05);"
DATA "    d = min(d, capsule(q, vec3f(0.0, s, 0.0), vec3f(0.0, 0.0, s), 0.05));"
DATA "    d = min(d, capsule(q, vec3f(0.0, 0.0, s), vec3f(s, 0.0, 0.0), 0.05));"
DATA "    oMat = 30;"
DATA "    oLoc = p;"
DATA "    let core = (q.x + q.y + q.z - 0.7) * 0.577;"
DATA "    if (core < d) { d = core; oMat = 28; }"
DATA "    return d;"
DATA "}"
DATA ""
DATA "// a tiled wall with two red doors and an EXIT sign over each"
DATA "fn oExitWall(p: vec3f, side: f32) -> f32 {"
DATA "    let g = -3.6;"
DATA "    let zq = abs(p.z) - 1.8;"
DATA "    var d = box3(p - vec3f(0.0, g + 1.6, 0.0), vec3f(0.25, 1.6, 4.0));"
DATA "    d = max(d, -box3(vec3f(p.x - side * 0.25, p.y - (g + 1.05), zq), vec3f(0.12, 1.05, 0.55)));"
DATA "    oMat = 32;"
DATA "    oLoc = p;"
DATA "    let door = box3(vec3f(p.x - side * 0.17, p.y - (g + 1.0), zq), vec3f(0.03, 1.0, 0.5));"
DATA "    if (door < d) { d = door; oMat = 33; }"
DATA "    let sc = vec3f(p.x - side * 0.31, p.y - (g + 2.45), zq);"
DATA "    let sg = box3(sc, vec3f(0.06, 0.17, 0.5));"
DATA "    if (sg < d) { d = sg; oMat = 7; oLoc = vec3f(-sc.z * sign(p.z) * side, sc.y, sc.x * side); }"
DATA "    return d;"
DATA "}"
DATA ""
DATA "fn piece(o: i32, p: vec3f, h: f32, side: f32) -> f32 {"
DATA "    oCtr = vec3f(0.0);"
DATA "    var d = 100.0;"
DATA "    switch o {"
DATA "        case 1: { d = oPool(p); }"
DATA "        case 2: { d = oSwing(p, h); }"
DATA "        case 3: { d = oSlide(p); }"
DATA "        case 4: { d = oDoor(p, h); }"
DATA "        case 5: { d = oExitPost(p); }"
DATA "        case 6: { d = oStair(p, h); }"
DATA "        case 7: { d = oCorridor(p); }"
DATA "        case 8: { d = oDesk(p, h); }"
DATA "        case 9: { d = oBoard(p, h); }"
DATA "        case 10: { d = oColumn(p, h); }"
DATA "        case 11: { d = oMeltArch(p, h); }"
DATA "        case 12: { d = oMenger(p, h); }"
DATA "        case 13: { d = oEyePillar(p, side); }"
DATA "        case 14: { d = oShard(p, h); }"
DATA "        case 15: { d = oGlitch(p, h); }"
DATA "        case 16: { d = oBigEye(p); }"
DATA "        case 17: { d = oTV(p, side); }"
DATA "        case 18: { d = oLamp(p, side); }"
DATA "        case 19: { d = oHouse(p, side); }"
DATA "        case 20: { d = oBed(p); }"
DATA "        case 21: { d = oStraightStairs(p, h); }"
DATA "        case 22: { d = oAqueduct(p); }"
DATA "        case 23: { d = oFrame(p, h); }"
DATA "        case 24: { d = oSpiral(p, h); }"
DATA "        case 25: { d = oSphere(p, h); }"
DATA "        case 26: { d = oWireCubes(p, h); }"
DATA "        case 27: { d = oPortal(p, h); }"
DATA "        case 28: { d = oTower(p); }"
DATA "        case 29: { d = oBlob(p, h); }"
DATA "        case 30: { d = oCrystal(p, h); }"
DATA "        default: { d = oExitWall(p, side); }"
DATA "    }"
DATA "    return d;"
DATA "}"
DATA ""
DATA "// one piece per cell; rows: 1 = one row centred at y0, 3 = rows at y0 - 9,"
DATA "// y0, y0 + 9. Columns |ix| < mi stay empty (the clear tube), except the rows"
DATA "// above and below the path and the upper row (allowAll)."
DATA "fn cells(p: vec3f, oa: i32, ob: i32, mx: f32, em: f32, mi: f32, rows: i32, y0: f32, flip: bool, allowAll: bool, seed: f32, best: f32) -> f32 {"
DATA "    let cw = 9.0;"
DATA "    let czs = 10.0;"
DATA "    let chh = 9.0;"
DATA "    let ix = round(p.x / cw);"
DATA "    let iz = round(p.z / czs);"
DATA "    var iy = 0.0;"
DATA "    if (rows == 3) { iy = clamp(round((p.y - y0) / chh), -1.0, 1.0); }"
DATA "    let ctr = vec3f(ix * cw, y0 + iy * chh, iz * czs);"
DATA "    var pl = p - ctr;"
DATA "    var bnd = min((cw * 0.5 - abs(pl.x)) / (abs(rdir.x) + 0.001), (czs * 0.5 - abs(pl.z)) / (abs(rdir.z) + 0.001));"
DATA "    if (rows == 3 && abs(pl.y) < chh * 0.5) { bnd = min(bnd, (chh * 0.5 - abs(pl.y)) / (abs(rdir.y) + 0.001)); }"
DATA "    // pieces keep 0.6 from their cell walls: stepping 0.5 past the wall is safe"
DATA "    bnd = max(bnd, 0.0) * (1.0 - 0.5 * min(mlt, 1.0)) + 0.5;"
DATA "    if (!useBnd) { bnd = 1000.0; }"
DATA "    // cells are laid out in world x, so every piece stays rigid; the path"
DATA "    // drifts by at most 0.9 over a cell, the watcher's tube needs 3.4 on each"
DATA "    // side and a piece reaches 3.6 from its centre: a cell nearer than 8 to"
DATA "    // the path (12.5 beside the arches) stays empty"
DATA "    let off = abs(ctr.x - pathX(ctr.z));"
DATA "    let offP = abs(p.x - pathX(p.z));"
DATA "    if (offP > 6.6 * cw) { return select(1000.0, (offP - 6.6 * cw) * 0.9 + 0.5, useBnd); }"
DATA "    var ok = off >= select(8.0, 12.5, mi > 1.5);"
DATA "    if (rows == 3 && iy != 0.0) { ok = true; }"
DATA "    if (allowAll) { ok = true; }"
DATA "    if (!ok) { return bnd; }"
DATA "    let h = hs(ix * 7.13 + iy * 3.71 + iz * 1.37 + seed);"
DATA "    if (hs(h * 91.7 + 3.1) < em) { return bnd; }"
DATA "    // the piece lies inside this box: when the box is farther than the cell"
DATA "    // wall or than what the caller already has, the piece cannot be nearer"
DATA "    let db = box3(pl, vec3f(3.7, 3.7, 4.5));"
DATA "    if (db > bnd) { return bnd; }"
DATA "    if (db > best) { return db; }"
DATA "    var o = ob;"
DATA "    if (h < mx) { o = oa; }"
DATA "    if (flip) { pl.y = -pl.y; }"
DATA "    let side = -sign(ctr.x - pathX(ctr.z) + 0.0001);"
DATA "    let d = piece(o, pl, h, side);"
DATA "    cMat = oMat;"
DATA "    cLoc = oLoc;"
DATA "    var oc = oCtr;"
DATA "    if (flip) { oc.y = -oc.y; }"
DATA "    cCtr = ctr + oc;"
DATA "    return min(d, bnd);"
DATA "}"
DATA ""
DATA "// ---- structures around the path ----"
DATA "fn sdHall(p: vec3f) -> f32 {"
DATA "    var d = 9.6 - p.y;"
DATA "    cMat = 19;"
DATA "    cLoc = p;"
DATA "    let zc = rp(p.z, 7.0);"
DATA "    let wall = max(6.5 - abs(p.x), -box3(vec3f(abs(p.x) - 6.5, p.y - 1.6, zc), vec3f(0.25, 1.6, 0.8)));"
DATA "    if (wall < d) { d = wall; cMat = 21; }"
DATA "    let door = box3(vec3f(abs(p.x) - 6.7, p.y - 1.55, zc), vec3f(0.05, 1.55, 0.75));"
DATA "    if (door < d) { d = door; cMat = 15; }"
DATA "    let lamp = box3(vec3f(p.x, p.y - 9.55, rp(p.z - 2.5, 5.0)), vec3f(1.4, 0.05, 0.5));"
DATA "    if (lamp < d) { d = lamp; cMat = 3; }"
DATA "    return d;"
DATA "}"
DATA ""
DATA "fn sdArches(pw: vec3f) -> f32 {"
DATA "    let k = round(pw.z / 6.0);"
DATA "    let zc = pw.z - k * 6.0;"
DATA "    let p = vec3f(pw.x - pathX(k * 6.0), pw.y, pw.z);"
DATA "    var ri = 5.6;"
DATA "    if (fract(k * 0.5) > 0.25) { ri = 6.3; }"
DATA "    let cy = 4.0;"
DATA "    let hy = (cy + ri + 1.2) * 0.5;"
DATA "    let opening = min(box3(vec3f(p.x, p.y - cy * 0.5 + 0.3, zc), vec3f(ri, cy * 0.5 + 0.3, 1.0)), length(vec2f(p.x, p.y - cy)) - ri);"
DATA "    cMat = 22;"
DATA "    cLoc = p;"
DATA "    return max(box3(vec3f(p.x, p.y - hy, zc), vec3f(ri + 1.3, hy, 0.45)), -opening);"
DATA "}"
DATA ""
DATA "fn world(k: i32, K: KI, p: vec3f, pw: vec3f) -> f32 {"
DATA "    var d = 1000.0;"
DATA "    var m = 0;"
DATA "    var loc = pw;"
DATA "    var ctr = vec3f(0.0);"
DATA "    if (K.fl > 0) {"
DATA "        if (K.fl == 2) {"
DATA "            d = (pw.y + 0.16 * (1.0 + sin(pw.x * 0.3) * sin(pw.z * 0.25))) * 0.9;"
DATA "            m = 10;"
DATA "        } else {"
DATA "            d = pw.y;"
DATA "            m = 8;"
DATA "            if (K.fl == 3) { m = 17; }"
DATA "            if (K.fl == 4) { m = 14; }"
DATA "            if (K.fl == 5) { m = 23; }"
DATA "            if (K.fl == 6) { m = 4; }"
DATA "            if (K.fl == 7) { m = 25; }"
DATA "            if (K.fl == 8) { m = 34; }"
DATA "            if (K.fl == 9) { m = 31; }"
DATA "        }"
DATA "    }"
DATA "    if (K.sp == 1) {"
DATA "        let s = sdHall(p);"
DATA "        if (s < d) { d = s; m = cMat; loc = cLoc; }"
DATA "    } else if (K.sp == 2) {"
DATA "        let s = sdArches(pw);"
DATA "        if (s < d) { d = s; m = cMat; loc = cLoc; }"
DATA "    } else if (K.sp == 3) {"
DATA "        let s = 11.0 - p.y;"
DATA "        if (s < d) { d = s; m = 8; loc = pw; }"
DATA "    }"
DATA "    // rows that are farther away than what is already found are skipped:"
DATA "    // grounded pieces reach y = 7.2, hanging ones come down to y = 3.8,"
DATA "    // the upper row starts at y = 9"
DATA "    var rowNear = true;"
DATA "    if (K.md == 0 && p.y - 7.2 > d) { rowNear = false; }"
DATA "    if (K.md == 2 && 3.8 - p.y > d) { rowNear = false; }"
DATA "    if (K.md >= 0 && rowNear) {"
DATA "        var cd = 0.0;"
DATA "        if (K.md == 0) {"
DATA "            cd = cells(pw, K.oa, K.ob, K.mx, K.em, K.mi, 1, 3.6, false, false, f32(k) * 1.9, d);"
DATA "        } else if (K.md == 1) {"
DATA "            cd = cells(pw, K.oa, K.ob, K.mx, K.em, K.mi, 3, 4.0, false, false, f32(k) * 1.9, d);"
DATA "        } else {"
DATA "            cd = cells(pw, K.oa, K.ob, K.mx, K.em, K.mi, 1, 7.4, true, false, f32(k) * 1.9, d);"
DATA "        }"
DATA "        if (cd < d) { d = cd; m = cMat; loc = cLoc; ctr = cCtr; }"
DATA "    }"
DATA "    if (K.up > 0 && 9.0 - p.y < d) {"
DATA "        let ud = cells(pw, K.up, K.up, 1.0, 0.45, 0.0, 1, 12.6, false, true, f32(k) * 2.7 + 5.0, d);"
DATA "        if (ud < d) { d = ud; m = cMat; loc = cLoc; ctr = cCtr; }"
DATA "    }"
DATA "    gMat = m;"
DATA "    gLoc = loc;"
DATA "    gCtr = ctr;"
DATA "    return d;"
DATA "}"
DATA ""
DATA "// ---- the watcher ----"
DATA "// tentacles behind the eye (away from the camera), spreading like a cone"
DATA "fn tentacles(e: vec3f) -> f32 {"
DATA "    let c = dot(e, gz);"
DATA "    let a1 = dot(e, tb1);"
DATA "    let a2 = dot(e, tb2);"
DATA "    let r = length(vec2f(a1, a2));"
DATA "    let ang = atan2(a2, a1) + T * 0.15;"
DATA "    let sect = 1.0471976;"
DATA "    let ai = floor(ang / sect + 0.5);"
DATA "    let aw = ang - ai * sect;"
DATA "    let s = max((-c - 0.2 * eyeR) / eyeR, 0.0);"
DATA "    let rc = eyeR * (0.55 + 0.38 * s);"
DATA "    let w1 = 0.32 * eyeR * sin(s * 2.1 - T * 1.9 + ai * 1.7) * s * 0.5;"
DATA "    let w2 = 0.3 * eyeR * cos(s * 1.7 - T * 1.5 + ai * 2.3) * s * 0.5;"
DATA "    let x = r * cos(aw) - rc - w1;"
DATA "    let y = r * sin(aw) - w2;"
DATA "    let rad = eyeR * 0.17 * (1.0 - s * 0.25);"
DATA "    return max((length(vec2f(x, y)) - rad) * 0.55, abs(c + eyeR * 1.9) - eyeR * 1.7);"
DATA "}"
DATA ""
DATA "// a ring of small eyes in front of the watcher (polar domain repetition)"
DATA "fn satRing(e: vec3f, n: f32, ringR: f32, off: f32, sz: f32, spin: f32) -> vec4f {"
DATA "    let c = dot(e, gz) - off;"
DATA "    let a1 = dot(e, tb1);"
DATA "    let a2 = dot(e, tb2);"
DATA "    let ang = atan2(a2, a1) + spin;"
DATA "    let sect = 6.2831853 / n;"
DATA "    let ai = floor(ang / sect + 0.5);"
DATA "    let aw = ang - ai * sect;"
DATA "    let r = length(vec2f(a1, a2));"
DATA "    let dd = length(vec3f(r * cos(aw) - ringR, r * sin(aw), c)) - sz;"
DATA "    let ca = ai * sect - spin;"
DATA "    let ctr = eyeP + (tb1 * cos(ca) + tb2 * sin(ca)) * ringR + gz * off;"
DATA "    return vec4f(dd, ctr);"
DATA "}"
DATA ""
DATA "fn mapd(pw: vec3f) -> f32 {"
DATA "    let lx = pw.x - pathX(pw.z);"
DATA "    var q = vec3f(lx, pw.y, pw.z);"
DATA "    if (mlt > 0.002) {"
DATA "        // melting: 3D sine noise pulls the pieces down like wax"
DATA "        let n3 = sin(lx * 0.9 + sin(pw.z * 0.7 + T)) * sin(pw.y * 1.1 + sin(lx * 0.6 - T * 0.7)) * sin(pw.z * 0.8 + sin(pw.y * 0.7 + T * 0.5));"
DATA "        q.y = pw.y + mlt * (0.5 * n3 + 0.5 * abs(sin(lx * 1.3 + T * 0.3) * sin(pw.z * 1.1 - T * 0.2)));"
DATA "        q.x = lx + mlt * 0.25 * sin(pw.y * 1.3 + T);"
DATA "    }"
DATA "    let qw = vec3f(q.x + pathX(pw.z), q.y, q.z);"
DATA "    // a transition melts the old world down, swaps under the white flash at"
DATA "    // its peak and lets the new one solidify: one world to evaluate at a time"
DATA "    var d = 0.0;"
DATA "    gSub = 0;"
DATA "    if (bl < 0.5) {"
DATA "        d = world(kA, KIa, q, qw);"
DATA "    } else {"
DATA "        d = world(kB, KIb, q, qw);"
DATA "        gSub = 1;"
DATA "    }"
DATA "    var m = gMat;"
DATA "    var loc = gLoc;"
DATA "    var ctr = gCtr;"
DATA "    // the last worlds break apart into dust"
DATA "    if (dissolve > 0.001) {"
DATA "        let hv = hash3(floor(qw * 4.0));"
DATA "        d = max(d, (dissolve - hv) * 0.5);"
DATA "    }"
DATA "    // a black figure with white eyes standing on the path behind the watcher"
DATA "    if (figOn) {"
DATA "        let fz = eyeP.z - 10.0;"
DATA "        var fy = 0.0;"
DATA "        if (KIa.fl == 0) { fy = pathY(fz) - 3.6; }"
DATA "        let fq = pw - vec3f(pathX(fz) + 2.2, fy, fz);"
DATA "        let fx = vec3f(abs(fq.x), fq.y, fq.z);"
DATA "        var fd = capsule(fq, vec3f(0.0, 0.95, 0.0), vec3f(0.0, 2.5, 0.0), 0.3);"
DATA "        fd = min(fd, capsule(fx, vec3f(0.12, 0.0, 0.0), vec3f(0.12, 1.0, 0.0), 0.1));"
DATA "        fd = min(fd, capsule(fx, vec3f(0.36, 2.4, 0.0), vec3f(0.44, 1.15, 0.05), 0.07));"
DATA "        fd = min(fd, length(fq - vec3f(0.0, 3.0, 0.0)) - 0.26);"
DATA "        if (fd < d) { d = fd; m = 28; }"
DATA "        let fe = length(fx - vec3f(0.09, 3.03, 0.22)) - 0.045;"
DATA "        if (fe < d) { d = fe; m = 27; }"
DATA "    }"
DATA "    let e = pw - eyeP;"
DATA "    let rr = length(e);"
DATA "    gEyeD = rr - eyeR;"
DATA "    if (rr - eyeR * 1.15 < d) {"
DATA "        var db = rr - eyeR;"
DATA "        let c = dot(e, gz) / max(rr, 0.0001);"
DATA "        if (c > 0.78) { db = db - (c - 0.78) * eyeR * 0.45; }"
DATA "        if (db < d) { d = db; m = 1; ctr = eyeP; }"
DATA "    }"
DATA "    if (rr - eyeR * 4.3 < d) {"
DATA "        if (dot(e, gz) < -0.1 * eyeR) {"
DATA "            let dt = tentacles(e);"
DATA "            if (dt < d) { d = dt; m = 5; }"
DATA "        }"
DATA "        if (sat1 > 0.0) {"
DATA "            let s1 = satRing(e, 5.0, eyeR * 1.75, eyeR * 0.35, eyeR * 0.28 * sat1, T * 0.25);"
DATA "            if (s1.x < d) { d = s1.x; m = 2; ctr = s1.yzw; }"
DATA "        }"
DATA "        if (sat2 > 0.0) {"
DATA "            let s2 = satRing(e, 9.0, eyeR * 2.6, eyeR * 0.1, eyeR * 0.22 * sat2, -T * 0.18);"
DATA "            if (s2.x < d) { d = s2.x; m = 2; ctr = s2.yzw; }"
DATA "        }"
DATA "    }"
DATA "    gMat = m;"
DATA "    gLoc = loc;"
DATA "    gCtr = ctr;"
DATA "    return d;"
DATA "}"
DATA ""
DATA "// ---- colour ----"
DATA "// the sky's colour gradient alone"
DATA "fn skyGrad(K: KI, rd: vec3f) -> vec3f {"
DATA "    let h = rd.y;"
DATA "    if (K.sky == 2) { return vec3f(45.0, 3.0, 10.0); }"
DATA "    if (K.sky == 3) { return vec3f(255.0, 252.0, 255.0) - vec3f(10.0, 20.0, 0.0) * (1.0 - abs(h)); }"
DATA "    var top = vec3f(105.0, 62.0, 185.0);"
DATA "    var hor2 = vec3f(238.0, 168.0, 222.0);"
DATA "    switch K.sky {"
DATA "        case 1: { top = vec3f(55.0, 30.0, 95.0); hor2 = vec3f(190.0, 120.0, 170.0); }"
DATA "        case 4: { top = vec3f(8.0, 10.0, 36.0); hor2 = vec3f(58.0, 48.0, 108.0); }"
DATA "        case 5: { top = vec3f(105.0, 58.0, 150.0); hor2 = vec3f(255.0, 155.0, 125.0); }"
DATA "        case 6: { top = vec3f(80.0, 160.0, 205.0); hor2 = vec3f(215.0, 240.0, 235.0); }"
DATA "        case 7: { top = vec3f(150.0, 170.0, 235.0); hor2 = vec3f(240.0, 200.0, 235.0); }"
DATA "        case 8: { top = vec3f(150.0, 140.0, 175.0); hor2 = vec3f(235.0, 205.0, 175.0); }"
DATA "        case 9: { top = vec3f(16.0, 8.0, 34.0); hor2 = vec3f(82.0, 42.0, 112.0); }"
DATA "        default: { }"
DATA "    }"
DATA "    return mix(hor2, top, clamp(h * 1.5 + 0.25, 0.0, 1.0));"
DATA "}"
DATA ""
DATA "fn skyCol(K: KI, rd: vec3f) -> vec3f {"
DATA "    var c: vec3f;"
DATA "    let h = rd.y;"
DATA "    if (K.sky == 2) {"
DATA "        let w = sin(h * 9.0 + T * 1.3 + sin(rd.x * 5.0 + T * 0.7) * 2.0);"
DATA "        c = mix(vec3f(10.0, 0.0, 6.0), vec3f(150.0, 8.0, 18.0), smoothstep(-0.4, 1.0, w) * (0.55 + 0.45 * smoothstep(-0.3, 0.5, h)));"
DATA "        let st = hash3(floor(rd * 300.0) + vec3f(floor(T * 20.0)));"
DATA "        if (st < 0.03) { c = vec3f(hash3(floor(rd * 211.0)) * 120.0); }"
DATA "        return c;"
DATA "    }"
DATA "    c = skyGrad(K, rd);"
DATA "    if (K.sky == 3) { return c; }"
DATA "    if (K.sky == 7) {"
DATA "        // iridescent sky: rainbow bands drifting through it"
DATA "        let t = rd.y * 1.6 + rd.x * 0.5 + fbm(rd * 2.0 + vec3f(T * 0.02)) * 0.8;"
DATA "        c = mix(c, iri(t) * 255.0, 0.35);"
DATA "    }"
DATA "    if (K.sky == 4) {"
DATA "        let md = normalize(vec3f(0.35, 0.3, -1.0));"
DATA "        let mc = dot(rd, md);"
DATA "        if (mc > 0.9988) { c = vec3f(235.0, 232.0, 250.0) * (0.85 + 0.15 * vnoise(rd * 400.0)); }"
DATA "        c = c + vec3f(60.0, 60.0, 90.0) * pow(max(mc, 0.0), 300.0);"
DATA "    }"
DATA "    if (K.sky == 5 || K.sky == 8) {"
DATA "        let sd = normalize(vec3f(0.25, 0.2, -1.0));"
DATA "        let sc = dot(rd, sd);"
DATA "        if (sc > 0.9975) { c = vec3f(255.0, 238.0, 215.0); }"
DATA "        c = c + vec3f(120.0, 80.0, 50.0) * pow(max(sc, 0.0), 60.0);"
DATA "    }"
DATA "    if (K.sky == 8) {"
DATA "        // a giant eye in the sky, watching the camera"
DATA "        let ed = normalize(vec3f(-0.4, 0.3, -1.0));"
DATA "        let ec = dot(rd, ed);"
DATA "        let rad = 0.16;"
DATA "        let ang = acos(clamp(ec, -1.0, 1.0));"
DATA "        if (ang < rad) {"
DATA "            let rho = ang / rad;"
DATA "            let rx = normalize(cross(ed, vec3f(0.0, 1.0, 0.0)));"
DATA "            let ry = cross(rx, ed);"
DATA "            let a2 = atan2(dot(rd, ry), dot(rd, rx));"
DATA "            if (rho < 0.22) { c = vec3f(12.0, 6.0, 10.0); }"
DATA "            else if (rho < 0.5) { c = vec3f(120.0, 140.0, 165.0) * (0.6 + 0.4 * sin(a2 * 31.0)); }"
DATA "            else {"
DATA "                c = vec3f(238.0, 215.0, 210.0) * (1.0 - 0.25 * rho * rho);"
DATA "                if (abs(sin(a2 * 9.0 + rho * 14.0)) < 0.05 + 0.1 * rho) { c = vec3f(170.0, 30.0, 40.0); }"
DATA "            }"
DATA "            if (dot(rd, ry) > (1.0 - 2.05 * lid) * sin(rad)) { c = vec3f(170.0, 120.0, 125.0); }"
DATA "            c = mix(c, skyGrad(K, rd), smoothstep(0.92, 1.0, rho));"
DATA "        }"
DATA "    }"
DATA "    let cl = fbm(rd * 2.6 + vec3f(T * 0.008, 0.0, T * 0.004));"
DATA "    let band = 1.0 - smoothstep(0.2, 0.75, abs(h + 0.08));"
DATA "    let cloud = smoothstep(0.42, 0.7, cl) * band;"
DATA "    var cc = vec3f(252.0, 210.0, 242.0);"
DATA "    if (K.sky == 1) { cc = vec3f(200.0, 150.0, 200.0); }"
DATA "    if (K.sky == 4) { cc = vec3f(80.0, 72.0, 130.0); }"
DATA "    if (K.sky == 5) { cc = vec3f(255.0, 190.0, 170.0); }"
DATA "    if (K.sky == 6 || K.sky == 7 || K.sky == 8) { cc = vec3f(250.0, 250.0, 255.0); }"
DATA "    if (K.sky == 9) { cc = vec3f(70.0, 40.0, 100.0); }"
DATA "    c = mix(c, cc, cloud * 0.9);"
DATA "    if (h > 0.1) {"
DATA "        let st = hash3(floor(rd * 380.0));"
DATA "        var sl = 0.9965;"
DATA "    }"
DATA "    return c;"
DATA "}"
DATA ""
DATA "fn iri(t: f32) -> vec3f {"
DATA "    return 0.5 + 0.5 * cos(6.2831853 * (vec3f(0.0, 0.33, 0.67) + t));"
DATA "}"
DATA ""
DATA "fn eyeCol(n: vec3f, ctr: vec3f, rd: vec3f) -> vec3f {"
DATA "    let g = normalize(cam - ctr);"
DATA "    let c = dot(n, g);"
DATA "    let pc = cos(0.17 + 0.22 * pup);"
DATA "    let ic = 0.84;"
DATA "    var t1 = vec3f(-g.z, 0.0, g.x);"
DATA "    t1 = t1 / (length(t1) + 0.000001);"
DATA "    let t2 = cross(g, t1);"
DATA "    let a = atan2(dot(n, t2), dot(n, t1));"
DATA "    var ir = vec3f(120.0, 135.0, 155.0);"
DATA "    if (act == 2) { ir = vec3f(110.0, 170.0, 110.0); }"
DATA "    if (act == 3) { ir = vec3f(200.0, 150.0, 60.0); }"
DATA "    if (act >= 4) { ir = vec3f(220.0, 35.0, 40.0); }"
DATA "    var col: vec3f;"
DATA "    if (c > pc) {"
DATA "        col = vec3f(10.0 + 110.0 * pup * hor + 160.0 * dark, 4.0, 8.0);"
DATA "    } else if (c > ic) {"
DATA "        let k = (c - ic) / (pc - ic);"
DATA "        let fib = 0.62 + 0.38 * sin(a * 31.0 + sin(a * 6.0) * 3.0);"
DATA "        let fib2 = 0.8 + 0.2 * sin(a * 71.0 + k * 11.0);"
DATA "        var q = (0.35 + 0.85 * k) * fib * fib2;"
DATA "        if (k < 0.1) { q = q * 0.35; }"
DATA "        col = ir * q + vec3f(160.0, 40.0, 60.0) * pup * k + vec3f(230.0, 20.0, 30.0) * dark;"
DATA "    } else {"
DATA "        let edge = 1.0 - c;"
DATA "        col = vec3f(240.0 - 15.0 * edge, 205.0 - 60.0 * edge, 205.0 - 50.0 * edge);"
DATA "        let vn = vnoise(n * 7.0 + vec3f(3.0));"
DATA "        let v1 = abs(sin(a * 8.0 + c * 13.0 + vn * 6.0));"
DATA "        let v2 = abs(sin(a * 19.0 - c * 25.0 + vn * 9.0));"
DATA "        let vw = 0.05 + 0.09 * edge;"
DATA "        if (v1 < vw || v2 < vw * 0.6) { col = mix(col, vec3f(160.0, 20.0, 30.0), 0.85); }"
DATA "    }"
DATA "    // eyelid: comes down from the top when the eye blinks"
DATA "    if (lid > 0.001 && c > 0.0) {"
DATA "        let up = -dot(n, t2);"
DATA "        if (up > 1.0 - 2.05 * lid) {"
DATA "            col = vec3f(205.0, 140.0, 140.0) * (0.75 + 0.25 * up);"
DATA "            if (abs(up - (1.0 - 2.05 * lid)) < 0.03) { col = vec3f(90.0, 40.0, 45.0); }"
DATA "        }"
DATA "    }"
DATA "    let sun = normalize(vec3f(-0.45, 0.75, 0.5));"
DATA "    col = col * (0.75 + 0.35 * max(dot(n, sun), 0.0));"
DATA "    col = col + vec3f(255.0) * pow(max(dot(reflect(rd, n), sun), 0.0), 120.0);"
DATA "    return col;"
DATA "}"
DATA ""
DATA "struct Surf { col: vec3f, gloss: f32, emis: f32 };"
DATA ""
DATA "fn tile(uv: vec2f, sc: f32, a: vec3f, b: vec3f) -> vec3f {"
DATA "    let f2 = fract(uv * sc);"
DATA "    if (f2.x < 0.05 || f2.y < 0.05) { return b; }"
DATA "    return a * (0.94 + 0.06 * hs(floor(uv.x * sc) * 7.1 + floor(uv.y * sc) * 3.3));"
DATA "}"
DATA ""
DATA "fn surf(mt: i32, p: vec3f, lp: vec3f, n: vec3f, rd: vec3f) -> Surf {"
DATA "    var s = Surf(vec3f(220.0, 200.0, 230.0), 0.0, 0.0);"
DATA "    let ay = abs(n.y);"
DATA "    var uv = lp.xy;"
DATA "    if (ay > 0.6) { uv = lp.xz; } else if (abs(n.x) > abs(n.z)) { uv = lp.zy; }"
DATA "    switch mt {"
DATA "        case 3: { s = Surf(vec3f(255.0, 250.0, 232.0), 0.0, 1.0); }"
DATA "        case 4: {"
DATA "            let r = 18.0 * sin(p.x * 3.0 + T * 1.6) * sin(p.z * 2.6 - T * 1.3);"
DATA "            s = Surf(vec3f(80.0 + r, 185.0 + r, 225.0 + r * 0.5), 0.85, 0.0);"
DATA "        }"
DATA "        case 7: {"
DATA "            // EXIT, readable from either side"
DATA "            let u = lp.x * sign(lp.z + 0.0001);"
DATA "            let col = i32(floor((u + 0.5) * 19.0));"
DATA "            let row = i32(floor((0.17 - lp.y) / 0.34 * 7.0));"
DATA "            var lit = false;"
DATA "            if (row >= 1 && row <= 5 && col >= 2 && col <= 16) {"
DATA "                let lc = col - 2;"
DATA "                let li = lc / 4;"
DATA "                let lcol = lc - li * 4;"
DATA "                if (lcol < 3) {"
DATA "                    var gl = 31143;"
DATA "                    if (li == 1) { gl = 23213; }"
DATA "                    if (li == 2) { gl = 29847; }"
DATA "                    if (li == 3) { gl = 29842; }"
DATA "                    let bp = 14 - ((row - 1) * 3 + lcol);"
DATA "                    lit = ((gl >> u32(bp)) & 1) == 1;"
DATA "                }"
DATA "            }"
DATA "            if (abs(lp.z) < 0.055) { s = Surf(vec3f(200.0, 200.0, 205.0), 0.3, 0.0); }"
DATA "            else if (lit) { s = Surf(vec3f(230.0, 255.0, 230.0), 0.0, 1.0); }"
DATA "            else { s = Surf(vec3f(20.0, 170.0, 75.0), 0.0, 1.0); }"
DATA "        }"
DATA "        case 8: { s = Surf(tile(uv, 1.25, vec3f(246.0, 200.0, 222.0), vec3f(255.0, 240.0, 248.0)), 0.25, 0.0); }"
DATA "        case 9: {"
DATA "            let fr = 1.0 - max(dot(-rd, n), 0.0);"
DATA "            s = Surf(mix(vec3f(235.0, 205.0, 245.0), iri(fr * 1.4 + lp.y * 0.12) * 255.0, 0.55), 0.7, 0.0);"
DATA "        }"
DATA "        case 10: { s = Surf(vec3f(120.0, 195.0, 115.0) * (0.85 + 0.3 * vnoise(p * 2.0)), 0.0, 0.0); }"
DATA "        case 12: { s = Surf(vec3f(175.0, 125.0, 85.0) * (0.85 + 0.15 * sin(uv.x * 30.0 + sin(uv.y * 3.0))), 0.1, 0.0); }"
DATA "        case 13: { s = Surf(vec3f(235.0, 95.0, 120.0), 0.35, 0.0); }"
DATA "        case 14: {"
DATA "            var mm = 0.5 + 0.5 * sin(uv.x * 2.0 + 3.0 * sin(uv.y * 1.3 + uv.x * 0.7));"
DATA "            mm = mm * mm * mm * mm;"
DATA "            s = Surf(mix(vec3f(150.0, 40.0, 55.0), vec3f(235.0, 215.0, 200.0), mm), 0.3, 0.0);"
DATA "        }"
DATA "        case 15: { s = Surf(vec3f(240.0, 236.0, 232.0), 0.15, 0.0); }"
DATA "        case 16: { s = Surf(vec3f(40.0, 78.0, 58.0), 0.05, 0.0); }"
DATA "        case 17: {"
DATA "            let ck = i32(floor(uv.x * 0.8) + floor(uv.y * 0.8));"
DATA "            s = Surf(select(vec3f(95.0, 72.0, 115.0), vec3f(130.0, 100.0, 150.0), (ck & 1) != 0), 0.1, 0.0);"
DATA "        }"
DATA "        case 19: { s = Surf(vec3f(70.0, 60.0, 90.0), 0.4, 0.0); }"
DATA "        case 20: { s = Surf(select(vec3f(255.0, 30.0, 140.0), vec3f(40.0, 230.0, 255.0), lp.x > 0.6), 0.0, 1.0); }"
DATA "        case 21: {"
DATA "            var q = 1.0;"
DATA "            if (sin(uv.x * 9.0) > 0.5) { q = 0.85; }"
DATA "            s = Surf(vec3f(220.0, 205.0, 145.0) * q, 0.05, 0.0);"
DATA "        }"
DATA "        case 22: { s = Surf(vec3f(245.0, 218.0, 232.0) * (0.92 + 0.08 * vnoise(p * 3.0)), 0.15, 0.0); }"
DATA "        case 23: { s = Surf(tile(uv, 2.5, vec3f(120.0, 205.0, 230.0), vec3f(235.0, 250.0, 255.0)), 0.35, 0.0); }"
DATA "        case 24: { s = Surf(vec3f(110.0, 40.0, 55.0) * (0.85 + 0.2 * vnoise(p * 2.5)), 0.2, 0.0); }"
DATA "        case 25: { s = Surf(vec3f(70.0, 70.0, 80.0) * (0.85 + 0.3 * vnoise(p * 3.0)), 0.1, 0.0); }"
DATA "        case 26: {"
DATA "            let st = hs(dot(floor(lp.yz * 55.0), vec2f(12.9, 78.2)) + floor(T * 24.0) * 1.7);"
DATA "            s = Surf(vec3f(st * 210.0, st * 215.0, st * 235.0), 0.0, 1.0);"
DATA "        }"
DATA "        case 27: { s = Surf(vec3f(255.0, 255.0, 255.0), 0.0, 1.0); }"
DATA "        case 28: { s = Surf(vec3f(3.0, 2.0, 5.0), 0.0, 1.0); }"
DATA "        case 29: {"
DATA "            let fr = 1.0 - max(dot(-rd, n), 0.0);"
DATA "            s = Surf(mix(vec3f(200.0, 200.0, 215.0), iri(fr * 1.2 + lp.y * 0.2) * 255.0, 0.35), 0.92, 0.0);"
DATA "        }"
DATA "        case 30: { s = Surf(mix(vec3f(190.0, 150.0, 255.0), vec3f(255.0, 220.0, 255.0), 0.5 + 0.5 * sin(T * 2.0 + lp.y * 3.0)), 0.0, 1.0); }"
DATA "        case 31: { s = Surf(vec3f(185.0, 170.0, 160.0) * (0.8 + 0.3 * vnoise(p * 4.0)), 0.05, 0.0); }"
DATA "        case 32: { s = Surf(tile(uv, 3.0, vec3f(242.0, 244.0, 246.0), vec3f(170.0, 175.0, 185.0)), 0.3, 0.0); }"
DATA "        case 33: { s = Surf(vec3f(200.0, 40.0, 45.0), 0.3, 0.0); }"
DATA "        case 34: {"
DATA "            let ck = i32(floor(uv.x * 0.7) + floor(uv.y * 0.7));"
DATA "            s = Surf(select(vec3f(30.0, 28.0, 34.0), vec3f(240.0, 238.0, 242.0), (ck & 1) != 0), 0.3, 0.0);"
DATA "        }"
DATA "        case 35: { s = Surf(vec3f(60.0, 45.0, 40.0), 0.25, 0.0); }"
DATA "        default: { }"
DATA "    }"
DATA "    return s;"
DATA "}"
DATA ""
DATA "fn softShadow(ro: vec3f, rd: vec3f, maxt: f32) -> f32 {"
DATA "    var res = 1.0;"
DATA "    var t = 0.06;"
DATA "    for (var i = 0; i < 16; i++) {"
DATA "        let h = mapd(ro + rd * t);"
DATA "        res = min(res, 9.0 * h / t);"
DATA "        t = t + clamp(h, 0.12, 1.6);"
DATA "        if (res < 0.02 || t > maxt) { break; }"
DATA "    }"
DATA "    return clamp(res, 0.0, 1.0);"
DATA "}"
DATA ""
DATA "fn ambOcc(p: vec3f, n: vec3f) -> f32 {"
DATA "    var occ = 0.0;"
DATA "    var sca = 1.0;"
DATA "    for (var i = 0; i < 3; i++) {"
DATA "        let h = 0.06 + 0.3 * f32(i);"
DATA "        occ = occ + (h - mapd(p + n * h)) * sca;"
DATA "        sca = sca * 0.65;"
DATA "    }"
DATA "    return clamp(1.0 - 1.6 * occ, 0.0, 1.0);"
DATA "}"
DATA ""
DATA "// ---- ray tracing: primary ray, sun shadow ray, one reflection bounce ----"
DATA "var<private> glowAcc: f32;"
DATA "const FAR = 110.0;"
DATA ""
DATA "// one ray; returns (distance, 1 hit / 0 past FAR / 2 out of steps). The step"
DATA "// grows a little with distance, where fog hides any detail it skips."
DATA "fn march(ro: vec3f, rd: vec3f, maxs: i32, wantGlow: bool) -> vec2f {"
DATA "    rdir = rd;"
DATA "    var t = 0.05;"
DATA "    let stepK = max(0.92 - 0.35 * mlt, 0.5);"
DATA "    for (var i = 0; i < 300; i++) {"
DATA "        if (i >= maxs) { break; }"
DATA "        let d = mapd(ro + rd * t);"
DATA "        if (wantGlow) { glowAcc = glowAcc + min(d * stepK, 0.5) / (1.0 + gEyeD * gEyeD * 4.0); }"
DATA "        if (d < 0.0008 * t + 0.001) { return vec2f(t, 1.0); }"
DATA "        t = t + d * stepK + 0.0015 * t;"
DATA "        if (t > FAR) { return vec2f(t, 0.0); }"
DATA "    }"
DATA "    return vec2f(t, 2.0);"
DATA "}"
DATA ""
DATA "struct Hit { col: vec3f, refl: f32, n: vec3f };"
DATA ""
DATA "// the surface the last march stopped on, lit by the sun (soft shadow ray),"
DATA "// the sky dome and ambient occlusion; refl = share of the reflected ray."
DATA "// Far surfaces, hidden in the haze anyway, skip the shadow ray and AO."
DATA "fn lightHit(p: vec3f, rd: vec3f, t: f32, primary: bool) -> Hit {"
DATA "    let mt = gMat;"
DATA "    let sb = gSub;"
DATA "    let lp = gLoc;"
DATA "    let ctr = gCtr;"
DATA "    let ee = vec2f(1.0, -1.0) * (0.0015 + 0.0006 * t);"
DATA "    let n = normalize(ee.xyy * mapd(p + ee.xyy) + ee.yyx * mapd(p + ee.yyx) + ee.yxy * mapd(p + ee.yxy) + ee.xxx * mapd(p + ee.xxx));"
DATA "    var hall = KIa.sp == 1;"
DATA "    if (sb == 1) { hall = KIb.sp == 1; }"
DATA "    var s = Surf(vec3f(0.0), 0.0, 0.0);"
DATA "    var nn = n;"
DATA "    let isEye = mt == 1 || mt == 2 || mt == 18;"
DATA "    if (isEye) {"
DATA "        if (mt == 1) { nn = normalize(p - eyeP); }"
DATA "        s = Surf(eyeCol(nn, ctr, rd), 0.22, 2.0);"
DATA "    } else if (mt == 5) {"
DATA "        s = Surf(vec3f(150.0, 55.0, 75.0) * (0.8 + 0.3 * vnoise(p * 6.0)), 0.3, 0.0);"
DATA "    } else {"
DATA "        s = surf(mt, p, lp, n, rd);"
DATA "    }"
DATA "    var col = s.col;"
DATA "    var refl = 0.0;"
DATA "    if (s.emis < 0.5 || s.emis > 1.5) {"
DATA "        let sun = normalize(vec3f(-0.45, 0.75, 0.5));"
DATA "        var sh = 1.0;"
DATA "        var ao = 1.0;"
DATA "        useBnd = false;"
DATA "        if (primary && !hall && t < 40.0) { sh = softShadow(p + nn * 0.02, sun, 20.0); }"
DATA "        if (primary && t < 30.0) { ao = ambOcc(p, nn); }"
DATA "        useBnd = true;"
DATA "        if (s.emis < 0.5) {"
DATA "            var dif = max(dot(nn, sun), 0.0);"
DATA "            if (hall) { dif = max(nn.y, 0.0) * 0.4 + 0.3; }"
DATA "            let hemi = mix(vec3f(0.62, 0.5, 0.66), vec3f(1.05, 0.95, 1.1), 0.5 + 0.5 * nn.y);"
DATA "            col = col * (hemi * 0.6 * ao + vec3f(1.0, 0.93, 0.97) * 0.62 * dif * sh);"
DATA "        } else {"
DATA "            col = col * (0.75 + 0.25 * sh) * (0.8 + 0.2 * ao);"
DATA "        }"
DATA "        if (s.gloss > 0.0) {"
DATA "            let r = reflect(rd, nn);"
DATA "            col = col + vec3f(255.0) * pow(max(dot(r, sun), 0.0), 48.0) * s.gloss * sh;"
DATA "            let fres = pow(1.0 - max(dot(-rd, nn), 0.0), 3.0);"
DATA "            refl = clamp(s.gloss * (0.25 + 0.65 * fres), 0.0, 0.9);"
DATA "        }"
DATA "    }"
DATA "    if (!isEye && mt != 27) { col = col * (1.0 - 0.93 * dark); }"
DATA "    return Hit(col, refl, nn);"
DATA "}"
DATA ""
DATA "// the sky in direction rd, hazed toward the fog colour near the horizon"
DATA "fn skyMix(rd: vec3f) -> vec3f {"
DATA "    var s = skyCol(KIa, rd);"
DATA "    if (bl > 0.002) { s = mix(s, skyCol(KIb, rd), bl); }"
DATA "    return mix(s, fogC, 0.75 * exp(-abs(rd.y) * 5.0)) * (1.0 - 0.95 * dark);"
DATA "}"
DATA ""
DATA "// the same without clouds and stars: what far surfaces fade into, so that"
DATA "// they melt into the sky behind them instead of standing out as flat shapes"
DATA "fn hazeCol(rd: vec3f) -> vec3f {"
DATA "    var s = skyGrad(KIa, rd);"
DATA "    if (bl > 0.002) { s = mix(s, skyGrad(KIb, rd), bl); }"
DATA "    return mix(s, fogC, 0.75 * exp(-abs(rd.y) * 5.0)) * (1.0 - 0.95 * dark);"
DATA "}"
DATA ""
DATA "fn render(ro: vec3f, rd: vec3f) -> vec3f {"
DATA "    glowAcc = 0.0;"
DATA "    var col = vec3f(0.0);"
DATA "    var thr = 1.0;"
DATA "    var o = ro;"
DATA "    var d = rd;"
DATA "    let maxs = i32(qual);"
DATA "    for (var b = 0; b < 2; b++) {"
DATA "        let m = march(o, d, select(maxs / 4, maxs, b == 0), b == 0);"
DATA "        if (m.y < 0.5) {"
DATA "            col = col + thr * skyMix(d);"
DATA "            break;"
DATA "        }"
DATA "        if (m.y > 1.5) {"
DATA "            col = col + thr * mix(hazeCol(d), skyMix(d), smoothstep(0.0, FAR, m.x));"
DATA "            break;"
DATA "        }"
DATA "        let p = o + d * m.x;"
DATA "        let h = lightHit(p, d, m.x, b == 0);"
DATA "        let fogk = max(1.0 - exp(-m.x * fogD), smoothstep(FAR * 0.45, FAR, m.x));"
DATA "        let hz = hazeCol(d);"
DATA "        let base = mix(h.col, hz, fogk);"
DATA "        if (b == 1 || h.refl < 0.02 || m.x > 45.0) {"
DATA "            col = col + thr * base;"
DATA "            break;"
DATA "        }"
DATA "        if (h.refl < 0.3) {"
DATA "            // a faint gloss only mirrors the sky: no second ray needed"
DATA "            col = col + thr * mix(base, skyMix(reflect(d, h.n)), h.refl * (1.0 - fogk));"
DATA "            break;"
DATA "        }"
DATA "        // the reflected ray: traced through the scene, not only the sky"
DATA "        col = col + thr * ((1.0 - h.refl) * base + h.refl * fogk * hz);"
DATA "        thr = thr * h.refl * (1.0 - fogk);"
DATA "        o = p + h.n * 0.03;"
DATA "        d = reflect(d, h.n);"
DATA "    }"
DATA "    col = col + glowAcc * (25.0 + 200.0 * venv) * vec3f(1.0, 0.35 + 0.4 * (1.0 - hor), 0.6 * (1.0 - hor) + 0.15);"
DATA "    return col;"
DATA "}"
DATA ""
DATA ""
DATA "fn shade(i: ShadeIn) -> vec4<f32> {"
DATA "    cam = i.params[0].xyz;"
DATA "    T = i.params[0].w;"
DATA "    kA = i32(i.params[1].x + 0.5);"
DATA "    kB = i32(i.params[1].y + 0.5);"
DATA "    bl = i.params[1].z;"
DATA "    mlt = i.params[1].w;"
DATA "    let roll = i.params[2].x;"
DATA "    venv = i.params[2].y;"
DATA "    let ev = i.params[2].z;"
DATA "    hor = clamp(T / 900.0, 0.0, 1.0);"
DATA "    gli = i.params[2].w;"
DATA "    let w = f32(kA);"
DATA "    stat = i.params[3].y;"
DATA "    let aspect = 1.7777778;"
DATA "    qual = i.params[3].w;"
DATA "    act = i32(floor(w / 10.0 + 0.001));"
DATA "    pup = clamp(venv * 1.2 + 0.08 * sin(T * 0.9), 0.0, 1.3);"
DATA "    sat1 = 0.0;"
DATA "    sat2 = 0.0;"
DATA "    if (T >= 630.0 && T < 900.0) { sat1 = clamp((T - 630.0) / 12.0, 0.0, 1.0); }"
DATA "    if (T >= 720.0 && T < 900.0) { sat2 = clamp((T - 720.0) / 12.0, 0.0, 1.0); }"
DATA "    KIa = winfo(kA);"
DATA "    KIb = winfo(kB);"
DATA "    let KA = KIa;"
DATA "    let KB = KIb;"
DATA "    fogC = mix(KA.fog, KB.fog, bl);"
DATA "    fogD = mix(KA.fd, KB.fd, bl) * 0.6;"
DATA "    if (w > 48.5) { fogD = fogD + smoothstep(884.0, 900.0, T) * 0.06; }"
DATA ""
DATA "    // the watcher, same formulas as UpdateWorld in the BASIC code"
DATA "    let ez = T * 1.4;"
DATA "    eyeP = vec3f(pathX(ez) + 0.5 * sin(T * 0.23), pathY(ez) + 0.25 * sin(T * 0.31), ez);"
DATA "    eyeR = 0.82 + 0.03 * f32(act);"
DATA "    gz = normalize(cam - eyeP);"
DATA "    tb1 = normalize(cross(gz, vec3f(0.0, 1.0, 0.0)));"
DATA "    tb2 = cross(gz, tb1);"
DATA ""
DATA "    // events"
DATA "    evT = i32(floor(ev + 0.0001));"
DATA "    evP = fract(ev);"
DATA "    lid = clamp(1.0 - abs(fract(T / 5.3) - 0.5) / 0.03, 0.0, 1.0);"
DATA "    if (evT == 1) { lid = max(lid, max(clamp(1.0 - abs(evP - 0.25) / 0.07, 0.0, 1.0), clamp(1.0 - abs(evP - 0.6) / 0.07, 0.0, 1.0))); }"
DATA "    dark = 0.0;"
DATA "    if (evT == 2) {"
DATA "        dark = smoothstep(0.03, 0.08, evP) * (1.0 - smoothstep(0.8, 0.9, evP));"
DATA "        if (evP < 0.2 && hs(floor(T * 18.0)) > 0.5) { dark = dark * 0.3; }"
DATA "    }"
DATA "    figOn = evT == 4 && evP > 0.08 && evP < 0.92 && (evP < 0.7 || hs(floor(T * 15.0)) > 0.4);"
DATA "    dissolve = smoothstep(860.0, 898.0, T) * 0.9;"
DATA "    var yaw = 0.0;"
DATA "    if (evT == 3) { yaw = 3.14159265 * smoothstep(0.02, 0.18, evP) * (1.0 - smoothstep(0.72, 0.84, evP)); }"
DATA "    // the camera looks at the watcher; pixel ray, CRT curvature, roll, glitch"
DATA "    // the camera turns away during event 3 (same formula as the BASIC code)"
DATA "    let tb = eyeP + vec3f(0.0, 0.3, 0.0) - cam;"
DATA "    let ty = rot2(tb.xz, yaw);"
DATA "    let tgt = cam + vec3f(ty.x, tb.y, ty.y);"
DATA "    let f = normalize(tgt - cam);"
DATA "    let r0 = normalize(vec3f(f.z, 0.0, -f.x));"
DATA "    let u0 = cross(f, r0);"
DATA "    let rd0 = normalize(i.world - cam);"
DATA "    let uv0 = vec2f(dot(rd0, r0), dot(rd0, u0)) / max(dot(rd0, f), 0.05);"
DATA "    let rowpx = uv0.y / max(abs(dpdy(uv0.y)), 0.000001);"
DATA "    let tanH = 0.5773503;"
DATA "    let sn = uv0 / vec2f(tanH * aspect, tanH);"
DATA "    let bar = sn * (1.0 + 0.03 * vec2f(sn.y * sn.y, sn.x * sn.x));"
DATA "    let bq = abs(bar) - vec2f(0.975 - 0.07, 0.96 - 0.07);"
DATA "    let bd = length(max(bq, vec2f(0.0))) + min(max(bq.x, bq.y), 0.0) - 0.07;"
DATA "    if (bd > 0.0) {"
DATA "        let sheen = 14.0 + 10.0 * exp(-bd * 60.0);"
DATA "        return vec4<f32>(vec3f(sheen, sheen * 0.95, sheen * 1.1) / 255.0, 1.0);"
DATA "    }"
DATA "    var uv = bar * vec2f(tanH * aspect, tanH);"
DATA "    uv = rot2(uv, roll);"
DATA "    if (gli > 0.05) {"
DATA "        let band = floor(uv.y * 30.0) + floor(T * 12.0) * 7.0;"
DATA "        if (hs(band * 1.37) < gli * 0.35) { uv.x = uv.x + (hs(band + 5.0) - 0.5) * 0.25 * gli; }"
DATA "    }"
DATA "    let rd = normalize(f + uv.x * r0 + uv.y * u0);"
DATA "    var col = render(cam, rd);"
DATA ""
DATA "    // vignette, CRT glass edge, scanlines, static, grain"
DATA "    col = col * (1.0 - 0.28 * dot(sn * sn, vec2f(0.7, 1.0)));"
DATA "    col = col * smoothstep(0.0, 0.045, -bd);"
DATA "    if (fract(rowpx / 3.0) < 0.34) { col = col * 0.88; }"
DATA "    if (stat > 0.0) {"
DATA "        let h = hs(dot(floor(uv * 420.0), vec2f(12.9898, 78.233)) + floor(T * 30.0) * 3.7);"
DATA "        if (h < stat) { col = vec3f(hs(h * 91.0) * 230.0); }"
DATA "    }"
DATA "    col = col + (hs(dot(uv, vec2f(127.1, 311.7)) * 50.0 + fract(T) * 13.0) - 0.5) * 8.0;"
DATA "    // melt flash at the middle of each transition, red pulse with the voice"
DATA "    // late in the dream, fade in from black, fade out to white"
DATA "    let mflash = sin(bl * 3.14159265);"
DATA "    if (mflash > 0.8) { col = mix(col, vec3f(255.0), (mflash - 0.8) * 0.78); }"
DATA "    if (hor > 0.78) { col = mix(col, vec3f(160.0, 0.0, 20.0), venv * 0.18); }"
DATA "    if (T < 3.0) { col = col * (T / 3.0); }"
DATA "    if (T > 899.0) { col = mix(col, vec3f(255.0), clamp((T - 899.0) / 6.0, 0.0, 1.0)); }"
DATA "    // lunge and snap-back: red flash and a frame of inverted colours"
DATA "    if (evT == 5) {"
DATA "        col = mix(col, vec3f(200.0, 0.0, 20.0), clamp(1.0 - abs(evP - 0.3) / 0.12, 0.0, 1.0) * 0.55);"
DATA "        if (evP > 0.27 && evP < 0.31) { col = vec3f(255.0) - col; }"
DATA "    }"
DATA "    if (evT == 3 && evP > 0.84 && evP < 0.88) { col = vec3f(255.0) - col; }"
DATA "    return vec4<f32>(clamp(col / 255.0, vec3f(0.0), vec3f(1.0)), 1.0);"
DATA "}"
DATA "~"

' ============================================================================
'  setup
' ============================================================================
SUB InitData
    RESTORE Events
    FOR i = 0 TO 49: READ evList(i): NEXT
    RESTORE Chords
    FOR a = 0 TO 4: FOR c = 0 TO 3: FOR n = 0 TO 3: READ chordN(a, c, n): NEXT: NEXT: NEXT
    RESTORE VoiceLines
    FOR k = 0 TO 50
        READ en$, ph$
        enL$(k) = en$: phL$(k) = ph$
        lineAt(k) = k * WLEN + 2.5
    NEXT
    lineAt(50) = 900.6
    subLine = -1: nextLine = 0: glLine = -1
END SUB

SUB LoadFonts
    DIM cand$(12)
    cand$(1) = "consolab.ttf"
    cand$(2) = "consola.ttf"
    cand$(3) = "lucon.ttf"
    cand$(4) = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf"
    cand$(5) = "/usr/share/fonts/dejavu/DejaVuSansMono-Bold.ttf"
    cand$(6) = "/usr/share/fonts/TTF/DejaVuSansMono-Bold.ttf"
    cand$(7) = "/usr/share/fonts/truetype/liberation/LiberationMono-Bold.ttf"
    cand$(8) = "/usr/share/fonts/liberation-mono/LiberationMono-Bold.ttf"
    cand$(9) = "/usr/share/fonts/truetype/ubuntu/UbuntuMono-B.ttf"
    cand$(10) = "/System/Library/Fonts/Supplemental/Courier New Bold.ttf"
    cand$(11) = "/Library/Fonts/Courier New Bold.ttf"
    cand$(12) = "cour.ttf"
    fontOK = 0: fontEn = 16: fontHud = 16: fontBig = 16
    FOR i = 1 TO 12
        f = _LOADFONT(cand$(i), INT(24 * uiK))
        IF f > 0 THEN
            fontEn = f: fontOK = 1
            f4 = _LOADFONT(cand$(i), INT(46 * uiK))
            IF f4 > 0 THEN fontBig = f4 ELSE fontBig = f
            IF uiK > 1.4 THEN
                f3 = _LOADFONT(cand$(i), INT(14 * uiK))
                IF f3 > 0 THEN fontHud = f3
            END IF
            EXIT FOR
        END IF
    NEXT
END SUB

SUB InitAudio
    sr = _SNDRATE
    IF sr <= 0 THEN
        sr = 44100: audioOn = 0
    ELSE
        audioOn = 1
    END IF
    genS = 0
    cF1 = 500: cF2 = 1500: cF3 = 2500: curPit = 1: cNF = 3000: cAmp = 0: cNz = 0
    rmS = 0: rmC = 1
    h1S = 0: h1C = 1: h1K = 2 * PI * 55 / sr
    h2S = 0: h2C = 1: h2K = 2 * PI * 55.7 / sr
    h3S = 0: h3C = 1: h3K = 2 * PI * 82.6 / sr
    echoLen = INT(sr * .23)
    IF echoLen > 32767 THEN echoLen = 32767
    ep = 0
    segN = 0: segIdx = 0: segPos = 0
    FOR i = 0 TO 4095: sinTab(i) = SIN(i * 2 * PI / 4096): NEXT
    FOR i = 0 TO 5: pInc(i) = 0: pPh(i) = i * .17: NEXT
    stingW = -1: lastHB = -1
END SUB

' ============================================================================
'  small math helpers
' ============================================================================
FUNCTION Mx (a, b)
    IF a > b THEN Mx = a ELSE Mx = b
END FUNCTION

FUNCTION Mn (a, b)
    IF a < b THEN Mn = a ELSE Mn = b
END FUNCTION

FUNCTION Cl (v, lo, hi)
    r = v
    IF r < lo THEN r = lo
    IF r > hi THEN r = hi
    Cl = r
END FUNCTION

FUNCTION Sm (e0, e1, v)
    tt = (v - e0) / (e1 - e0)
    IF tt < 0 THEN tt = 0
    IF tt > 1 THEN tt = 1
    Sm = tt * tt * (3 - 2 * tt)
END FUNCTION

' the GPU worlds' path: higher and wider, a clear tube around it
FUNCTION GpuPathX (z)
    GpuPathX = 2.2 * SIN(z * .045) + .9 * SIN(z * .11 + 1.3)
END FUNCTION

FUNCTION GpuPathY (z)
    GpuPathY = 4 + .4 * SIN(z * .07 + .4)
END FUNCTION

' ============================================================================
'  per-frame update: worlds, melting, camera
' ============================================================================
SUB UpdateWorld (el, dt)
    DIM pb AS LONG
    tNow = el
    w = INT(el / WLEN)
    IF w > 49 THEN w = 49
    wCur = w
    act = INT(w / 10)
    horror = Cl(el / 900, 0, 1)
    lt = el - w * WLEN

    kindA = w: kindB = w: blendB = 0
    IF w < 49 AND lt > 14 THEN
        blendB = Sm(14, 18, lt)
        kindB = w + 1
    END IF
    meltT = SIN(blendB * PI)
    IF w = 49 THEN meltT = Mx(meltT, Sm(888, 900, el) * .8)
    fin = 0
    IF el > 900 THEN fin = Cl((el - 900) / 6, 0, 1)
    melt = meltT * (.7 + .3 * horror) + .06 * horror * (.5 + .5 * SIN(el * .37)) + fin * .8

    ' glitch and CRT static grow toward the end
    glitchAmt = 0
    IF el >= 720 THEN glitchAmt = .15 + .6 * (el - 720) / 180
    IF w >= 46 AND w < 49 THEN glitchAmt = glitchAmt + .2
    IF RND < .015 + .04 * horror THEN glitchAmt = glitchAmt + .4 * horror
    glitchAmt = glitchAmt + meltT * .25 * horror
    IF el > 895 THEN glitchAmt = glitchAmt * Cl((900 - el) / 5, 0, 1)
    staticAmt = 0
    IF el >= 540 THEN staticAmt = .004
    IF el >= 720 THEN staticAmt = .012 + .045 * (el - 720) / 180
    IF w = 41 OR w = 48 THEN staticAmt = staticAmt + .015
    IF el > 895 THEN staticAmt = staticAmt * Cl((900 - el) / 5, 0, 1)

    ' what the speaker is saying right now drives the pupil and the glow
    pb = INT(CDBL(el) * sr / BLK)
    en = envRing(pb AND 4095)
    voiceEnv = voiceEnv + (en - voiceEnv) * .5

    ' third person: the camera flies backwards in front of the watcher and
    ' looks at it while it follows (same formulas as the shader). Both stay in
    ' the clear tube around the path, so they never touch the worlds.
    shk = .006 + .03 * horror + .04 * voiceEnv * horror + glitchAmt * .05
    ez = el * 1.4
    tx = GpuPathX(ez) + .5 * SIN(el * .23)
    ty = GpuPathY(ez) + .25 * SIN(el * .31) + .3
    tz = ez
    camZ = ez + 6.2 + SIN(el * .13) - 1.8 * horror
    camX = GpuPathX(camZ) + 1.2 * SIN(el * .21) + .35 * SIN(el * .57) + (RND - .5) * shk
    camY = GpuPathY(camZ) + .6 + .5 * SIN(el * .17) + .15 * SIN(el * .71) + (RND - .5) * shk

    ' scare events (the shader reads the same type and phase)
    EventAt el, evType, evPh
    dz = 0: kick = 0
    IF evType = 3 THEN
        ' turn to look down the path; snapping back, the watcher is right there
        yaw = PI * Sm(.02, .18, evPh) * (1 - Sm(.72, .84, evPh))
        bx = tx - camX: bz = tz - camZ
        cs = COS(yaw): sn = SIN(yaw)
        tx = camX + bx * cs - bz * sn: tz = camZ + bx * sn + bz * cs
        dz = 3.6 * Sm(.82, .86, evPh) * (1 - Sm(.9, 1, evPh))
        kick = .2 * Cl(1 - ABS(evPh - .86) / .06, 0, 1)
    ELSEIF evType = 5 THEN
        ' the watcher lunges at the camera
        dz = 4.2 * Cl(1 - ABS(evPh - .3) / .12, 0, 1)
        kick = .25 * Cl(1 - ABS(evPh - .3) / .15, 0, 1)
    END IF
    dz = Mn(dz, camZ - ez - 2.4)
    IF dz > 0 THEN camZ = camZ - dz
    camX = camX + (RND - .5) * kick: camY = camY + (RND - .5) * kick
    camTX = tx: camTY = ty: camTZ = tz
    camRoll = .1 * SIN(el * .17) + .07 * SIN(el * .43) * horror + meltT * .25 * SIN(el * .9) + (RND - .5) * shk

    ' camera basis without roll, for the overlay quad
    fx = tx - camX: fy = ty - camY: fz = tz - camZ
    l = SQR(fx * fx + fy * fy + fz * fz)
    cfX = fx / l: cfY = fy / l: cfZ = fz / l
    rbX = cfZ: rbZ = -cfX
    l = SQR(rbX * rbX + rbZ * rbZ)
    rbX = rbX / l: rbZ = rbZ / l
    ubX = cfY * rbZ
    ubY = cfZ * rbX - cfX * rbZ
    ubZ = -cfY * rbX
END SUB

' ============================================================================
'  screen effects, HUD, subtitles
' ============================================================================
' ---- overlay: HUD and subtitles ----------------------------------------------
' OverlayPrep works out what the overlay shows. The overlay image is redrawn
' and handed to the GPU again only when that changes (the HUD once a second,
' the subtitle once per letter), so most frames upload no texture at all.
SUB OverlayPrep (el)
    hudA = 230
    IF el > 899 THEN hudA = 230 * (1 - Cl((el - 899) / 3, 0, 1))
    hudA = INT(hudA / 16) * 16
    tc = INT(Mx(el, 0))
    IF tc > 900 THEN tc = 900
    hud1$ = "WORLD: " + Two$(wCur + 1) + "/50 | TIME: " + Two$(tc \ 60) + ":" + Two$(tc MOD 60) + " / 15:00"
    hud2$ = "GPU ACTIVE - " + LTRIM$(STR$(INT(fpsVal + .5))) + " FPS"
    IF showFps THEN hud2$ = hud2$ + " -" + STR$(scrW) + "x" + LTRIM$(STR$(scrH)) + " -" + STR$(INT(gpuQ)) + " STEPS"
    introA = 0
    IF el < 7 THEN introA = INT(255 * Sm(0, 1.5, el) * (1 - Sm(5, 7, el)) / 16) * 16

    subOn = 0: subS$ = ""
    IF subLine >= 0 THEN
        k = subLine
        age = el - lineStart(k)
        tail = age - lineDur(k)
        IF age >= 0 AND tail <= 1.8 THEN
            subOn = 1
            subAl = 255
            IF tail > 1 THEN subAl = 255 * (1 - (tail - 1) / .8)
            subAl = INT(subAl / 16) * 16
            prog = Cl((age - .1) / Mx(.1, lineDur(k) - .5), 0, 1)
            ' glitched letters change at most 8 times a second
            tick = INT(el * 8)
            IF tick <> glTick OR k <> glLine THEN
                glTick = tick: glLine = k
                gl = .003 + horror * horror * .07 + glitchAmt * .12
                IF el > 899 THEN gl = 0
                subGl$ = GlitchEn$(enL$(k), gl)
            END IF
            subS$ = LEFT$(subGl$, INT(LEN(subGl$) * prog + .999))
            subFull$ = enL$(k)
        END IF
    END IF
    evTxt$ = ""
    SELECT CASE evType
        CASE 1: IF evPh > .15 AND evPh < .7 THEN evTxt$ = "DON'T BLINK"
        CASE 2: IF evPh > .3 AND evPh < .75 THEN evTxt$ = "WHO TURNED OFF THE LIGHTS?"
        CASE 3: IF evPh > .22 AND evPh < .7 THEN evTxt$ = "DON'T TURN AROUND"
        CASE 4: IF evPh > .3 AND evPh < .8 THEN evTxt$ = "SOMEONE IS STANDING THERE"
        CASE 5: IF evPh > .26 AND evPh < .5 THEN evTxt$ = "FOUND YOU"
    END SELECT
    IF LEN(evTxt$) > 0 AND (INT(el * 10) MOD 4) = 0 THEN evTxt$ = GlitchEn$(evTxt$, .3)
    fin$ = "0"
    IF el > 899 THEN fin$ = "1"
    ovlKey$ = hud1$ + "|" + hud2$ + "|" + STR$(hudA) + STR$(introA) + "|" + STR$(subOn) + subS$ + "|" + STR$(subAl) + STR$(act) + fin$ + "|" + evTxt$
END SUB

FUNCTION Two$ (n)
    Two$ = RIGHT$("0" + LTRIM$(STR$(INT(n))), 2)
END FUNCTION

FUNCTION GlitchEn$ (s$, gl)
    r$ = s$
    FOR i = 1 TO LEN(r$)
        IF MID$(r$, i, 1) <> " " THEN
            IF RND < gl THEN MID$(r$, i, 1) = MID$("#%&@$*!?/\<>", INT(RND * 12) + 1, 1)
        END IF
    NEXT
    GlitchEn$ = r$
END FUNCTION

' VHS-style status box top left, green monospace subtitle on a black bar at
' the bottom of the CRT frame
SUB DrawOverlay (el)
    _DEST ovl
    CLS , _RGBA32(0, 0, 0, 0)
    _FONT fontHud
    fh = _FONTHEIGHT
    bx = INT(scrW * .045): by = INT(scrH * .045)
    pd = INT(fh * .45)
    wbox = Mx(_PRINTWIDTH(hud1$), _PRINTWIDTH(hud2$)) + pd * 2
    LINE (bx, by)-(bx + wbox, by + fh * 2 + pd * 2), _RGBA32(10, 6, 20, hudA * .62), BF
    COLOR _RGBA32(235, 235, 240, hudA)
    _PRINTSTRING (bx + pd, by + pd), hud1$
    _PRINTSTRING (bx + pd, by + pd + fh), hud2$
    IF introA > 0 THEN
        _FONT fontEn
        t$ = "D R E A M C O R E"
        COLOR _RGBA32(255, 255, 255, introA)
        _PRINTSTRING ((scrW - _PRINTWIDTH(t$)) / 2, scrH / 2 - 44 * uiK), t$
        _FONT fontHud
        t$ = "[ ESC ] EXIT"
        COLOR _RGBA32(225, 225, 225, introA * .8)
        _PRINTSTRING ((scrW - _PRINTWIDTH(t$)) / 2, scrH / 2 + 20 * uiK), t$
    END IF
    ' the last act: the walls of the screen keep saying it
    IF act >= 4 AND el < 899 THEN
        _FONT fontHud
        COLOR _RGBA32(170, 120, 220, 42)
        fh2 = _FONTHEIGHT
        FOR i = 0 TO 13
            yy = INT(scrH * .17 + i * fh2 * 1.25)
            _PRINTSTRING (INT(scrW * .055), yy), "YOU ARE BEING WATCHED"
            _PRINTSTRING (INT(scrW * .945) - _PRINTWIDTH("YOU ARE BEING WATCHED"), yy), "YOU ARE BEING WATCHED"
        NEXT
    END IF
    IF LEN(evTxt$) > 0 THEN
        _FONT fontBig
        ex = INT((scrW - _PRINTWIDTH(evTxt$)) / 2): ey = INT(scrH * .26)
        o = INT(Mx(2, 3 * uiK))
        COLOR _RGBA32(0, 230, 255, 150): _PRINTSTRING (ex - o, ey), evTxt$
        COLOR _RGBA32(0, 0, 0, 220): _PRINTSTRING (ex + o, ey + o), evTxt$
        COLOR _RGBA32(255, 35, 50, 255): _PRINTSTRING (ex, ey), evTxt$
    END IF
    IF subOn THEN
        _FONT fontEn
        fh = _FONTHEIGHT
        so = INT(Mx(1, 1.6 * uiK))
        IF el > 899 THEN
            tr = 40: tg = 60: tb = 45
        ELSE
            tr = 110: tg = 255: tb = 120
            IF act >= 4 THEN tr = 255: tg = 95: tb = 95
            LINE (scrW * .04, scrH * .955 - fh * 1.7)-(scrW * .96, scrH * .955), _RGBA32(0, 0, 0, subAl * .75), BF
        END IF
        x0 = INT((scrW - _PRINTWIDTH(subFull$)) / 2)
        y0 = INT(scrH * .955 - fh * 1.35)
        IF LEN(subS$) > 0 THEN
            IF horror > .3 AND el < 899 THEN
                COLOR _RGBA32(255, 0, 60, subAl * .45): _PRINTSTRING (x0 + so, y0), subS$
                COLOR _RGBA32(0, 230, 255, subAl * .35): _PRINTSTRING (x0 - so, y0), subS$
            END IF
            IF el < 899 THEN COLOR _RGBA32(0, 0, 0, subAl): _PRINTSTRING (x0 + so, y0 + so), subS$
            COLOR _RGBA32(tr, tg, tb, subAl): _PRINTSTRING (x0, y0), subS$
        END IF
    END IF
    _DEST scr
END SUB

' ============================================================================
'  voice: text -> segments -> formant synthesis -> _SNDRAW

SUB AddSeg (du, f1, f2, f3, am, nz, nf, p0, p1, iv)
    IF segN > SEGMAX THEN EXIT SUB
    sgDur(segN) = du: sgF1(segN) = f1: sgF2(segN) = f2: sgF3(segN) = f3
    sgAmp(segN) = am: sgNz(segN) = nz: sgNF(segN) = nf
    sgP0(segN) = p0: sgP1(segN) = p1: sgV(segN) = iv
    segTot = segTot + du
    segN = segN + 1
END SUB

SUB Pause (du)
    AddSeg du, 500, 1500, 2500, 0, 0, 3000, 1, 1, 0
END SUB

' a vowel: formants, length, pitch at its start (falls slightly through it)
SUB Vow (f1, f2, f3, du, p)
    AddSeg du, f1, f2, f3, 1, 0, 3000, p, p * .96, 1
END SUB

SUB Nasal (f2, du, p)
    AddSeg du, 260, f2, 2500, .5, 0, 3000, p, p, 0
END SUB

SUB Glide (f1, f2, f3, du, p)
    AddSeg du, f1, f2, f3, .6, 0, 3000, p, p, 0
END SUB

' fricative: noise centre, loudness, length, voiced share
SUB Fric (nf, am, du, voi, p)
    AddSeg du, 350, 1500, 2500, am, 1 - voi, nf, p, p, 0
END SUB

' stop: closure (a low hum when voiced), burst, breath when unvoiced
SUB Plos (nf, am, voiced, p)
    IF voiced THEN
        AddSeg .05, 250, 900, 2300, .22, 0, 3000, p, p, 0
        AddSeg .022, 400, 1500, 2500, am, .7, nf, p, p, 0
    ELSE
        AddSeg .06, 400, 1500, 2500, 0, 1, nf, p, p, 0
        AddSeg .022, 400, 1500, 2500, am, 1, nf, p, p, 0
        AddSeg .035, 400, 1500, 2500, am * .45, 1, 1600, p, p, 0
    END IF
END SUB

' one ARPAbet phoneme; st = 1 for the stressed vowel of a word
SUB Phone (ph$, st, p)
    rt = 1.2 * (1 + .35 * st)
    SELECT CASE ph$
        CASE "IY": Vow 280, 2250, 2900, .14 * rt, p
        CASE "IH": Vow 400, 1920, 2550, .09 * rt, p
        CASE "EH": Vow 550, 1770, 2490, .1 * rt, p
        CASE "AE": Vow 690, 1660, 2490, .13 * rt, p
        CASE "AA": Vow 730, 1100, 2440, .14 * rt, p
        CASE "AO": Vow 590, 880, 2540, .14 * rt, p
        CASE "UH": Vow 450, 1030, 2380, .09 * rt, p
        CASE "UW": Vow 310, 870, 2250, .14 * rt, p
        CASE "AH": Vow 620, 1220, 2550, .09 * rt, p
        CASE "ER": Vow 480, 1350, 1700, .13 * rt, p
        CASE "EY": Vow 550, 1770, 2490, .09 * rt, p: Vow 300, 2200, 2850, .07 * rt, p * .97
        CASE "AY": Vow 730, 1150, 2440, .1 * rt, p: Vow 320, 2150, 2850, .07 * rt, p * .97
        CASE "OW": Vow 570, 900, 2440, .1 * rt, p: Vow 330, 850, 2250, .07 * rt, p * .97
        CASE "AW": Vow 730, 1150, 2440, .1 * rt, p: Vow 330, 850, 2250, .07 * rt, p * .97
        CASE "OY": Vow 590, 880, 2540, .1 * rt, p: Vow 320, 2150, 2850, .07 * rt, p * .97
        CASE "P": Plos 900, .45, 0, p
        CASE "B": Plos 900, .4, 1, p
        CASE "T": Plos 4300, .5, 0, p
        CASE "D": Plos 3800, .42, 1, p
        CASE "K": Plos 2200, .5, 0, p
        CASE "G": Plos 2000, .42, 1, p
        CASE "F": Fric 3800, .25, .1, 0, p
        CASE "V": Fric 3600, .25, .08, .55, p
        CASE "TH": Fric 5200, .16, .09, 0, p
        CASE "DH": Fric 4500, .2, .06, .6, p
        CASE "S": Fric 5800, .42, .11, 0, p
        CASE "Z": Fric 5400, .35, .09, .5, p
        CASE "SH": Fric 2800, .45, .12, 0, p
        CASE "ZH": Fric 2800, .35, .09, .5, p
        CASE "HH": Fric 1600, .2, .06, 0, p
        CASE "CH": Plos 3000, .45, 0, p: Fric 2900, .42, .07, 0, p
        CASE "JH": Plos 2800, .4, 1, p: Fric 2800, .35, .06, .5, p
        CASE "M": Nasal 1100, .08 * rt, p
        CASE "N": Nasal 1600, .07 * rt, p
        CASE "NG": Nasal 2100, .08 * rt, p
        CASE "L": Glide 360, 1300, 2700, .07 * rt, p
        CASE "R": Glide 420, 1250, 1600, .07 * rt, p
        CASE "W": Glide 300, 700, 2200, .065 * rt, p
        CASE "Y": Glide 280, 2200, 2900, .06 * rt, p
        CASE "/": Pause .07
        CASE ",": Pause .25
        CASE ".", "?", "!": Pause .4
    END SELECT
END SUB

' a line in ARPAbet: phonemes separated by spaces, a 1 after the stressed
' vowel, / between words; pitch falls through the line (rises for a question)
SUB AddPhones (s$)
    DIM i AS LONG, j AS LONG, n AS LONG
    n = LEN(s$)
    q = 0
    IF INSTR(s$, "?") > 0 THEN q = 1
    i = 1
    DO WHILE i <= n
        DO WHILE i <= n
            IF MID$(s$, i, 1) <> " " THEN EXIT DO
            i = i + 1
        LOOP
        IF i > n THEN EXIT DO
        j = i
        DO WHILE j <= n
            IF MID$(s$, j, 1) = " " THEN EXIT DO
            j = j + 1
        LOOP
        tk$ = MID$(s$, i, j - i)
        i = j
        fr = i / n
        st = 0
        IF RIGHT$(tk$, 1) = "1" THEN st = 1: tk$ = LEFT$(tk$, LEN(tk$) - 1)
        p = 1.06 - .16 * fr
        IF q = 1 THEN p = p + .4 * fr * fr * fr
        IF st = 1 THEN p = p * 1.12
        Phone tk$, st, p
    LOOP
END SUB

SUB SpeakLine (k)
    segN = 0: segIdx = 0: segPos = 0: segTot = 0
    Pause .12
    AddPhones phL$(k)
    Pause .3
    subLine = k
    lineStart(k) = genS / sr
    lineDur(k) = segTot
END SUB

' the scare event running at second t: type (0 none, 1 blinks, 2 blackout,
' 3 the camera turns away, 4 a figure on the path, 5 lunge) and phase 0..1;
' each world has one, from 8.5 s to 12.5 s of its 18
SUB EventAt (t, ty, ph)
    ty = 0: ph = 0
    IF t < 0 OR t >= 900 THEN EXIT SUB
    w = INT(t / WLEN)
    lt = t - w * WLEN
    IF lt >= 8.5 AND lt < 12.5 THEN
        ty = evList(w)
        ph = (lt - 8.5) / 4
    END IF
END SUB

FUNCTION NoteInc (n)
    NoteInc = 440 * 2 ^ ((n - 69) / 12) / sr
END FUNCTION

SUB GenBlock
    DIM i AS LONG, j AS LONG, hb AS LONG, bi AS LONG
    bt = genS / sr
    IF nextLine <= 50 THEN
        IF bt >= lineAt(nextLine) THEN
            SpeakLine nextLine
            nextLine = nextLine + 1
        END IF
    END IF

    ' ---- voice control rate: glide toward the current segment ----
    tF1 = cF1: tF2 = cF2: tF3 = cF3: tAmp = 0: tNz = cNz: tP = curPit: tNF = cNF
    IF segIdx < segN THEN
        i = segIdx
        tF1 = sgF1(i): tF2 = sgF2(i): tF3 = sgF3(i)
        tAmp = sgAmp(i): tNz = sgNz(i): tNF = sgNF(i)
        sl = sgDur(i) * sr
        fr = segPos / sl
        tP = sgP0(i) + (sgP1(i) - sgP0(i)) * fr
        segPos = segPos + BLK
        IF segPos >= sl THEN segIdx = segIdx + 1: segPos = 0
    END IF
    ' formants glide over about 20 ms, so sounds run into each other like speech
    cF1 = cF1 + (tF1 - cF1) * .3
    cF2 = cF2 + (tF2 - cF2) * .3
    cF3 = cF3 + (tF3 - cF3) * .3
    cAmp = cAmp + (tAmp - cAmp) * .4
    cNz = cNz + (tNz - cNz) * .6
    curPit = curPit + (tP - curPit) * .4
    cNF = tNF

    ' robotic pitch: snapped to semitones, cracking an octave down late in the dream
    f0 = (110 - 26 * horror) * curPit
    st = INT(12 * LOG(f0 / 55) / LOG(2) + .5)
    f0 = 55 * 2 ^ (st / 12)
    IF horror > .6 THEN
        IF RND < .03 * horror THEN f0 = f0 * .5
    END IF
    pinc = f0 / sr

    ' formant resonators (two-pole), unity gain at the centre
    rr = EXP(-PI * 80 / sr): th = 2 * PI * cF1 / sr
    b11 = 2 * rr * COS(th): b12 = -rr * rr: a1 = (1 - rr) * 2 * SIN(th)
    rr = EXP(-PI * 110 / sr): th = 2 * PI * cF2 / sr
    b21 = 2 * rr * COS(th): b22 = -rr * rr: a2 = (1 - rr) * 2 * SIN(th)
    rr = EXP(-PI * 170 / sr): th = 2 * PI * cF3 / sr
    b31 = 2 * rr * COS(th): b32 = -rr * rr: a3 = (1 - rr) * 2 * SIN(th)
    nfc = cNF
    IF nfc > sr * .45 THEN nfc = sr * .45
    rr = EXP(-PI * 1800 / sr): th = 2 * PI * nfc / sr
    bn1 = 2 * rr * COS(th): bn2 = -rr * rr: an = (1 - rr) * 2 * SIN(th)

    ' the voice stays clear early on and grows whispery and metallic later
    wh = .04 + .35 * horror
    vAmp = cAmp * (1 - cNz) * (1 - wh * .5)
    wAmp = cAmp * (1 - cNz) * wh * .6
    nAmp = cAmp * cNz
    rmDepth = .06 + .4 * horror
    rmK = 2 * PI * (30 + 55 * horror) / sr
    drA = .02 + .04 * horror
    stA = glitchAmt * .1
    crush = 0
    IF glitchAmt > .35 THEN crush = 6 + 18 * (1 - glitchAmt)
    envA = 0

    ' ---- events, heard at the moment they are seen ----
    EventAt bt, ety, eph
    mute = 1
    IF ety = 2 THEN mute = 1 - .92 * Sm(.03, .08, eph) * (1 - Sm(.8, .9, eph))
    IF (ety = 5 AND eph >= .27) OR (ety = 3 AND eph >= .84) THEN
        IF stingW <> INT(bt / WLEN) THEN stingW = INT(bt / WLEN): stE = 1: stF = 900
    END IF
    figV = 0
    IF ety = 4 AND eph > .08 AND eph < .92 THEN figV = .035
    rumV = 0
    IF ety = 2 THEN rumV = .25 * (1 - mute)

    ' ---- music: slow chords, a music box, a heartbeat in the last act ----
    am = INT(bt / 180)
    IF am > 4 THEN am = 4
    beat = bt * 66 / 60
    bi = INT(beat)
    chd = (bi \ 4) MOD 4
    IF bt > 897 THEN am = 0: chd = 0
    wob = 1 + .003 * SIN(bt * 1.7) + .002 * SIN(bt * 4.3)
    IF am = 3 THEN wob = wob + .012 * SIN(bt * .9)
    FOR j = 0 TO 3
        pInc(j) = pInc(j) + (NoteInc(chordN(am, chd, j) - 12) - pInc(j)) * .06
    NEXT
    pInc(4) = pInc(0) * 1.0045: pInc(5) = pInc(2) * .996
    hb = INT(beat * 2)
    IF hb <> lastHB THEN
        lastHB = hb
        IF am <= 3 AND bt < 897 THEN
            IF RND < .62 - .1 * am THEN
                nt = chordN(am, chd, INT(RND * 4)) + 24
                IF RND < .3 THEN nt = nt + 12
                mbI = NoteInc(nt)
                IF am = 3 THEN mbI = mbI * (1 + (RND - .5) * .05)
                mbE = .9: mbP = 0
            END IF
        END IF
        IF am = 4 AND bt < 897 THEN
            IF (hb AND 1) = 0 THEN
                hbE = 1: hbI = 70 / sr
            ELSE
                hbE = .55: hbI = 58 / sr
            END IF
        END IF
    END IF
    padV = .05
    IF am = 4 THEN padV = .035
    musG = (1 - .5 * Cl(cAmp, 0, 1)) * mute
    IF bt > 900 THEN musG = musG * Cl((909 - bt) / 9, 0, 1)

    FOR j = 1 TO BLK
        ph = ph + pinc
        IF ph >= 1 THEN ph = ph - 1
        src = ph + ph - 1
        nz = RND + RND - 1
        ex = src * vAmp + nz * wAmp
        o1 = a1 * ex + b11 * y11 + b12 * y12: y12 = y11: y11 = o1
        o2 = a2 * ex + b21 * y21 + b22 * y22: y22 = y21: y21 = o2
        o3 = a3 * ex + b31 * y31 + b32 * y32: y32 = y31: y31 = o3
        o4 = an * nz * nAmp + bn1 * yn1 + bn2 * yn2: yn2 = yn1: yn1 = o4
        v = (o1 + .7 * o2 + .35 * o3) * 2.6 + o4 * 1.6
        rmS = rmS + rmK * rmC: rmC = rmC - rmK * rmS
        v = v * (1 - rmDepth + rmDepth * rmS)
        envA = envA + ABS(v)
        e = echoBuf(ep): echoBuf(ep) = v + e * .32
        ep = ep + 1
        IF ep >= echoLen THEN ep = 0
        h1S = h1S + h1K * h1C: h1C = h1C - h1K * h1S
        h2S = h2S + h2K * h2C: h2C = h2C - h2K * h2S
        h3S = h3S + h3K * h3C: h3C = h3C - h3K * h3S
        dr = drA * (h1S * .5 + h2S * .3 + h3S * .4) * mute + stA * (RND - .5)

        ' music
        pad = 0
        FOR i = 0 TO 5
            pPh(i) = pPh(i) + pInc(i) * wob
            IF pPh(i) >= 1 THEN pPh(i) = pPh(i) - 1
            pad = pad + sinTab(INT(pPh(i) * 4096) AND 4095)
        NEXT
        mbP = mbP + mbI
        IF mbP >= 1 THEN mbP = mbP - 1
        mb = (sinTab(INT(mbP * 4096) AND 4095) + .25 * sinTab(INT(mbP * 8192) AND 4095)) * mbE
        mbE = mbE * .99993
        hbP = hbP + hbI
        IF hbP >= 1 THEN hbP = hbP - 1
        hs1 = sinTab(INT(hbP * 4096) AND 4095) * hbE
        hbE = hbE * .99965: hbI = hbI * .99997
        mus = (pad * padV + mb * .09 + hs1 * .5) * musG

        ' stinger, the figure's whine, the blackout rumble
        sg = 0
        IF stE > .001 THEN
            stP = stP + stF / sr
            IF stP >= 1 THEN stP = stP - 1
            stF = stF * .99993
            sg = (sinTab(INT(stP * 4096) AND 4095) * .6 + (RND - .5) * 1.1) * stE * .8
            stE = stE * .99986
        END IF
        IF figV > 0 THEN
            fgP = fgP + 1830 / sr * (1 + .01 * SIN(bt * 9))
            IF fgP >= 1 THEN fgP = fgP - 1
            sg = sg + sinTab(INT(fgP * 4096) AND 4095) * figV
        END IF
        IF rumV > 0 THEN
            rmb = rmb + ((RND - .5) - rmb) * .01
            sg = sg + rmb * rumV * 6
        END IF

        lo = v + e * .25 + dr + mus + sg: ro = v * .85 + e * .4 + dr + mus + sg
        IF crush > 0 THEN lo = INT(lo * crush) / crush: ro = INT(ro * crush) / crush
        lo = lo / (1 + ABS(lo)): ro = ro / (1 + ABS(ro))
        IF audioOn THEN _SNDRAW lo * .9, ro * .9
    NEXT
    ev = envA / BLK * 4
    IF ev > 1 THEN ev = 1
    envRing((genS \ BLK) AND 4095) = ev
    genS = genS + BLK
END SUB

SUB AudioPump (el)
    DIM tg AS LONG, need AS LONG, mx2 AS LONG
    tg = CLNG((CDBL(el) + LEAD) * sr)
    need = tg - genS
    mx2 = sr \ 2
    IF need > mx2 THEN genS = tg - mx2: need = mx2
    DO WHILE need > 0
        GenBlock
        need = need - BLK
    LOOP
END SUB

' ============================================================================
'  GPU: compile the shader and set up the cube it is drawn on
' ============================================================================
FUNCTION InitGPU
    InitGPU = 0
    src$ = ""
    RESTORE GpuShader
    DO
        READ l$
        IF l$ = "~" THEN EXIT DO
        src$ = src$ + l$ + CHR$(10)
    LOOP
    tracer = _SHADER(src$)
    IF tracer = -1 THEN
        IF INSTR(UCASE$(COMMAND$), "/SHADERLOG") > 0 THEN
            OPEN "dreamcore_shader.log" FOR OUTPUT AS #1
            PRINT #1, _SHADERERROR$
            CLOSE #1
        END IF
        EXIT FUNCTION
    END IF
    dome = _MESHCUBE(1)
    _MESHSHADER dome, tracer
    _CAMERARANGE .1, 100
    ' the overlay quad shows its texture as it is, alpha included
    quadSh = _SHADER("fn shade(i: ShadeIn) -> vec4<f32> { return i.base; }")
    IF quadSh = -1 THEN EXIT FUNCTION
    ovl = _NEWIMAGE(scrW, scrH, 32)
    _DEST ovl
    _PRINTMODE _KEEPBACKGROUND
    _DEST scr
    quadM = 0
    gpuQ = 120
    InitGPU = 1
END FUNCTION

' A quad exactly covering the camera's view half a unit in front of it,
' textured with the overlay image. It is built once; each frame _DRAW3D turns
' it to face the camera, and the texture is sent again only after a redraw.
SUB OverlayQuad
    dd = .5
    hh = dd * .5773503
    hw = hh * scrW / scrH
    IF quadM = 0 THEN
        _MESHBEGIN
        _MESHVERTEX -hw, hh, 0, 0, 0
        _MESHVERTEX hw, hh, 0, 1, 0
        _MESHVERTEX hw, -hh, 0, 1, 1
        _MESHVERTEX -hw, -hh, 0, 0, 1
        _MESHTRIANGLE 0, 1, 2
        _MESHTRIANGLE 0, 2, 3
        _MESHTRIANGLE 0, 2, 1
        _MESHTRIANGLE 0, 3, 2
        quadM = _MESHEND
        _MESHSHADER quadM, quadSh
        ovlDirty = 1
    END IF
    IF ovlDirty THEN
        _MESHTEXTURE quadM, ovl
        ovlDirty = 0
    END IF
    ' local +z toward the camera, local +x to the screen's right
    pitch = ATN(cfY / SQR(Mx(1 - cfY * cfY, .000001)))
    yaw = Atan2(-cfX, -cfZ)
    ' text shake: the whole overlay moves a few pixels
    jit = (horror * 2 + voiceEnv * 2 * horror + glitchAmt * 4) * uiK * (2 * hh / scrH)
    sx = (RND - .5) * jit: sy = (RND - .5) * jit
    qx = camX + cfX * dd - rbX * sx + ubX * sy
    qy = camY + cfY * dd + ubY * sy
    qz = camZ + cfZ * dd - rbZ * sx + ubZ * sy
    _DRAW3D quadM, qx, qy, qz, pitch * 57.29578, yaw * 57.29578, 0
END SUB

FUNCTION Atan2 (y, x)
    IF ABS(x) < .000001 THEN
        IF y >= 0 THEN Atan2 = PI / 2 ELSE Atan2 = -PI / 2
        EXIT FUNCTION
    END IF
    a = ATN(y / x)
    IF x < 0 THEN
        IF y >= 0 THEN a = a + PI ELSE a = a - PI
    END IF
    Atan2 = a
END FUNCTION
