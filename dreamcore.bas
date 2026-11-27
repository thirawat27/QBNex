' ============================================================================
'  DREAMCORE v2  -  a 15-minute walk through 50 dreams
'  for the QBNex compiler (QBasic / QB64-compatible syntax, one file, no assets)
'
'  - 1920 x 1080, all 3D on the GPU: a WGSL shader (_SHADER) raymarches signed
'    distance fields for every pixel and ray traces each one: the primary
'    ray, a soft shadow ray toward the sun, ambient occlusion and a
'    reflection bounce through the scene (water, chrome, the wet eyes)
'  - 50 scenes in 11 composed places (grassy hills, a playground, poolrooms,
'    an endless hallway, a childhood bedroom, an empty mall, a walkway over
'    the clouds, a classroom, a street at night, mirror water, the void),
'    each with its own light, sky and camera shot
'  - no cuts between worlds: every scene ends in a doorway, and through it
'    the next world is already there; the camera walks through
'  - "The Sentient Watcher": a living eye that waits, peeks, follows and
'    watches from the sky, turning its iris to the camera; from minute 10.5
'    rings of smaller eyes circle it
'  - a music box lullaby on worn tape with long reverb, warm pads, and the
'    sound of each place (fluorescent hum, water and drips, wind, crickets,
'    a ticking clock); a soft robotic voice speaks, with subtitles
'  - scare events: blinks, blackouts, a look back, a figure, a lunge
'  - ESC quits at any time; after 900 seconds the screen fades to white,
'    the voice says goodbye and the program ends
'
'  Build:  qb dreamcore.bas     Run:  dreamcore
'  Options: /W window, /T=seconds start later, /FPS details in the HUD,
'           /SHADERLOG write shader errors to dreamcore_shader.log
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
DIM SHARED camX, camY, camZ, camYaw, camPitch, camRoll
DIM SHARED cfX, cfY, cfZ, rbX, rbZ, ubX, ubY, ubZ
DIM SHARED ovl AS LONG, quadSh AS LONG, quadM AS LONG
DIM SHARED fontEn AS LONG, fontHud AS LONG, fontOK AS LONG, fontBig AS LONG
DIM SHARED showFps AS LONG, fpsVal
DIM SHARED hud1$, hud2$, hudA, introA, ovlKey$, ovlDirty AS LONG
DIM SHARED subOn, subS$, subFull$, subAl, subGl$, glTick, glLine

' ---- scenes ----------------------------------------------------------------------
' each scene: place, camera shot, scare event
DIM SHARED scEnv(49), scCam(49), evList(49)
DIM SHARED tNow, horror, act, wCur, wNext
DIM SHARED glitchAmt, staticAmt, voiceEnv, evType, evPh, evTxt$

' ---- voice ----------------------------------------------------------------------
DIM SHARED sr AS LONG, audioOn AS LONG, genS AS LONG
DIM SHARED segN AS LONG, segIdx AS LONG, segPos AS LONG, segTot
DIM SHARED sgDur(SEGMAX), sgF1(SEGMAX), sgF2(SEGMAX), sgF3(SEGMAX)
DIM SHARED sgAmp(SEGMAX), sgNz(SEGMAX), sgNF(SEGMAX), sgP0(SEGMAX), sgP1(SEGMAX)
DIM SHARED sgV(SEGMAX) AS LONG
DIM SHARED cF1, cF2, cF3, cAmp, cNz, cNF, curPit
DIM SHARED ph, y11, y12, y21, y22, y31, y32, yn1, yn2
DIM SHARED a1, b11, b12, a2, b21, b22, a3, b31, b32, an, bn1, bn2
DIM SHARED rmS, rmC
DIM SHARED envRing(4095)
DIM SHARED nextLine, subLine
DIM SHARED enL$(50), phL$(50), lineAt(50), lineStart(50), lineDur(50)

' ---- music and sound ---------------------------------------------------------------
DIM SHARED sinTab(4095), melody(63), chordN(7, 3)
DIM SHARED pPh(7), pInc(7), pLp(7), lastStep AS LONG
DIM SHARED mbP, mbP2, mbI, mbE, bsP, bsI, hbP, hbI, hbE
DIM SHARED stE, stF, stP, stingW, fgP, rmb, wowP, mLpL, mLpR
DIM SHARED rvBuf(24575), rvOff(11) AS LONG, rvLen(11) AS LONG, rvIdx(11) AS LONG, rvLp(7)
DIM SHARED gWind, gWater, gHum, gTick, gCrick, gDrone, gHiss, mCut
DIM SHARED wLp, wLp2, hum1, drpE, drpP, drpI, tickE, crP, drP1, drP2, crk, musPos

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

    UpdateWorld el
    AudioPump el
    ' a cube around the camera: every pixel of the screen runs the shader once
    _CAMERA (camX, camY, camZ) TO (camX + cfX * 5, camY + cfY * 5, camZ + cfZ * 5), 60
    _SHADERPARAM tracer, 0, camX: _SHADERPARAM tracer, 1, camY
    _SHADERPARAM tracer, 2, camZ: _SHADERPARAM tracer, 3, el
    _SHADERPARAM tracer, 4, wCur: _SHADERPARAM tracer, 5, wNext
    _SHADERPARAM tracer, 8, camRoll: _SHADERPARAM tracer, 9, voiceEnv
    _SHADERPARAM tracer, 10, evType + evPh * .999: _SHADERPARAM tracer, 11, glitchAmt
    _SHADERPARAM tracer, 12, camYaw: _SHADERPARAM tracer, 13, camPitch
    _SHADERPARAM tracer, 14, staticAmt: _SHADERPARAM tracer, 15, gpuQ
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
' the 50 scenes: place (0 hills, 1 playground, 2 poolrooms, 3 hallway,
' 4 bedroom, 5 mall, 6 walkway over the clouds, 7 classroom, 8 street,
' 9 mirror water, 10 void), camera shot (0 walk, 1 glide, 2 curve around,
' 3 crane down, 4 creep, 5 stand and stare), scare event (0 none, 1 blinks,
' 2 blackout, 3 look back, 4 a figure, 5 lunge). The places, skies and
' doorways are in the shader's sceneBase table, in the same order.
Scenes:
DATA 0,2,0, 1,0,1, 2,1,0, 4,4,4, 0,3,1, 5,3,2, 1,2,0, 2,0,3, 7,4,1, 0,5,4
DATA 3,0,2, 6,0,1, 2,1,3, 5,2,4, 3,4,5, 6,0,1, 8,0,4, 7,0,2, 3,4,3, 6,5,1
DATA 4,4,2, 3,4,5, 9,2,1, 8,3,4, 2,0,5, 0,5,1, 5,0,3, 7,4,2, 6,0,4, 1,4,5
DATA 9,1,1, 9,3,2, 3,0,5, 4,5,4, 10,1,3, 8,0,2, 9,2,3, 2,4,5, 6,0,4, 9,2,2
DATA 9,0,5, 10,1,2, 3,4,5, 0,3,4, 9,1,2, 4,4,5, 10,1,4, 9,0,3, 10,1,0, 10,1,0

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

' the lullaby: 8 bars of eighth notes (MIDI note, 0 = rest)
Melody:
DATA 76,0,79,0,84,0,83,81, 79,0,0,0,76,0,0,0
DATA 77,0,81,0,86,0,84,83, 84,0,0,0,79,0,0,0
DATA 81,0,79,0,76,0,74,0, 72,0,76,0,79,0,0,0
DATA 77,0,76,0,74,0,67,0, 72,0,0,0,0,0,0,0
' one chord per bar: C, Em, Dm, C, Am, C, G7, C
Harmony:
DATA 48,52,55,59, 52,55,59,62, 50,53,57,60, 48,52,55,60
DATA 45,48,52,55, 48,52,55,59, 43,47,50,53, 48,52,55,59

GpuShader:
' the GPU raymarcher in WGSL, one source line per DATA string ('~' ends it)
DATA "// DREAMCORE v2 raymarcher. Each of the 50 scenes is a composed place in its"
DATA "// own coordinates: floor at y = 0, the walk runs along +z from the entry"
DATA "// doorway at z = 0 to the exit doorway at z = L. Looking through the exit,"
DATA "// a ray that passes the doorway carries on in the next scene (portal), so the"
DATA "// camera walks from one world into the next without a cut."
DATA "// params: 0-2 camera, 3 time, 4 scene, 5 next scene, 6 (unused), 7 (unused),"
DATA "// 8 roll, 9 voice level, 10 event (type + phase), 11 glitch, 12 yaw,"
DATA "// 13 pitch, 14 static, 15 raymarch steps"
DATA "var<private> T: f32;"
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
DATA "var<private> eyeOn: bool;"
DATA "var<private> eyeHere: bool;"
DATA "var<private> skyEye: bool;"
DATA "var<private> gz: vec3f;"
DATA "var<private> tb1: vec3f;"
DATA "var<private> tb2: vec3f;"
DATA "var<private> sat1: f32;"
DATA "var<private> sat2: f32;"
DATA "var<private> fogC: vec3f;"
DATA "var<private> fogD: f32;"
DATA "var<private> gMat: i32;"
DATA "var<private> gEyeD: f32;"
DATA "var<private> gLoc: vec3f;"
DATA "var<private> gCtr: vec3f;"
DATA "var<private> oMat: i32;"
DATA "var<private> oLoc: vec3f;"
DATA "var<private> oCtr: vec3f;"
DATA "var<private> evT: i32;"
DATA "var<private> evP: f32;"
DATA "var<private> lid: f32;"
DATA "var<private> dark: f32;"
DATA "var<private> figOn: bool;"
DATA "var<private> dissolve: f32;"
DATA "var<private> glowAcc: f32;"
DATA "var<private> wallTint: vec3f;"
DATA "var<private> sunDir: vec3f;"
DATA "var<private> sunCol: vec3f;"
DATA "var<private> ambK: f32;"
DATA "var<private> indoor: bool;"
DATA ""
DATA "const FAR = 110.0;"
DATA "const DOOR_W = 0.85;"
DATA "const DOOR_H = 2.3;"
DATA ""
DATA "// a scene: place (0 meadow, 1 playground, 2 poolrooms, 3 hallway, 4 bedroom,"
DATA "// 5 mall, 6 walkway over the clouds, 7 classroom, 8 night street, 9 mirror"
DATA "// water, 10 void), its variant, sky, exit doorway (0 white door, 1 stone"
DATA "// arch, 2 pastel frame, 3 old TV, 4 glowing frame, 5 red door, 6 elevator),"
DATA "// where the watcher is (0 nowhere, 1 floats ahead beside the path, 2 waits at"
DATA "// a set place, 3 giant in the sky, 4 at the end of the hall, 5 behind you),"
DATA "// entry doorway (the exit of the scene before) and length"
DATA "struct SC { env: i32, v: i32, sky: i32, door: i32, eye: i32, ent: i32, L: f32 };"
DATA ""
DATA "var<private> S: SC;"
DATA "var<private> SA: SC;"
DATA "var<private> SB: SC;"
DATA ""
DATA "fn envLen(env: i32) -> f32 {"
DATA "    var l = 28.0;"
DATA "    switch env {"
DATA "        case 0, 3, 6, 8: { l = 30.0; }"
DATA "        case 4: { l = 14.0; }"
DATA "        case 7: { l = 16.0; }"
DATA "        default: { }"
DATA "    }"
DATA "    return l;"
DATA "}"
DATA ""
DATA "fn sc5(env: i32, v: i32, sky: i32, door: i32, eye: i32) -> SC {"
DATA "    return SC(env, v, sky, door, eye, -1, envLen(env));"
DATA "}"
DATA ""
DATA "fn sceneBase(w: i32) -> SC {"
DATA "    var s = sc5(10, 3, 3, 4, 0);"
DATA "    switch w {"
DATA "        // nostalgia"
DATA "        case 0: { s = sc5(0, 0, 10, 0, 1); }"
DATA "        case 1: { s = sc5(1, 0, 10, 1, 2); }"
DATA "        case 2: { s = sc5(2, 0, 10, 0, 1); }"
DATA "        case 3: { s = sc5(4, 0, 10, 3, 2); }"
DATA "        case 4: { s = sc5(0, 1, 6, 2, 1); }"
DATA "        case 5: { s = sc5(5, 0, 10, 6, 2); }"
DATA "        case 6: { s = sc5(1, 1, 5, 0, 1); }"
DATA "        case 7: { s = sc5(2, 1, 4, 1, 4); }"
DATA "        case 8: { s = sc5(7, 0, 10, 0, 2); }"
DATA "        case 9: { s = sc5(0, 2, 0, 4, 3); }"
DATA "        // liminal"
DATA "        case 10: { s = sc5(3, 0, 1, 0, 4); }"
DATA "        case 11: { s = sc5(6, 0, 0, 2, 1); }"
DATA "        case 12: { s = sc5(2, 2, 0, 1, 5); }"
DATA "        case 13: { s = sc5(5, 1, 4, 6, 4); }"
DATA "        case 14: { s = sc5(3, 1, 0, 3, 4); }"
DATA "        case 15: { s = sc5(6, 1, 5, 1, 1); }"
DATA "        case 16: { s = sc5(8, 0, 4, 0, 2); }"
DATA "        case 17: { s = sc5(7, 1, 5, 0, 4); }"
DATA "        case 18: { s = sc5(3, 2, 1, 6, 5); }"
DATA "        case 19: { s = sc5(6, 2, 6, 4, 3); }"
DATA "        // uncanny"
DATA "        case 20: { s = sc5(4, 1, 4, 3, 2); }"
DATA "        case 21: { s = sc5(3, 3, 9, 5, 4); }"
DATA "        case 22: { s = sc5(9, 0, 0, 1, 1); }"
DATA "        case 23: { s = sc5(8, 1, 5, 0, 1); }"
DATA "        case 24: { s = sc5(2, 3, 9, 5, 4); }"
DATA "        case 25: { s = sc5(0, 3, 4, 0, 3); }"
DATA "        case 26: { s = sc5(5, 2, 1, 6, 5); }"
DATA "        case 27: { s = sc5(7, 2, 9, 5, 4); }"
DATA "        case 28: { s = sc5(6, 3, 1, 1, 1); }"
DATA "        case 29: { s = sc5(1, 2, 4, 5, 2); }"
DATA "        // distortion"
DATA "        case 30: { s = sc5(9, 1, 7, 4, 1); }"
DATA "        case 31: { s = sc5(9, 2, 8, 1, 3); }"
DATA "        case 32: { s = sc5(3, 4, 2, 5, 4); }"
DATA "        case 33: { s = sc5(4, 2, 9, 3, 2); }"
DATA "        case 34: { s = sc5(10, 1, 7, 4, 1); }"
DATA "        case 35: { s = sc5(8, 2, 2, 5, 1); }"
DATA "        case 36: { s = sc5(9, 1, 7, 4, 5); }"
DATA "        case 37: { s = sc5(2, 4, 2, 5, 4); }"
DATA "        case 38: { s = sc5(6, 4, 9, 4, 1); }"
DATA "        case 39: { s = sc5(9, 2, 8, 1, 3); }"
DATA "        // the watcher"
DATA "        case 40: { s = sc5(9, 3, 2, 5, 1); }"
DATA "        case 41: { s = sc5(10, 2, 9, 4, 1); }"
DATA "        case 42: { s = sc5(3, 4, 2, 3, 4); }"
DATA "        case 43: { s = sc5(0, 4, 2, 5, 3); }"
DATA "        case 44: { s = sc5(9, 4, 9, 4, 1); }"
DATA "        case 45: { s = sc5(4, 3, 2, 3, 2); }"
DATA "        case 46: { s = sc5(10, 2, 9, 4, 1); }"
DATA "        case 47: { s = sc5(9, 3, 2, 5, 5); }"
DATA "        case 48: { s = sc5(10, 0, 9, 4, 1); }"
DATA "        default: { }"
DATA "    }"
DATA "    return s;"
DATA "}"
DATA ""
DATA "fn scene(w: i32) -> SC {"
DATA "    var s = sceneBase(w);"
DATA "    if (w > 0) { s.ent = sceneBase(w - 1).door; }"
DATA "    return s;"
DATA "}"
DATA ""
DATA "fn fogOf(sky: i32) -> vec3f {"
DATA "    var c = vec3f(225.0, 190.0, 235.0);"
DATA "    switch sky {"
DATA "        case 1: { c = vec3f(150.0, 115.0, 170.0); }"
DATA "        case 2: { c = vec3f(45.0, 4.0, 14.0); }"
DATA "        case 3: { c = vec3f(255.0, 255.0, 255.0); }"
DATA "        case 4: { c = vec3f(40.0, 40.0, 82.0); }"
DATA "        case 5: { c = vec3f(240.0, 160.0, 150.0); }"
DATA "        case 6: { c = vec3f(195.0, 230.0, 230.0); }"
DATA "        case 7: { c = vec3f(215.0, 200.0, 240.0); }"
DATA "        case 8: { c = vec3f(225.0, 200.0, 180.0); }"
DATA "        case 9: { c = vec3f(55.0, 32.0, 85.0); }"
DATA "        case 10: { c = vec3f(205.0, 225.0, 245.0); }"
DATA "        default: { }"
DATA "    }"
DATA "    return c;"
DATA "}"
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
DATA ""
DATA "// ---- doorways (opening |x| < 0.85, 0 < y < 2.3 around z = 0 of q) ----"
DATA "fn doorway(q: vec3f, ty: i32) -> f32 {"
DATA "    let open = box3(q - vec3f(0.0, 1.15, 0.0), vec3f(DOOR_W, 1.15, 0.6));"
DATA "    var d = max(box3(q - vec3f(0.0, 1.25, 0.0), vec3f(1.05, 1.25, 0.1)), -open);"
DATA "    oMat = 15;"
DATA "    oLoc = q;"
DATA "    switch ty {"
DATA "        case 0, 5: {"
DATA "            // a door swung wide open beside the opening"
DATA "            let hq = q - vec3f(-DOOR_W, 1.15, 0.0);"
DATA "            let r = rot2(hq.xz, -1.75);"
DATA "            let panel = box3(vec3f(r.x - 0.84, hq.y, r.y), vec3f(0.82, 1.13, 0.035));"
DATA "            if (ty == 5) { oMat = 12; }"
DATA "            if (panel < d) { d = panel; oMat = select(15, 33, ty == 5); }"
DATA "        }"
DATA "        case 1: {"
DATA "            let a = max(box3(q - vec3f(0.0, 2.85, 0.0), vec3f(1.45, 0.6, 0.22)), -(length(q.xy - vec2f(0.0, 2.3)) - DOOR_W));"
DATA "            d = min(max(box3(q - vec3f(0.0, 1.25, 0.0), vec3f(1.45, 1.25, 0.22)), -open), a);"
DATA "            oMat = 22;"
DATA "        }"
DATA "        case 2: {"
DATA "            d = max(box3(q - vec3f(0.0, 1.3, 0.0), vec3f(1.15, 1.3, 0.14)), -open) - 0.02;"
DATA "            oMat = 22;"
DATA "        }"
DATA "        case 3: {"
DATA "            // the doorway is the screen of a huge old TV"
DATA "            d = max(box3(q - vec3f(0.0, 1.35, 0.35), vec3f(1.6, 1.6, 0.55)) - 0.08, -open);"
DATA "            oMat = 35;"
DATA "            let rim = max(box3(q - vec3f(0.0, 1.15, -0.2), vec3f(1.0, 1.3, 0.04)), -open);"
DATA "            if (rim < d) { d = rim; oMat = 26; }"
DATA "            let aq = vec3f(abs(q.x), q.y, q.z);"
DATA "            let ant = capsule(aq, vec3f(0.0, 3.0, 0.35), vec3f(0.9, 4.2, 0.35), 0.03);"
DATA "            if (ant < d) { d = ant; oMat = 9; }"
DATA "        }"
DATA "        case 4: {"
DATA "            d = max(box3(q - vec3f(0.0, 1.2, 0.0), vec3f(0.95, 1.2, 0.05)), -open);"
DATA "            oMat = 30;"
DATA "        }"
DATA "        default: {"
DATA "            d = max(box3(q - vec3f(0.0, 1.3, 0.0), vec3f(1.5, 1.3, 0.15)), -open);"
DATA "            oMat = 9;"
DATA "            let sl = box3(vec3f(abs(q.x) - 1.15, q.y - 1.15, q.z - 0.05), vec3f(0.3, 1.15, 0.03));"
DATA "            if (sl < d) { d = sl; oMat = 9; }"
DATA "        }"
DATA "    }"
DATA "    return d;"
DATA "}"
DATA ""
DATA "// put a piece built around y = -3.6 (its floor) on the ground at (x, h, z)"
DATA "fn onGround(p: vec3f, x: f32, h: f32, z: f32) -> vec3f { return p - vec3f(x, h + 3.6, z); }"
DATA ""
DATA "fn tree(q: vec3f) -> f32 {"
DATA "    var d = capsule(q, vec3f(0.0, 0.0, 0.0), vec3f(0.0, 2.6, 0.0), 0.22);"
DATA "    oMat = 12;"
DATA "    var c = length(q - vec3f(0.0, 3.4, 0.0)) - 1.45;"
DATA "    c = smin(c, length(q - vec3f(0.95, 2.9, 0.3)) - 1.0, 0.5);"
DATA "    c = smin(c, length(q - vec3f(-0.85, 3.0, -0.4)) - 1.1, 0.5);"
DATA "    c = smin(c, length(q - vec3f(0.2, 4.1, -0.2)) - 0.9, 0.5);"
DATA "    if (c < d) { d = c; oMat = 36; }"
DATA "    return d;"
DATA "}"
DATA ""
DATA "var<private> wd: f32;"
DATA "var<private> wm: i32;"
DATA "var<private> wl: vec3f;"
DATA "var<private> wc: vec3f;"
DATA "fn put(d: f32) {"
DATA "    if (d < wd) { wd = d; wm = oMat; wl = oLoc; }"
DATA "}"
DATA ""
DATA "// ---- 0 meadow: rolling hills, a few things standing in the grass ----"
DATA "fn mh(x: f32, z: f32) -> f32 {"
DATA "    var h = 0.45 * sin(x * 0.17 + 1.0) * sin(z * 0.13) + 0.25 * sin(x * 0.37 + z * 0.21);"
DATA "    h = h * smoothstep(1.5, 7.0, abs(x));"
DATA "    return h + 3.0 * smoothstep(14.0, 45.0, abs(x)) * (0.6 + 0.4 * sin(z * 0.04 + x * 0.02));"
DATA "}"
DATA ""
DATA "fn envMeadow(p: vec3f, v: i32) {"
DATA "    wd = (p.y - mh(p.x, p.z)) * 0.7; wm = 10; wl = p;"
DATA "    if (v == 0) {"
DATA "        put(tree(p - vec3f(6.5, mh(6.5, 7.0), 7.0)));"
DATA "        put(doorway(spinY(p - vec3f(-7.0, mh(-7.0, 11.0), 11.0), 0.5), 0));"
DATA "        put(doorway(spinY(p - vec3f(9.0, mh(9.0, 19.0), 19.0), -0.4), 2));"
DATA "        put(doorway(spinY(p - vec3f(-12.0, mh(-12.0, 24.0), 24.0), 0.2), 0));"
DATA "    } else if (v == 1) {"
DATA "        put(oTV(spinY(p - vec3f(-3.8, mh(-3.8, 10.0) + 0.85, 10.0), 0.3), 1.0));"
DATA "        put(oBed(spinY(onGround(p, 4.8, mh(4.8, 17.0), 17.0), -0.3)));"
DATA "        put(tree(p - vec3f(-10.0, mh(-10.0, 22.0), 22.0)));"
DATA "        let pz = p.z - 9.0 * round(p.z / 9.0);"
DATA "        var pole = capsule(vec3f(p.x + 9.0, p.y, pz), vec3f(0.0, -1.0, 0.0), vec3f(0.0, 7.2, 0.0), 0.12);"
DATA "        pole = min(pole, box3(vec3f(p.x + 9.0, p.y - 6.8, pz), vec3f(1.1, 0.07, 0.07)));"
DATA "        let sag = 0.6 * (1.0 - pz * pz / 20.25);"
DATA "        pole = min(pole, length(vec2f(abs(p.x + 9.0) - 0.9, p.y - 6.75 + sag)) - 0.02);"
DATA "        if (pole < wd) { wd = pole; wm = 19; wl = p; }"
DATA "    } else if (v == 2) {"
DATA "        // stairs to nowhere and a door at the top"
DATA "        let sq = p - vec3f(5.5, 0.0, 6.0);"
DATA "        let k = clamp(floor(sq.z / 0.7), 0.0, 15.0);"
DATA "        var st = 100.0;"
DATA "        for (var n = 0; n < 2; n++) {"
DATA "            let kk = min(k + f32(n), 15.0);"
DATA "            st = min(st, box3(sq - vec3f(0.0, 0.175 + kk * 0.35, kk * 0.7 + 0.35), vec3f(0.9, 0.175 + 0.0, 0.35)));"
DATA "        }"
DATA "        if (st < wd) { wd = st; wm = 22; wl = sq; }"
DATA "        put(doorway(p - vec3f(5.5, 5.6, 17.6), 4));"
DATA "        put(oStair(p - vec3f(-9.0, 3.5 + mh(-9.0, 20.0), 20.0), 0.3));"
DATA "    } else if (v == 3) {"
DATA "        put(oSwing(onGround(p, -7.5, mh(-7.5, 14.0), 14.0), 0.4));"
DATA "        put(oLamp(onGround(p, -4.6, mh(-4.6, 8.0), 8.0), 1.0));"
DATA "        put(tree(p - vec3f(8.0, mh(8.0, 20.0), 20.0)));"
DATA "    } else {"
DATA "        // a field of doors"
DATA "        let cx = round(p.x / 7.0);"
DATA "        let cz = round(p.z / 9.0);"
DATA "        if (abs(cx) >= 1.0) {"
DATA "            let h = hs(cx * 3.7 + cz * 1.9);"
DATA "            let q = vec3f(p.x - cx * 7.0, p.y - mh(cx * 7.0, cz * 9.0), p.z - cz * 9.0);"
DATA "            let dd = doorway(spinY(q, (h - 0.5) * 1.5), select(0, 5, h > 0.5));"
DATA "            let bnd = max(min(3.5 - abs(q.x), 4.5 - abs(q.z)), 0.0) + 0.5;"
DATA "            put(min(dd, bnd));"
DATA "        }"
DATA "    }"
DATA "}"
DATA ""
DATA "// ---- 1 playground ----"
DATA "fn envPlay(p: vec3f, v: i32) {"
DATA "    wd = p.y; wm = 37; wl = p;"
DATA "    put(oSwing(onGround(p, -7.5, 0.0, 9.0), 0.2));"
DATA "    put(oSlide(spinY(onGround(p, 7.0, 0.0, 14.0), 3.14159)));"
DATA "    // merry-go-round"
DATA "    let m = spinY(p - vec3f(-6.8, 0.0, 20.0), T * 0.35);"
DATA "    var mg = max(length(m.xz) - 1.7, abs(m.y - 0.55) - 0.06);"
DATA "    mg = min(mg, capsule(m, vec3f(0.0, 0.0, 0.0), vec3f(0.0, 1.3, 0.0), 0.06));"
DATA "    let ma = vec3f(abs(m.x), m.y, abs(m.z));"
DATA "    mg = min(mg, capsule(ma, vec3f(0.3, 1.25, 0.3), vec3f(1.2, 1.1, 1.2), 0.035));"
DATA "    mg = min(mg, capsule(ma, vec3f(1.2, 0.6, 1.2), vec3f(1.2, 1.1, 1.2), 0.035));"
DATA "    if (mg < wd) { wd = mg; wm = 13; wl = m; }"
DATA "    // seesaw"
DATA "    let s = p - vec3f(6.8, 0.0, 23.0);"
DATA "    let r = rot2(vec2f(s.z, s.y - 0.55), 0.25 * sin(T * 0.8));"
DATA "    var ss = box3(vec3f(s.x, r.y, r.x), vec3f(0.14, 0.05, 1.9));"
DATA "    ss = min(ss, box3(s - vec3f(0.0, 0.27, 0.0), vec3f(0.2, 0.27, 0.15)));"
DATA "    if (ss < wd) { wd = ss; wm = 13; wl = s; }"
DATA "    // fence"
DATA "    let fz = p.z - 2.0 * round(p.z / 2.0);"
DATA "    var fe = box3(vec3f(abs(p.x) - 10.5, p.y - 0.55, fz), vec3f(0.06, 0.55, 0.06));"
DATA "    fe = min(fe, box3(vec3f(abs(p.x) - 10.5, p.y - 0.85, 0.0), vec3f(0.04, 0.04, 100.0)));"
DATA "    fe = min(fe, box3(vec3f(abs(p.x) - 10.5, p.y - 0.4, 0.0), vec3f(0.04, 0.04, 100.0)));"
DATA "    if (fe < wd) { wd = fe; wm = 15; wl = p; }"
DATA "    if (v >= 1) { put(oLamp(onGround(p, -4.8, 0.0, 5.0), 1.0)); put(oLamp(onGround(p, 4.8, 0.0, 17.0), -1.0)); }"
DATA "}"
DATA ""
DATA "// ---- 2 poolrooms ----"
DATA "fn envPool(p: vec3f, L: f32) {"
DATA "    // the rooms are hollow boxes: the solid is outside all of them"
DATA "    let hall = box3(p - vec3f(0.0, 2.3, L * 0.5), vec3f(7.0, 3.7, L * 0.5));"
DATA "    var room = hall;"
DATA "    let az = p.z - 6.0 * clamp(round((p.z - 3.0) / 6.0), 0.0, 3.0) - 3.0;"
DATA "    let arch = min(box3(vec3f(abs(p.x) - 7.2, p.y - 0.9, az), vec3f(0.6, 2.3, 1.1)), length(vec2f(az, p.y - 3.2)) - 1.1);"
DATA "    room = min(room, max(arch, abs(abs(p.x) - 7.2) - 0.6));"
DATA "    room = min(room, box3(vec3f(abs(p.x) - 11.0, p.y - 2.3, p.z - L * 0.5), vec3f(3.4, 3.7, L * 0.5)));"
DATA "    let skyl = box3(vec3f(abs(p.x) - 3.5, p.y - 6.2, p.z - 6.0 * round(p.z / 6.0)), vec3f(1.4, 0.6, 1.4));"
DATA "    room = min(room, max(skyl, -(p.z - 0.5)));"
DATA "    room = min(room, box3(p - vec3f(0.0, 1.15, L), vec3f(DOOR_W, 1.15, 0.25)));"
DATA "    room = min(room, box3(p - vec3f(0.0, 1.15, 0.0), vec3f(DOOR_W, 1.15, 0.25)));"
DATA "    wd = -room; wm = 32; wl = p;"
DATA "    // walkway, pool basins and the water in them"
DATA "    let walk = box3(p - vec3f(0.0, -0.75, L * 0.5), vec3f(1.3, 0.75, L * 0.5 + 1.0));"
DATA "    let lip = box3(vec3f(abs(p.x) - 6.6, p.y + 0.75, p.z - L * 0.5), vec3f(0.4, 0.75, L * 0.5 + 1.0));"
DATA "    let solid = min(min(walk, lip), p.y + 1.4);"
DATA "    if (solid < wd) { wd = solid; wm = 32; wl = p; }"
DATA "    let water = p.y + 0.22;"
DATA "    if (water < wd && abs(p.x) > 1.3) { wd = water; wm = 4; wl = p; }"
DATA "    // tiled columns standing in the water"
DATA "    let cz = p.z - 6.0 * clamp(round((p.z - 6.0) / 6.0), 0.0, 3.0) - 6.0;"
DATA "    put(box3(vec3f(abs(p.x) - 4.2, p.y - 1.8, cz), vec3f(0.35, 3.3, 0.35)));"
DATA "    // ladders"
DATA "    let lq = vec3f(abs(p.x) - 1.55, p.y, abs(p.z - 13.0) - 0.3);"
DATA "    let lad = min(capsule(lq, vec3f(0.12, -1.0, 0.0), vec3f(0.12, 0.6, 0.0), 0.035), capsule(lq, vec3f(0.12, 0.6, 0.0), vec3f(-0.15, 0.6, 0.0), 0.035));"
DATA "    if (lad < wd) { wd = lad; wm = 9; wl = p; }"
DATA "}"
DATA ""
DATA "// ---- 3 hallway (backrooms) ----"
DATA "fn envHall(p: vec3f, L: f32) {"
DATA "    let i1 = box3(p - vec3f(0.0, 1.4, L * 0.5), vec3f(1.8, 1.4, L * 0.5));"
DATA "    let i2 = box3(p - vec3f(-21.0, 1.4, 14.0), vec3f(19.2, 1.4, 1.0));"
DATA "    // doors along the walls; every other one stands open onto a dark room"
DATA "    let k = clamp(round((p.z - 2.5) / 5.0), 0.0, 5.0);"
DATA "    let dz = p.z - 2.5 - k * 5.0;"
DATA "    let side = select(-1.0, 1.0, fract(k * 0.5) > 0.25);"
DATA "    let sx = p.x * side;"
DATA "    let rec = box3(vec3f(sx - 1.9, p.y - 1.05, dz), vec3f(0.15, 1.05, 0.5));"
DATA "    var i3 = rec;"
DATA "    if (hs(k * 7.1) > 0.5) { i3 = min(rec, box3(vec3f(sx - 3.4, p.y - 1.2, dz), vec3f(1.5, 1.2, 1.6))); }"
DATA "    var room = min(min(i1, i2), i3);"
DATA "    room = min(room, box3(p - vec3f(0.0, 1.15, L), vec3f(DOOR_W, 1.15, 0.25)));"
DATA "    room = min(room, box3(p - vec3f(0.0, 1.15, 0.0), vec3f(DOOR_W, 1.15, 0.25)));"
DATA "    wd = -room; wm = 21; wl = p;"
DATA "    if (p.y < 0.02) { wm = 39; }"
DATA "    if (p.y > 2.78) { wm = 32; }"
DATA "    if (hs(k * 7.1) <= 0.5) {"
DATA "        let door = box3(vec3f(sx - 1.98, p.y - 1.05, dz), vec3f(0.03, 1.02, 0.47));"
DATA "        if (door < wd) { wd = door; wm = 15; wl = p; }"
DATA "    }"
DATA "    let lz = p.z - 3.0 * clamp(round((p.z - 1.5) / 3.0), 0.0, 9.0) - 1.5;"
DATA "    let lamp = box3(vec3f(p.x, p.y - 2.79, lz), vec3f(0.55, 0.02, 0.28));"
DATA "    if (lamp < wd) { wd = lamp; wm = 38; wl = vec3f(p.z, 0.0, 0.0); }"
DATA "}"
DATA ""
DATA "// ---- 4 bedroom ----"
DATA "fn envBedroom(p: vec3f, L: f32, v: i32) {"
DATA "    var room = box3(p - vec3f(0.0, 1.6, L * 0.5), vec3f(4.5, 1.6, L * 0.5));"
DATA "    room = min(room, box3(p - vec3f(-4.7, 1.75, 9.0), vec3f(0.5, 0.75, 1.3)));"
DATA "    room = min(room, box3(p - vec3f(0.0, 1.15, L), vec3f(DOOR_W, 1.15, 0.25)));"
DATA "    room = min(room, box3(p - vec3f(0.0, 1.15, 0.0), vec3f(DOOR_W, 1.15, 0.25)));"
DATA "    wd = -room; wm = 21; wl = p;"
DATA "    if (p.y < 0.02) { wm = 12; }"
DATA "    let mul = min(box3(p - vec3f(-4.55, 1.75, 9.0), vec3f(0.05, 0.75, 0.04)), box3(p - vec3f(-4.55, 1.75, 9.0), vec3f(0.05, 0.04, 1.3)));"
DATA "    if (mul < wd) { wd = mul; wm = 15; wl = p; }"
DATA "    put(oBed(onGround(p, -2.7, 0.0, 5.0)));"
DATA "    put(box3(p - vec3f(-3.9, 0.3, 2.4), vec3f(0.4, 0.3, 0.4)));"
DATA "    let lampq = p - vec3f(-3.9, 0.85, 2.4);"
DATA "    let lp = length(lampq) - 0.2;"
DATA "    if (lp < wd) { wd = lp; wm = 3; wl = p; }"
DATA "    put(box3(p - vec3f(3.9, 0.45, 9.0), vec3f(0.5, 0.45, 1.2)));"
DATA "    put(oTV(spinY(p - vec3f(3.85, 1.45, 9.0), 0.0) * 1.8, -1.0) / 1.8);"
DATA "    let rug = max(length(p.xz - vec2f(0.0, 7.0)) - 2.3, abs(p.y) - 0.015);"
DATA "    if (rug < wd) { wd = rug; wm = 41; wl = p; }"
DATA "    let bq = p - vec3f(1.8, 0.0, 3.5);"
DATA "    put(min(box3(bq - vec3f(0.0, 0.2, 0.0), vec3f(0.2)), box3(spinY(bq - vec3f(0.45, 0.2, 0.3), 0.6), vec3f(0.2))));"
DATA "    let ball = length(p - vec3f(1.4, 0.3, 11.5)) - 0.3;"
DATA "    if (ball < wd) { wd = ball; wm = 13; wl = p; }"
DATA "    let post = box3(vec3f(abs(p.x) - 4.48, p.y - 1.8, p.z - select(4.0, 12.0, p.x > 0.0)), vec3f(0.02, 0.6, 0.45));"
DATA "    if (post < wd) { wd = post; wm = select(42, 48, v >= 2); wl = p - vec3f(0.0, 1.8, select(4.0, 12.0, p.x > 0.0)); }"
DATA "    let ceil = length(p - vec3f(0.0, 3.05, 7.0)) - 0.25;"
DATA "    if (ceil < wd) { wd = ceil; wm = 3; wl = p; }"
DATA "}"
DATA ""
DATA "// ---- 5 empty mall ----"
DATA "fn envMall(p: vec3f, L: f32) {"
DATA "    var room = box3(p - vec3f(0.0, 5.0, L * 0.5), vec3f(12.0, 5.0, L * 0.5));"
DATA "    room = min(room, box3(p - vec3f(0.0, 10.5, L * 0.5), vec3f(4.0, 0.8, L * 0.5 - 1.0)));"
DATA "    room = min(room, box3(p - vec3f(0.0, 1.15, L), vec3f(DOOR_W, 1.15, 0.25)));"
DATA "    room = min(room, box3(p - vec3f(0.0, 1.15, 0.0), vec3f(DOOR_W, 1.15, 0.25)));"
DATA "    wd = -room; wm = 45; wl = p;"
DATA "    if (p.y < 0.02) { wm = 34; }"
DATA "    if (p.y > 9.9) { wm = 32; }"
DATA "    let bal = box3(vec3f(abs(p.x) - 10.5, p.y - 4.45, p.z - L * 0.5), vec3f(1.5, 0.15, L * 0.5));"
DATA "    put(bal);"
DATA "    let rail = box3(vec3f(abs(p.x) - 9.05, p.y - 5.1, p.z - L * 0.5), vec3f(0.04, 0.5, L * 0.5));"
DATA "    if (rail < wd) { wd = rail; wm = 9; wl = p; }"
DATA "    let cz = p.z - 6.0 * clamp(round((p.z - 4.0) / 6.0), 0.0, 3.0) - 4.0;"
DATA "    let col = length(vec2f(abs(p.x) - 8.6, cz)) - 0.45;"
DATA "    if (col < wd) { wd = col; wm = 32; wl = p; }"
DATA "    // fountain"
DATA "    let f = p - vec3f(-5.5, 0.0, 14.0);"
DATA "    let fr = length(f.xz);"
DATA "    let basin = max(abs(fr - 2.1) - 0.15, f.y - 0.6);"
DATA "    put(basin);"
DATA "    let fw = max(fr - 2.0, abs(f.y - 0.42) - 0.02);"
DATA "    if (fw < wd) { wd = fw; wm = 4; wl = p; }"
DATA "    put(max(fr - 0.25, f.y - 1.6));"
DATA "    put(max(abs(length(vec2f(fr, f.y - 1.6)) - 0.7) - 0.05, f.y - 1.6));"
DATA "    // planters with round shrubs"
DATA "    let pq = vec3f(p.x - 5.6, p.y, p.z - 8.0 * clamp(round((p.z - 6.0) / 8.0), 0.0, 2.0) - 6.0);"
DATA "    let box = box3(pq - vec3f(0.0, 0.45, 0.0), vec3f(0.7, 0.45, 0.7));"
DATA "    if (box < wd) { wd = box; wm = 22; wl = p; }"
DATA "    let bush = length(pq - vec3f(0.0, 1.5, 0.0)) - 0.8;"
DATA "    if (bush < wd) { wd = bush; wm = 36; wl = p; }"
DATA "    // a staircase up to the balcony"
DATA "    let eq = p - vec3f(7.4, 0.0, 9.0);"
DATA "    let ek = clamp(floor(eq.z / 0.5), 0.0, 15.0);"
DATA "    var es = box3(eq - vec3f(0.0, 0.14 + ek * 0.28, ek * 0.5 + 0.25), vec3f(0.9, 0.14 + ek * 0.0, 0.25));"
DATA "    es = min(es, box3(eq - vec3f(0.0, (ek + 1.0) * 0.14, ek * 0.5 + 0.25), vec3f(0.9, (ek + 1.0) * 0.14, 0.25)));"
DATA "    put(es);"
DATA "    // a bench"
DATA "    put(box3(p - vec3f(-4.0, 0.45, 5.0), vec3f(1.2, 0.06, 0.35)));"
DATA "}"
DATA ""
DATA "// ---- 6 a walkway over the clouds ----"
DATA "fn envWalk(p: vec3f, L: f32, v: i32) {"
DATA "    wd = box3(p - vec3f(0.0, -0.3, L * 0.5), vec3f(1.15, 0.3, L * 0.5 + 0.8)); wm = 22; wl = p;"
DATA "    let az = p.z - 7.5 * clamp(round((p.z - 5.0) / 7.5), 0.0, 2.0) - 5.0;"
DATA "    let aq = vec3f(p.x, p.y, az);"
DATA "    let arch = max(box3(aq - vec3f(0.0, 2.0, 0.0), vec3f(2.0, 2.6, 0.22)), -min(box3(aq - vec3f(0.0, 0.8, 0.0), vec3f(1.45, 2.2, 0.5)), length(aq.xy - vec2f(0.0, 3.0)) - 1.45));"
DATA "    put(max(arch, -(p.y + 0.6)));"
DATA "    put(oStair(p - vec3f(-7.5, 3.0, 9.0), 0.2));"
DATA "    put(oStair(p - vec3f(8.5, -1.0, 18.0), 0.7));"
DATA "    put(oStair(p - vec3f(-11.0, 1.0, 26.0), 0.5));"
DATA "    put(oStraightStairs(p - vec3f(6.5, 4.5, 5.0), 0.3));"
DATA "    put(oFrame(p - vec3f(9.0, 5.5, 25.0), 0.4));"
DATA "    put(oFrame(p - vec3f(-6.0, 6.0, 3.0), 0.8));"
DATA "    let aqd = vec3f(p.x + 18.0, p.y + 2.0, p.z - 8.8 * round(p.z / 8.8));"
DATA "    put(oAqueduct(aqd));"
DATA "    if (v >= 3) { put(oSpiral(p - vec3f(6.0, 2.0, 13.0), 0.3)); }"
DATA "}"
DATA ""
DATA "// ---- 7 classroom ----"
DATA "fn envClass(p: vec3f, L: f32) {"
DATA "    var room = box3(p - vec3f(0.0, 1.6, L * 0.5), vec3f(5.0, 1.6, L * 0.5));"
DATA "    let wz = p.z - 4.0 * clamp(round((p.z - 3.0) / 4.0), 0.0, 2.0) - 3.0;"
DATA "    room = min(room, box3(vec3f(p.x - 5.2, p.y - 1.75, wz), vec3f(0.5, 0.8, 1.2)));"
DATA "    room = min(room, box3(p - vec3f(0.0, 1.15, L), vec3f(DOOR_W, 1.15, 0.25)));"
DATA "    room = min(room, box3(p - vec3f(0.0, 1.15, 0.0), vec3f(DOOR_W, 1.15, 0.25)));"
DATA "    wd = -room; wm = 21; wl = p;"
DATA "    if (p.y < 0.02) { wm = 46; }"
DATA "    let mul = box3(vec3f(p.x - 5.05, p.y - 1.75, wz), vec3f(0.04, 0.8, 0.04));"
DATA "    if (mul < wd) { wd = mul; wm = 15; wl = p; }"
DATA "    let board = box3(p - vec3f(-4.96, 1.7, 8.0), vec3f(0.04, 0.8, 2.6));"
DATA "    if (board < wd) { wd = board; wm = 16; wl = p; }"
DATA "    // desks in rows on both sides of the aisle"
DATA "    let ax = abs(p.x);"
DATA "    let cx = select(1.9, 3.5, ax > 2.7);"
DATA "    let cz = clamp(round((p.z - 3.0) / 2.5), 0.0, 3.0) * 2.5 + 3.0;"
DATA "    let q = vec3f(ax - cx, p.y, p.z - cz);"
DATA "    var dk = box3(q - vec3f(0.0, 0.74, 0.0), vec3f(0.5, 0.03, 0.33));"
DATA "    dk = min(dk, box3(vec3f(abs(q.x) - 0.44, q.y - 0.37, abs(q.z) - 0.27), vec3f(0.025, 0.37, 0.025)));"
DATA "    if (dk < wd) { wd = dk; wm = 12; wl = q; }"
DATA "    var ch = box3(q - vec3f(0.0, 0.45, -0.62), vec3f(0.24, 0.025, 0.22));"
DATA "    ch = min(ch, box3(q - vec3f(0.0, 0.75, -0.85), vec3f(0.24, 0.28, 0.025)));"
DATA "    ch = min(ch, box3(vec3f(abs(q.x) - 0.2, q.y - 0.22, abs(q.z + 0.62) - 0.18), vec3f(0.02, 0.22, 0.02)));"
DATA "    if (ch < wd) { wd = ch; wm = 9; wl = q; }"
DATA "    put(box3(p - vec3f(2.8, 0.45, 13.6), vec3f(0.9, 0.45, 0.5)));"
DATA "    let clk = max(length(p.xy - vec2f(-2.6, 2.5)) - 0.32, abs(p.z - (L - 0.02)) - 0.03);"
DATA "    if (clk < wd) { wd = clk; wm = 49; wl = p - vec3f(-2.6, 2.5, L); }"
DATA "    let lz = p.z - 4.0 * clamp(round((p.z - 2.0) / 4.0), 0.0, 3.0) - 2.0;"
DATA "    let lamp = box3(vec3f(abs(p.x) - 2.2, p.y - 3.19, lz), vec3f(0.5, 0.02, 0.25));"
DATA "    if (lamp < wd) { wd = lamp; wm = 38; wl = vec3f(p.z, 0.0, 0.0); }"
DATA "}"
DATA ""
DATA "// ---- 8 a street at night ----"
DATA "fn envStreet(p: vec3f, v: i32) {"
DATA "    let ax = abs(p.x);"
DATA "    wd = p.y; wm = 25; wl = p;"
DATA "    let walk = box3(vec3f(ax - 5.0, p.y, 0.0), vec3f(1.0, 0.15, 1000.0));"
DATA "    if (walk < wd) { wd = walk; wm = 31; wl = p; }"
DATA "    let lawn = max(p.y - 0.05, 6.0 - ax);"
DATA "    if (lawn < wd) { wd = lawn; wm = 10; wl = p; }"
DATA "    let sd = sign(p.x + 0.0001);"
DATA "    let hz = p.z - 6.0 * f32(sd > 0.0);"
DATA "    let hq = vec3f(ax - 12.5, p.y - 3.6 - 0.05, hz - 12.0 * round(hz / 12.0));"
DATA "    put(oHouse(vec3f(-hq.x, hq.y, hq.z), 1.0));"
DATA "    let lq = vec3f(ax - 4.9, p.y - 3.6, p.z - 10.0 * round((p.z - 5.0) / 10.0) - 5.0);"
DATA "    put(oLamp(vec3f(-lq.x, lq.y, lq.z), 1.0));"
DATA "    let pz = p.z - 9.0 * round(p.z / 9.0);"
DATA "    var pole = capsule(vec3f(p.x + 6.6, p.y, pz), vec3f(0.0, 0.0, 0.0), vec3f(0.0, 7.2, 0.0), 0.12);"
DATA "    let sag = 0.6 * (1.0 - pz * pz / 20.25);"
DATA "    pole = min(pole, length(vec2f(p.x + 6.6, p.y - 6.9 + sag)) - 0.02);"
DATA "    if (pole < wd) { wd = pole; wm = 19; wl = p; }"
DATA "    let fz = p.z - 0.35 * round(p.z / 0.35);"
DATA "    let pick = box3(vec3f(ax - 7.4, p.y - 0.5, fz), vec3f(0.04, 0.5, 0.06));"
DATA "    if (pick < wd) { wd = pick; wm = 15; wl = p; }"
DATA "}"
DATA ""
DATA "// ---- 9 mirror water ----"
DATA "fn envMirror(p: vec3f, v: i32) {"
DATA "    wd = p.y; wm = 47; wl = p;"
DATA "    if (v == 0) {"
DATA "        let cz = p.z - 4.0 * round((p.z - 2.0) / 4.0) - 2.0;"
DATA "        let q = vec3f(abs(p.x) - 4.2, p.y, cz);"
DATA "        let ang = atan2(q.z, q.x);"
DATA "        var c = max(length(q.xz) - (0.42 + 0.03 * cos(16.0 * ang)), abs(q.y - 3.0) - 3.0) * 0.85;"
DATA "        c = min(c, box3(q - vec3f(0.0, 6.15, 0.0), vec3f(0.62, 0.15, 0.62)));"
DATA "        c = min(c, box3(q - vec3f(0.0, 0.12, 0.0), vec3f(0.6, 0.12, 0.6)));"
DATA "        let beam = box3(vec3f(abs(p.x) - 4.2, p.y - 6.5, p.z), vec3f(0.5, 0.2, 1000.0));"
DATA "        c = min(c, beam);"
DATA "        if (c < wd) { wd = c; wm = 22; wl = p; }"
DATA "        put(oSpiral(p - vec3f(0.0, 9.5, 16.0), 0.5));"
DATA "    } else if (v == 1) {"
DATA "        let sx = vec3f(abs(p.x), p.y, p.z);"
DATA "        put(oSphere(sx - vec3f(5.0, 1.6, 8.0), 0.2));"
DATA "        put(oSphere(sx - vec3f(7.5, 2.6, 19.0), 0.7));"
DATA "        put(oWireCubes(sx - vec3f(6.5, 3.2, 13.5), 0.4));"
DATA "        put(oWireCubes(p - vec3f(0.0, 7.0, 22.0), 0.9));"
DATA "        put(oSphere(p - vec3f(0.0, 6.5, 9.0), 0.5));"
DATA "    } else if (v == 2) {"
DATA "        let tz = p.z - 9.0 * round((p.z - 5.0) / 9.0) - 5.0;"
DATA "        put(oTower(vec3f(abs(p.x) - 8.5, p.y - 3.6, tz)));"
DATA "        put(oPortal(spinY(vec3f(abs(p.x) - 4.6, p.y - 2.4, p.z - 11.0), 0.0), 0.3));"
DATA "        put(oPortal(vec3f(abs(p.x) - 5.0, p.y - 2.2, p.z - 22.0), 0.7));"
DATA "    } else if (v == 3) {"
DATA "        let ez = p.z - 5.0 * round((p.z - 2.5) / 5.0) - 2.5;"
DATA "        let ep = oEyePillar(vec3f(abs(p.x) - 3.9, p.y - 3.6, ez), -1.0);"
DATA "        if (ep < wd) {"
DATA "            wd = ep; wm = oMat; wl = oLoc;"
DATA "            wc = vec3f(sign(p.x) * (3.9 + oCtr.x), 3.6 + oCtr.y, p.z - ez + oCtr.z);"
DATA "        }"
DATA "    } else {"
DATA "        let gq = vec3f(abs(p.x) - 6.5, p.y - 2.2, p.z - 8.0 * round(p.z / 8.0));"
DATA "        put(oGlitch(gq, 0.3));"
DATA "        put(oCorridor(spinY(p - vec3f(-9.0, 2.0, 15.0), 0.3)));"
DATA "    }"
DATA "}"
DATA ""
DATA "// ---- 10 void ----"
DATA "fn envVoid(p: vec3f, v: i32) {"
DATA "    // one floating thing per 9-unit cell; the column along the walk stays empty"
DATA "    let c = round(p / 9.0);"
DATA "    let q = p - c * 9.0;"
DATA "    wd = max(min(min(4.5 - abs(q.x), 4.5 - abs(q.y)), 4.5 - abs(q.z)), 0.0) + 0.5;"
DATA "    wm = 0; wl = p;"
DATA "    let h = hs(dot(c, vec3f(3.1, 7.7, 1.3)));"
DATA "    // only about one cell in three holds something: sparse, not a crowd"
DATA "    if ((abs(c.x) >= 1.0 || abs(c.y) >= 1.0) && hs(h * 53.1 + 2.0) < 0.34 && abs(c.y) <= 1.0) {"
DATA "        var e = 100.0;"
DATA "        if (v == 0) { e = oShard(q, h); }"
DATA "        else if (v == 1) { e = select(oBlob(q, h), oSphere(q, h), h > 0.55); }"
DATA "        else if (v == 2) { e = select(oCrystal(q, h), oBigEye(q * 1.4) / 1.4, h > 0.6); }"
DATA "        else { e = oFrame(q, h); }"
DATA "        if (e < wd) {"
DATA "            wd = e; wm = oMat; wl = oLoc;"
DATA "            if (oMat == 18) { wc = c * 9.0; }"
DATA "        }"
DATA "    }"
DATA "    if (v == 0 || v == 3) {"
DATA "        let path = box3(p - vec3f(0.0, -0.1, 14.0), vec3f(1.0, 0.1, 15.0));"
DATA "        if (path < wd) { wd = path; wm = 32; wl = p; }"
DATA "    }"
DATA "}"
DATA "fn envSDF(s: SC, p: vec3f) {"
DATA "    wc = vec3f(0.0);"
DATA "    switch s.env {"
DATA "        case 0: { envMeadow(p, s.v); }"
DATA "        case 1: { envPlay(p, s.v); }"
DATA "        case 2: { envPool(p, s.L); }"
DATA "        case 3: { envHall(p, s.L); }"
DATA "        case 4: { envBedroom(p, s.L, s.v); }"
DATA "        case 5: { envMall(p, s.L); }"
DATA "        case 6: { envWalk(p, s.L, s.v); }"
DATA "        case 7: { envClass(p, s.L); }"
DATA "        case 8: { envStreet(p, s.v); }"
DATA "        case 9: { envMirror(p, s.v); }"
DATA "        default: { envVoid(p, s.v); }"
DATA "    }"
DATA "    // the doorways in and out"
DATA "    let ex = doorway(p - vec3f(0.0, 0.0, s.L), s.door);"
DATA "    if (ex < wd) { wd = ex; wm = oMat; wl = oLoc; }"
DATA "    if (s.ent >= 0) {"
DATA "        let en = doorway(p, s.ent);"
DATA "        if (en < wd) { wd = en; wm = oMat; wl = oLoc; }"
DATA "    }"
DATA "}"
DATA ""
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
DATA ""
DATA "fn skyGrad(sky: i32, rd: vec3f) -> vec3f {"
DATA "    let h = rd.y;"
DATA "    if (sky == 2) { return vec3f(45.0, 3.0, 10.0); }"
DATA "    if (sky == 3) { return vec3f(255.0, 252.0, 255.0) - vec3f(10.0, 20.0, 0.0) * (1.0 - abs(h)); }"
DATA "    var top = vec3f(105.0, 62.0, 185.0);"
DATA "    var hor2 = vec3f(238.0, 168.0, 222.0);"
DATA "    switch sky {"
DATA "        case 1: { top = vec3f(55.0, 30.0, 95.0); hor2 = vec3f(190.0, 120.0, 170.0); }"
DATA "        case 4: { top = vec3f(8.0, 10.0, 36.0); hor2 = vec3f(58.0, 48.0, 108.0); }"
DATA "        case 5: { top = vec3f(105.0, 58.0, 150.0); hor2 = vec3f(255.0, 155.0, 125.0); }"
DATA "        case 6: { top = vec3f(80.0, 160.0, 205.0); hor2 = vec3f(215.0, 240.0, 235.0); }"
DATA "        case 7: { top = vec3f(150.0, 170.0, 235.0); hor2 = vec3f(240.0, 200.0, 235.0); }"
DATA "        case 8: { top = vec3f(150.0, 140.0, 175.0); hor2 = vec3f(235.0, 205.0, 175.0); }"
DATA "        case 9: { top = vec3f(16.0, 8.0, 34.0); hor2 = vec3f(82.0, 42.0, 112.0); }"
DATA "        case 10: { top = vec3f(70.0, 135.0, 230.0); hor2 = vec3f(200.0, 225.0, 250.0); }"
DATA "        default: { }"
DATA "    }"
DATA "    return mix(hor2, top, clamp(h * 1.5 + 0.25, 0.0, 1.0));"
DATA "}"
DATA ""
DATA "fn skyCol(sky: i32, rd: vec3f) -> vec3f {"
DATA "    var c: vec3f;"
DATA "    let h = rd.y;"
DATA "    if (sky == 2) {"
DATA "        let w = sin(h * 9.0 + T * 1.3 + sin(rd.x * 5.0 + T * 0.7) * 2.0);"
DATA "        c = mix(vec3f(10.0, 0.0, 6.0), vec3f(150.0, 8.0, 18.0), smoothstep(-0.4, 1.0, w) * (0.55 + 0.45 * smoothstep(-0.3, 0.5, h)));"
DATA "        let st = hash3(floor(rd * 300.0) + vec3f(floor(T * 20.0)));"
DATA "        if (st < 0.03) { c = vec3f(hash3(floor(rd * 211.0)) * 120.0); }"
DATA "        return c;"
DATA "    }"
DATA "    c = skyGrad(sky, rd);"
DATA "    if (sky == 3) { return c; }"
DATA "    if (sky == 7) {"
DATA "        // iridescent sky: rainbow bands drifting through it"
DATA "        let t = rd.y * 1.6 + rd.x * 0.5 + fbm(rd * 2.0 + vec3f(T * 0.02)) * 0.8;"
DATA "        c = mix(c, iri(t) * 255.0, 0.35);"
DATA "    }"
DATA "    if (sky == 4) {"
DATA "        let md = normalize(vec3f(0.35, 0.3, 1.0));"
DATA "        let mc = dot(rd, md);"
DATA "        if (mc > 0.9988) { c = vec3f(235.0, 232.0, 250.0) * (0.85 + 0.15 * vnoise(rd * 400.0)); }"
DATA "        c = c + vec3f(60.0, 60.0, 90.0) * pow(max(mc, 0.0), 300.0);"
DATA "    }"
DATA "    if (sky == 5 || sky == 8) {"
DATA "        let sd = normalize(vec3f(0.25, 0.2, 1.0));"
DATA "        let sc = dot(rd, sd);"
DATA "        if (sc > 0.9975) { c = vec3f(255.0, 238.0, 215.0); }"
DATA "        c = c + vec3f(120.0, 80.0, 50.0) * pow(max(sc, 0.0), 60.0);"
DATA "    }"
DATA "    if (sky == 8 || skyEye) {"
DATA "        // a giant eye in the sky, watching the camera"
DATA "        let ed = normalize(vec3f(-0.4, 0.3, 1.0));"
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
DATA "            c = mix(c, skyGrad(sky, rd), smoothstep(0.92, 1.0, rho));"
DATA "        }"
DATA "    }"
DATA "    let cl = fbm(rd * 2.6 + vec3f(T * 0.008, 0.0, T * 0.004));"
DATA "    let band = 1.0 - smoothstep(0.2, 0.75, abs(h + 0.08));"
DATA "    let cloud = smoothstep(0.42, 0.7, cl) * band;"
DATA "    var cc = vec3f(252.0, 210.0, 242.0);"
DATA "    if (sky == 1) { cc = vec3f(200.0, 150.0, 200.0); }"
DATA "    if (sky == 4) { cc = vec3f(80.0, 72.0, 130.0); }"
DATA "    if (sky == 5) { cc = vec3f(255.0, 190.0, 170.0); }"
DATA "    if (sky == 6 || sky == 7 || sky == 8 || sky == 10) { cc = vec3f(252.0, 252.0, 255.0); }"
DATA "    if (sky == 9) { cc = vec3f(70.0, 40.0, 100.0); }"
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
DATA "        case 36: { s = Surf(vec3f(95.0, 165.0, 90.0) * (0.75 + 0.4 * vnoise(p * 3.0)), 0.0, 0.0); }"
DATA "        case 37: { s = Surf(vec3f(225.0, 205.0, 165.0) * (0.85 + 0.2 * vnoise(p * 5.0)), 0.0, 0.0); }"
DATA "        case 38: {"
DATA "            // fluorescent panel; some flicker"
DATA "            let id = floor(lp.x / 3.0);"
DATA "            var on = 1.0;"
DATA "            if (hs(id * 4.3) > 0.7 && hs(floor(T * 9.0) + id) > 0.6) { on = 0.25; }"
DATA "            s = Surf(vec3f(255.0, 250.0, 225.0) * on, 0.0, 1.0);"
DATA "        }"
DATA "        case 39: { s = Surf(vec3f(185.0, 165.0, 115.0) * (0.85 + 0.2 * vnoise(p * 9.0)), 0.0, 0.0); }"
DATA "        case 41: {"
DATA "            let r = length(p.xz - vec2f(0.0, 7.0));"
DATA "            s = Surf(select(vec3f(120.0, 170.0, 220.0), vec3f(240.0, 200.0, 120.0), fract(r * 1.3) > 0.5), 0.0, 0.0);"
DATA "        }"
DATA "        case 42: { s = Surf(iri(lp.y * 1.5 + lp.z * 0.8) * 230.0, 0.1, 0.0); }"
DATA "        case 45: { s = Surf(vec3f(215.0, 205.0, 225.0), 0.1, 0.0); }"
DATA "        case 46: { s = Surf(tile(uv, 1.6, vec3f(210.0, 215.0, 205.0), vec3f(170.0, 175.0, 165.0)), 0.2, 0.0); }"
DATA "        case 47: { s = Surf(vec3f(175.0, 190.0, 210.0), 0.85, 0.0); }"
DATA "        case 48: {"
DATA "            // a poster that is an eye"
DATA "            let rr = length(lp.yz) / 0.45;"
DATA "            var c = vec3f(235.0, 215.0, 210.0);"
DATA "            if (rr < 0.55) { c = vec3f(110.0, 130.0, 160.0); }"
DATA "            if (rr < 0.25) { c = vec3f(10.0); }"
DATA "            s = Surf(c, 0.1, 0.0);"
DATA "        }"
DATA "        case 49: {"
DATA "            let a = atan2(lp.y, lp.x);"
DATA "            var c = vec3f(240.0);"
DATA "            if (abs(sin(a - T * 0.1)) < 0.04 || abs(sin(a - T * 1.2)) < 0.03) { c = vec3f(20.0); }"
DATA "            s = Surf(c, 0.1, 0.0);"
DATA "        }"
DATA "        default: { }"
DATA "    }"
DATA "    if (mt == 21 || mt == 39 || mt == 32 || mt == 8 || mt == 15 || mt == 12 || mt == 46 || mt == 22) { s.col = s.col * wallTint; }"
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
DATA ""
DATA "// ---- the whole scene: the place, the watcher, the figure ----"
DATA "fn mapd(p: vec3f) -> f32 {"
DATA "    envSDF(S, p);"
DATA "    var d = wd;"
DATA "    var m = wm;"
DATA "    var loc = wl;"
DATA "    var ctr = wc;"
DATA "    if (dissolve > 0.001) {"
DATA "        let hv = hash3(floor(p * 4.0));"
DATA "        d = max(d, (dissolve - hv) * 0.5);"
DATA "    }"
DATA "    if (eyeHere) {"
DATA "        if (figOn) {"
DATA "            let fz = min(cam.z + 10.0, S.L - 2.0);"
DATA "            let fq = p - vec3f(1.15, 0.0, fz);"
DATA "            let fx = vec3f(abs(fq.x), fq.y, fq.z);"
DATA "            var fd = capsule(fq, vec3f(0.0, 0.95, 0.0), vec3f(0.0, 2.5, 0.0), 0.3);"
DATA "            fd = min(fd, capsule(fx, vec3f(0.12, 0.0, 0.0), vec3f(0.12, 1.0, 0.0), 0.1));"
DATA "            fd = min(fd, capsule(fx, vec3f(0.36, 2.4, 0.0), vec3f(0.44, 1.15, -0.05), 0.07));"
DATA "            fd = min(fd, length(fq - vec3f(0.0, 3.0, 0.0)) - 0.26);"
DATA "            if (fd < d) { d = fd; m = 28; }"
DATA "            let fe = length(fx - vec3f(0.09, 3.03, -0.22)) - 0.045;"
DATA "            if (fe < d) { d = fe; m = 27; }"
DATA "        }"
DATA "        let e = p - eyeP;"
DATA "        let rr = length(e);"
DATA "        gEyeD = 100.0;"
DATA "        if (eyeOn) {"
DATA "            gEyeD = rr - eyeR;"
DATA "            if (rr - eyeR * 1.15 < d) {"
DATA "                var db = rr - eyeR;"
DATA "                let c = dot(e, gz) / max(rr, 0.0001);"
DATA "                if (c > 0.78) { db = db - (c - 0.78) * eyeR * 0.45; }"
DATA "                if (db < d) { d = db; m = 1; ctr = eyeP; }"
DATA "            }"
DATA "            if (rr - eyeR * 4.3 < d) {"
DATA "                if (dot(e, gz) < -0.1 * eyeR) {"
DATA "                    let dt = tentacles(e);"
DATA "                    if (dt < d) { d = dt; m = 5; }"
DATA "                }"
DATA "                if (sat1 > 0.0) {"
DATA "                    let s1 = satRing(e, 5.0, eyeR * 1.75, eyeR * 0.35, eyeR * 0.28 * sat1, T * 0.25);"
DATA "                    if (s1.x < d) { d = s1.x; m = 2; ctr = s1.yzw; }"
DATA "                }"
DATA "                if (sat2 > 0.0) {"
DATA "                    let s2 = satRing(e, 9.0, eyeR * 2.6, eyeR * 0.1, eyeR * 0.22 * sat2, -T * 0.18);"
DATA "                    if (s2.x < d) { d = s2.x; m = 2; ctr = s2.yzw; }"
DATA "                }"
DATA "            }"
DATA "        }"
DATA "    }"
DATA "    gMat = m;"
DATA "    gLoc = loc;"
DATA "    gCtr = ctr;"
DATA "    return d;"
DATA "}"
DATA ""
DATA "// where the watcher waits in each kind of place"
DATA "fn peekSpot(s: SC) -> vec3f {"
DATA "    var q = vec3f(3.0, 2.0, s.L * 0.5);"
DATA "    switch s.env {"
DATA "        case 0: { q = vec3f(-7.9, 1.9 + mh(-7.9, 11.8), 11.8); }"
DATA "        case 1: { q = vec3f(-7.5, 3.75, 9.0); }"
DATA "        case 2: { q = vec3f(4.2, 0.05, 16.0); }"
DATA "        case 4: { q = vec3f(-2.7, 1.18, 3.75); }"
DATA "        case 5: { q = vec3f(-5.5, 2.5, 14.0); }"
DATA "        case 7: { q = vec3f(2.8, 1.42, 13.6); }"
DATA "        case 8: { q = vec3f(-10.3, 1.4, 12.0); }"
DATA "        case 9: { q = vec3f(0.0, 4.6, s.L * 0.6); }"
DATA "        default: { }"
DATA "    }"
DATA "    return q;"
DATA "}"
DATA ""
DATA "fn placeEye(s: SC, w: i32, f: vec3f) {"
DATA "    eyeOn = true;"
DATA "    eyeR = 0.7;"
DATA "    skyEye = s.eye == 3;"
DATA "    let side = select(-1.0, 1.0, (w & 1) == 1);"
DATA "    switch s.eye {"
DATA "        case 1: { eyeP = vec3f(side * 2.6, 2.0 + 0.25 * sin(T * 0.7), clamp(cam.z + 7.0, 3.0, s.L - 2.5)); }"
DATA "        case 2: { eyeP = peekSpot(s); eyeR = 0.5; }"
DATA "        case 4: {"
DATA "            eyeP = vec3f(0.0, 1.5 + 0.1 * sin(T * 1.3), min(cam.z + 9.0, s.L - 1.2));"
DATA "            eyeR = 0.42;"
DATA "            eyeOn = cam.z < s.L - 7.5;"
DATA "        }"
DATA "        case 5: { eyeP = cam + vec3f(0.0, 0.2, -4.2); eyeR = 0.8; }"
DATA "        default: { eyeOn = false; eyeP = cam + f * 12.0; }"
DATA "    }"
DATA "    if (act >= 3) { eyeR = eyeR * 1.15; }"
DATA "    // the lunge: wherever it was, it comes at the camera"
DATA "    if (evT == 5) {"
DATA "        let k = clamp(1.0 - abs(evP - 0.3) / 0.14, 0.0, 1.0);"
DATA "        if (k > 0.0) {"
DATA "            eyeP = mix(eyeP, cam + f * 1.7, k);"
DATA "            eyeOn = true;"
DATA "        }"
DATA "    }"
DATA "}"
DATA ""
DATA "// fog, light and wall colours of a scene"
DATA "fn look(s: SC) {"
DATA "    fogC = fogOf(s.sky);"
DATA "    fogD = 0.012;"
DATA "    switch s.env {"
DATA "        case 2: { fogD = 0.016; }"
DATA "        case 3: { fogD = 0.03; fogC = vec3f(200.0, 185.0, 120.0); }"
DATA "        case 4, 7: { fogD = 0.02; }"
DATA "        case 5: { fogD = 0.014; }"
DATA "        case 8: { fogD = 0.02; }"
DATA "        case 10: { fogD = 0.015; }"
DATA "        default: { }"
DATA "    }"
DATA "    indoor = s.env == 2 || s.env == 3 || s.env == 4 || s.env == 5 || s.env == 7;"
DATA "    sunDir = normalize(vec3f(-0.45, 0.75, 0.5));"
DATA "    sunCol = vec3f(1.0, 0.93, 0.97) * 0.62;"
DATA "    ambK = 0.6;"
DATA "    switch s.sky {"
DATA "        case 10: { sunDir = normalize(vec3f(-0.4, 0.8, 0.45)); sunCol = vec3f(1.0, 0.97, 0.9) * 0.75; ambK = 0.62; }"
DATA "        case 4: { sunDir = normalize(vec3f(0.35, 0.6, 0.7)); sunCol = vec3f(0.45, 0.5, 0.8) * 0.35; ambK = 0.28; }"
DATA "        case 5: { sunDir = normalize(vec3f(0.25, 0.22, 1.0)); sunCol = vec3f(1.0, 0.7, 0.55) * 0.8; ambK = 0.5; }"
DATA "        case 6: { sunDir = normalize(vec3f(-0.3, 0.7, 0.6)); sunCol = vec3f(0.9, 1.0, 1.0) * 0.6; ambK = 0.62; }"
DATA "        case 1: { sunDir = normalize(vec3f(0.3, 0.45, 0.8)); sunCol = vec3f(0.8, 0.6, 0.9) * 0.45; ambK = 0.45; }"
DATA "        case 9: { sunDir = normalize(vec3f(0.3, 0.6, 0.5)); sunCol = vec3f(0.5, 0.4, 0.8) * 0.3; ambK = 0.3; }"
DATA "        case 2: { sunDir = normalize(vec3f(0.2, 0.5, 0.8)); sunCol = vec3f(0.9, 0.25, 0.25) * 0.45; ambK = 0.3; }"
DATA "        case 8: { sunDir = normalize(vec3f(0.25, 0.25, 1.0)); sunCol = vec3f(1.0, 0.85, 0.7) * 0.7; ambK = 0.55; }"
DATA "        case 3: { ambK = 1.0; }"
DATA "        default: { }"
DATA "    }"
DATA "    if (indoor) { ambK = max(ambK, 0.5); }"
DATA "    if (s.env == 3) { ambK = select(0.75, 0.3, s.v == 3); }"
DATA "    wallTint = vec3f(1.0);"
DATA "    switch s.env {"
DATA "        case 2: {"
DATA "            if (s.v == 2) { wallTint = vec3f(1.15, 0.82, 0.95); }"
DATA "            if (s.v == 3) { wallTint = vec3f(0.6, 0.75, 0.8); }"
DATA "            if (s.v == 4) { wallTint = vec3f(1.1, 0.55, 0.55); }"
DATA "        }"
DATA "        case 3: {"
DATA "            if (s.v == 1) { wallTint = vec3f(1.15, 0.8, 0.95); }"
DATA "            if (s.v == 2) { wallTint = vec3f(0.95, 1.08, 1.1); }"
DATA "            if (s.v == 3) { wallTint = vec3f(0.55, 0.52, 0.45); }"
DATA "            if (s.v == 4) { wallTint = vec3f(1.15, 0.5, 0.45); }"
DATA "        }"
DATA "        case 4: {"
DATA "            wallTint = vec3f(0.85, 0.95, 1.15);"
DATA "            if (s.v == 1 || s.v == 3) { wallTint = vec3f(0.45, 0.5, 0.75); }"
DATA "            if (s.v == 2) { wallTint = vec3f(0.9, 0.7, 1.1); }"
DATA "        }"
DATA "        case 7: { if (s.v == 2) { wallTint = vec3f(0.5, 0.55, 0.7); } }"
DATA "        default: { }"
DATA "    }"
DATA "}"
DATA ""
DATA "// ---- ray tracing: primary ray (through the doorway into the next scene),"
DATA "// a soft shadow ray toward the sun, AO and one reflection bounce ----"
DATA "fn march(ro: vec3f, rd: vec3f, maxs: i32, wantGlow: bool, maxT: f32) -> vec2f {"
DATA "    var t = 0.04;"
DATA "    for (var i = 0; i < 300; i++) {"
DATA "        if (i >= maxs) { break; }"
DATA "        let d = mapd(ro + rd * t);"
DATA "        if (wantGlow) { glowAcc = glowAcc + min(d, 0.5) / (1.0 + gEyeD * gEyeD * 4.0); }"
DATA "        if (d < 0.0008 * t + 0.001) { return vec2f(t, 1.0); }"
DATA "        t = t + d * 0.92 + 0.0015 * t;"
DATA "        if (t > maxT) { return vec2f(maxT, 0.0); }"
DATA "    }"
DATA "    return vec2f(t, 2.0);"
DATA "}"
DATA ""
DATA "struct Hit { col: vec3f, refl: f32, n: vec3f };"
DATA ""
DATA "fn lightHit(p: vec3f, rd: vec3f, t: f32, primary: bool) -> Hit {"
DATA "    let mt = gMat;"
DATA "    let lp = gLoc;"
DATA "    let ctr = gCtr;"
DATA "    let ee = vec2f(1.0, -1.0) * (0.0015 + 0.0006 * t);"
DATA "    let n = normalize(ee.xyy * mapd(p + ee.xyy) + ee.yyx * mapd(p + ee.yyx) + ee.yxy * mapd(p + ee.yxy) + ee.xxx * mapd(p + ee.xxx));"
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
DATA "        var sh = 1.0;"
DATA "        var ao = 1.0;"
DATA "        let hall = S.env == 3;"
DATA "        if (primary && !hall && t < 40.0) { sh = softShadow(p + nn * 0.02, sunDir, 20.0); }"
DATA "        if (primary && t < 30.0) { ao = ambOcc(p, nn); }"
DATA "        if (s.emis < 0.5) {"
DATA "            var dif = max(dot(nn, sunDir), 0.0);"
DATA "            if (hall) { dif = 0.35 + 0.35 * max(nn.y, 0.0); }"
DATA "            let hemi = mix(vec3f(0.62, 0.55, 0.66), vec3f(1.05, 0.97, 1.1), 0.5 + 0.5 * nn.y);"
DATA "            col = col * (hemi * ambK * ao + sunCol * dif * sh);"
DATA "        } else {"
DATA "            col = col * (0.5 + 0.5 * ambK + 0.25 * sh) * (0.8 + 0.2 * ao);"
DATA "        }"
DATA "        if (s.gloss > 0.0) {"
DATA "            let r = reflect(rd, nn);"
DATA "            col = col + vec3f(255.0) * pow(max(dot(r, sunDir), 0.0), 48.0) * s.gloss * sh * length(sunCol);"
DATA "            let fres = pow(1.0 - max(dot(-rd, nn), 0.0), 3.0);"
DATA "            refl = clamp(s.gloss * (0.25 + 0.65 * fres), 0.0, 0.9);"
DATA "        }"
DATA "    }"
DATA "    if (!isEye && mt != 27) { col = col * (1.0 - 0.93 * dark); }"
DATA "    return Hit(col, refl, nn);"
DATA "}"
DATA ""
DATA "fn skyMix(rd: vec3f) -> vec3f {"
DATA "    let s = skyCol(S.sky, rd);"
DATA "    return mix(s, fogC, 0.7 * exp(-abs(rd.y) * 5.0)) * (1.0 - 0.95 * dark);"
DATA "}"
DATA ""
DATA "fn hazeCol(rd: vec3f) -> vec3f {"
DATA "    let s = skyGrad(S.sky, rd);"
DATA "    return mix(s, fogC, 0.7 * exp(-abs(rd.y) * 5.0)) * (1.0 - 0.95 * dark);"
DATA "}"
DATA ""
DATA "fn render(ro: vec3f, rd: vec3f) -> vec3f {"
DATA "    glowAcc = 0.0;"
DATA "    S = SA;"
DATA "    eyeHere = true;"
DATA "    look(SA);"
DATA "    let maxs = i32(qual);"
DATA "    // does this ray leave through the exit doorway?"
DATA "    var tp = 1000000.0;"
DATA "    if (rd.z > 0.0001 && ro.z < SA.L) {"
DATA "        let t0 = (SA.L - ro.z) / rd.z;"
DATA "        let hp = ro + rd * t0;"
DATA "        if (abs(hp.x) < DOOR_W && hp.y > 0.0 && hp.y < DOOR_H) { tp = t0; }"
DATA "    }"
DATA "    var o = ro;"
DATA "    var tOff = 0.0;"
DATA "    var m = march(o, rd, maxs, true, min(tp, FAR));"
DATA "    if (m.y < 0.5 && tp < FAR) {"
DATA "        // through the doorway: the rest of the ray is in the next scene"
DATA "        o = ro + rd * tp - vec3f(0.0, 0.0, SA.L);"
DATA "        tOff = tp;"
DATA "        S = SB;"
DATA "        eyeHere = false;"
DATA "        look(SB);"
DATA "        m = march(o, rd, maxs, false, FAR - tp);"
DATA "    }"
DATA "    var col = vec3f(0.0);"
DATA "    var thr = 1.0;"
DATA "    var d = rd;"
DATA "    for (var b = 0; b < 2; b++) {"
DATA "        if (b == 1) { m = march(o, d, maxs / 4, false, 45.0); }"
DATA "        let tt = m.x + select(0.0, tOff, b == 0);"
DATA "        if (m.y < 0.5) {"
DATA "            col = col + thr * skyMix(d);"
DATA "            break;"
DATA "        }"
DATA "        if (m.y > 1.5) {"
DATA "            col = col + thr * mix(hazeCol(d), skyMix(d), smoothstep(0.0, FAR, tt));"
DATA "            break;"
DATA "        }"
DATA "        let p = o + d * m.x;"
DATA "        let h = lightHit(p, d, tt, b == 0);"
DATA "        let fogk = max(1.0 - exp(-tt * fogD), smoothstep(FAR * 0.45, FAR, tt));"
DATA "        let hz = hazeCol(d);"
DATA "        let base = mix(h.col, hz, fogk);"
DATA "        if (b == 1 || h.refl < 0.02 || tt > 45.0) {"
DATA "            col = col + thr * base;"
DATA "            break;"
DATA "        }"
DATA "        if (h.refl < 0.3) {"
DATA "            col = col + thr * mix(base, skyMix(reflect(d, h.n)), h.refl * (1.0 - fogk));"
DATA "            break;"
DATA "        }"
DATA "        col = col + thr * ((1.0 - h.refl) * base + h.refl * fogk * hz);"
DATA "        thr = thr * h.refl * (1.0 - fogk);"
DATA "        o = p + h.n * 0.03;"
DATA "        d = reflect(d, h.n);"
DATA "    }"
DATA "    col = col + glowAcc * (20.0 + 200.0 * venv) * vec3f(1.0, 0.35 + 0.4 * (1.0 - hor), 0.6 * (1.0 - hor) + 0.15);"
DATA "    return col;"
DATA "}"
DATA ""
DATA "fn shade(i: ShadeIn) -> vec4<f32> {"
DATA "    cam = i.params[0].xyz;"
DATA "    T = i.params[0].w;"
DATA "    let wA = i32(i.params[1].x + 0.5);"
DATA "    let wB = i32(i.params[1].y + 0.5);"
DATA "    let roll = i.params[2].x;"
DATA "    venv = i.params[2].y;"
DATA "    let ev = i.params[2].z;"
DATA "    gli = i.params[2].w;"
DATA "    let yaw = i.params[3].x;"
DATA "    let pitch = i.params[3].y;"
DATA "    stat = i.params[3].z;"
DATA "    qual = i.params[3].w;"
DATA "    hor = clamp(T / 900.0, 0.0, 1.0);"
DATA "    mlt = 0.0;"
DATA "    act = i32(floor(f32(wA) / 10.0 + 0.001));"
DATA "    pup = clamp(venv * 1.2 + 0.08 * sin(T * 0.9), 0.0, 1.3);"
DATA "    SA = scene(wA);"
DATA "    SB = scene(wB);"
DATA "    sat1 = 0.0;"
DATA "    sat2 = 0.0;"
DATA "    if (T >= 630.0 && T < 900.0) { sat1 = clamp((T - 630.0) / 12.0, 0.0, 1.0); }"
DATA "    if (T >= 720.0 && T < 900.0) { sat2 = clamp((T - 720.0) / 12.0, 0.0, 1.0); }"
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
DATA "    dissolve = 0.0;"
DATA "    if (wA == 48) { dissolve = smoothstep(870.0, 880.0, T) * 0.85; }"
DATA ""
DATA "    // the camera: position from the BASIC code, looking along yaw and pitch"
DATA "    let f = vec3f(cos(pitch) * sin(yaw), sin(pitch), cos(pitch) * cos(yaw));"
DATA "    let r0 = normalize(vec3f(f.z, 0.0, -f.x));"
DATA "    let u0 = cross(f, r0);"
DATA "    placeEye(SA, wA, f);"
DATA "    gz = normalize(cam - eyeP);"
DATA "    tb1 = normalize(cross(gz, vec3f(0.0, 1.0, 0.0)));"
DATA "    tb2 = cross(gz, tb1);"
DATA ""
DATA "    let rd0 = normalize(i.world - cam);"
DATA "    let uv0 = vec2f(dot(rd0, r0), dot(rd0, u0)) / max(dot(rd0, f), 0.05);"
DATA "    let rowpx = uv0.y / max(abs(dpdy(uv0.y)), 0.000001);"
DATA "    let tanH = 0.5773503;"
DATA "    let aspect = 1.7777778;"
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
DATA "    // dreamy grade: lifted shadows, soft highlights, a little pink in the light"
DATA "    col = col / (1.0 + col / 520.0) * 1.25;"
DATA "    col = col * 0.9 + vec3f(16.0, 10.0, 18.0);"
DATA "    col = col * (1.0 - 0.28 * dot(sn * sn, vec2f(0.7, 1.0)));"
DATA "    col = col * smoothstep(0.0, 0.045, -bd);"
DATA "    if (fract(rowpx / 3.0) < 0.34) { col = col * 0.92; }"
DATA "    if (stat > 0.0) {"
DATA "        let h = hs(dot(floor(uv * 420.0), vec2f(12.9898, 78.233)) + floor(T * 30.0) * 3.7);"
DATA "        if (h < stat) { col = vec3f(hs(h * 91.0) * 230.0); }"
DATA "    }"
DATA "    col = col + (hs(dot(uv, vec2f(127.1, 311.7)) * 50.0 + fract(T) * 13.0) - 0.5) * 9.0;"
DATA "    if (hor > 0.78) { col = mix(col, vec3f(160.0, 0.0, 20.0), venv * 0.15); }"
DATA "    if (T < 3.0) { col = col * (T / 3.0); }"
DATA "    if (T > 899.0) { col = mix(col, vec3f(255.0), clamp((T - 899.0) / 6.0, 0.0, 1.0)); }"
DATA "    if (evT == 5) {"
DATA "        col = mix(col, vec3f(200.0, 0.0, 20.0), clamp(1.0 - abs(evP - 0.3) / 0.12, 0.0, 1.0) * 0.5);"
DATA "        if (evP > 0.27 && evP < 0.3) { col = vec3f(255.0) - col; }"
DATA "    }"
DATA "    if (evT == 3 && evP > 0.84 && evP < 0.87) { col = vec3f(255.0) - col; }"
DATA "    return vec4<f32>(clamp(col / 255.0, vec3f(0.0), vec3f(1.0)), 1.0);"
DATA "}"
DATA "~"

' ============================================================================
'  setup
' ============================================================================
SUB InitData
    RESTORE Scenes
    FOR i = 0 TO 49: READ scEnv(i), scCam(i), evList(i): NEXT
    RESTORE Melody
    FOR i = 0 TO 63: READ melody(i): NEXT
    RESTORE Harmony
    FOR i = 0 TO 7: FOR j = 0 TO 3: READ chordN(i, j): NEXT: NEXT
    RESTORE VoiceLines
    FOR k = 0 TO 50
        READ en$, ph$
        enL$(k) = en$: phL$(k) = ph$
        lineAt(k) = k * WLEN + 2.5
    NEXT
    lineAt(50) = 900.6
    subLine = -1: nextLine = 0: glLine = -1
END SUB

SUB InitAudio
    DIM i AS LONG, o AS LONG
    sr = _SNDRATE
    IF sr <= 0 THEN
        sr = 44100: audioOn = 0
    ELSE
        audioOn = 1
    END IF
    genS = 0
    cF1 = 500: cF2 = 1500: cF3 = 2500: curPit = 1: cNF = 3000: cAmp = 0: cNz = 0
    rmS = 0: rmC = 1
    segN = 0: segIdx = 0: segPos = 0
    FOR i = 0 TO 4095: sinTab(i) = SIN(i * 2 * PI / 4096): NEXT
    FOR i = 0 TO 7: pInc(i) = 0: pPh(i) = i * .13: pLp(i) = 0: NEXT
    stingW = -1: lastStep = -1
    ' reverb: 4 combs and 2 all-pass filters per channel (Freeverb lengths)
    o = 0
    FOR i = 0 TO 11
        SELECT CASE i MOD 6
            CASE 0: n = 1116
            CASE 1: n = 1188
            CASE 2: n = 1277
            CASE 3: n = 1356
            CASE 4: n = 556
            CASE ELSE: n = 441
        END SELECT
        IF i >= 6 THEN n = n + 23
        n = INT(n * sr / 44100)
        IF n > 2040 THEN n = 2040
        rvOff(i) = o: rvLen(i) = n: rvIdx(i) = 0
        o = o + 2048
    NEXT
    FOR i = 0 TO 7: rvLp(i) = 0: NEXT
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

' the length of a place, from its entry doorway to its exit (as the shader)
FUNCTION EnvLen (env)
    SELECT CASE env
        CASE 0, 3, 6, 8: EnvLen = 30
        CASE 4: EnvLen = 14
        CASE 7: EnvLen = 16
        CASE ELSE: EnvLen = 28
    END SELECT
END FUNCTION

' ============================================================================
'  per frame: which scene, the camera shot, events
' ============================================================================
' the camera through scene w at progress u (0 at the entry doorway, 1 at the
' exit doorway); the first tenth eases in from the doorway and the last
' quarter lines up with the exit, so one scene's end is the next one's start
SUB CamPose (w, u, el)
    env = scEnv(w): ct = scCam(w): L = EnvLen(env)
    side = 1
    IF (w AND 1) = 0 THEN side = -1
    s = u * .85 + u * u * (3 - 2 * u) * .15
    x = 0: y = 1.55: yaw = 0: pitch = -.03: roll = 0
    low = 0
    IF env = 3 OR env = 4 OR env = 7 THEN low = 1
    SELECT CASE ct
        CASE 0
            y = 1.55 + .035 * SIN(L * s * 4.2)
            x = .3 * SIN(u * 5.3)
            yaw = .22 * SIN(u * 6.1 + 1): pitch = -.03 + .03 * SIN(u * 3.7)
            roll = .012 * SIN(L * s * 2.1)
        CASE 1
            amp = .9: lift = 1.5
            IF env = 6 THEN amp = .45: lift = 1.1
            y = 1.55 + lift * SIN(u * PI)
            x = amp * SIN(u * PI * 1.5) * side
            yaw = -.2 * COS(u * PI * 1.5) * side: pitch = -.1 * SIN(u * PI)
            roll = .05 * SIN(u * PI * 1.5) * side
        CASE 2
            x = 2.5 * SIN(u * PI) * side
            tz = L * .55
            yaw = Atan2(-x, Mx(tz - L * s, .5)) * (1 - Sm(.45, .62, u))
            y = 1.6
        CASE 3
            k = 1 - Sm(0, .55, u)
            hgt = 5
            IF low THEN hgt = 1
            y = 1.55 + hgt * k: pitch = -.55 * k * (1 - low * .6) - .03
        CASE 4
            s = u + .035 * SIN(u * PI * 6)
            y = 1.45 + .02 * SIN(el * 1.7)
            x = .15 * SIN(u * 4)
            yaw = .45 * SIN(u * 4.2) * Sm(.1, .3, u)
            pitch = -.05
            roll = .02 * SIN(el * 2.3) + .01 * SIN(el * 5.1)
        CASE ELSE
            s = Sm(.33, 1, u)
            pitch = .06 * (1 - Sm(.3, .5, u))
    END SELECT
    bi = 1 - Sm(0, .12, u)
    bo = Sm(.72, .97, u)
    keep = (1 - bi) * (1 - bo)
    camX = x * keep
    camY = y + (1.4 - y) * Mx(bi, bo)
    camZ = L * s
    camYaw = yaw * keep: camPitch = pitch * keep: camRoll = roll * keep
END SUB

SUB UpdateWorld (el)
    DIM pb AS LONG
    tNow = el
    w = INT(el / WLEN)
    IF w > 49 THEN w = 49
    IF w < 0 THEN w = 0
    u = Cl((el - w * WLEN) / WLEN, 0, 1)
    wCur = w
    wNext = w + 1
    IF wNext > 49 THEN wNext = 49
    act = INT(w / 10)
    horror = Cl(el / 900, 0, 1)

    ' glitches and CRT static only in the last act
    glitchAmt = 0
    IF el >= 720 THEN glitchAmt = .1 + .4 * (el - 720) / 180
    IF RND < .01 + .03 * horror THEN glitchAmt = glitchAmt + .3 * horror
    IF el > 895 THEN glitchAmt = glitchAmt * Cl((900 - el) / 5, 0, 1)
    staticAmt = 0
    IF el >= 720 THEN staticAmt = .004 + .02 * (el - 720) / 180
    IF el > 895 THEN staticAmt = 0

    ' what the voice is saying right now drives the pupil and the glow
    pb = INT(CDBL(el) * sr / BLK)
    en = envRing(pb AND 4095)
    voiceEnv = voiceEnv + (en - voiceEnv) * .5

    CamPose w, u, el

    ' scare events (the shader reads the same type and phase)
    EventAt el, evType, evPh
    IF evType = 3 THEN camYaw = camYaw + PI * Sm(.05, .22, evPh) * (1 - Sm(.7, .85, evPh))
    kick = 0
    IF evType = 5 THEN kick = .2 * Cl(1 - ABS(evPh - .3) / .15, 0, 1)
    IF evType = 3 THEN kick = .15 * Cl(1 - ABS(evPh - .86) / .06, 0, 1)
    camX = camX + (RND - .5) * kick: camY = camY + (RND - .5) * kick
    camRoll = camRoll + (RND - .5) * kick * .5 + (RND - .5) * glitchAmt * .03

    ' camera basis without roll, for the overlay quad
    cfX = COS(camPitch) * SIN(camYaw): cfY = SIN(camPitch): cfZ = COS(camPitch) * COS(camYaw)
    l = SQR(cfZ * cfZ + cfX * cfX) + .000001
    rbX = cfZ / l: rbZ = -cfX / l
    ubX = cfY * rbZ
    ubY = cfZ * rbX - cfX * rbZ
    ubZ = -cfY * rbX
END SUB

' ============================================================================
'  overlay, voice, audio, GPU
' ============================================================================
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

FUNCTION NoteInc (n)
    NoteInc = 440 * 2 ^ ((n - 69) / 12) / sr
END FUNCTION

' the same tune in a minor key: lower the third, sixth and seventh
FUNCTION MinorOf (n)
    pc = n - 12 * INT(n / 12)
    IF pc = 4 OR pc = 9 OR pc = 11 THEN MinorOf = n - 1 ELSE MinorOf = n
END FUNCTION

FUNCTION SinT (p)
    SinT = sinTab(INT(p * 4096) AND 4095)
END FUNCTION

SUB GenBlock
    DIM i AS LONG, j AS LONG, c AS LONG, k AS LONG, stp AS LONG
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
    cF1 = cF1 + (tF1 - cF1) * .3
    cF2 = cF2 + (tF2 - cF2) * .3
    cF3 = cF3 + (tF3 - cF3) * .3
    cAmp = cAmp + (tAmp - cAmp) * .4
    cNz = cNz + (tNz - cNz) * .6
    curPit = curPit + (tP - curPit) * .4
    cNF = tNF
    f0 = (112 - 26 * horror) * curPit
    st = INT(12 * LOG(f0 / 55) / LOG(2) + .5)
    f0 = 55 * 2 ^ (st / 12)
    pinc = f0 / sr
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
    wh = .05 + .3 * horror
    vAmp = cAmp * (1 - cNz) * (1 - wh * .5)
    wAmp = cAmp * (1 - cNz) * wh * .6
    nAmp = cAmp * cNz
    rmDepth = .05 + .3 * horror
    rmK = 2 * PI * (30 + 50 * horror) / sr
    envA = 0

    ' ---- events, heard at the moment they are seen ----
    EventAt bt, ety, eph
    mute = 1
    IF ety = 2 THEN mute = 1 - .92 * Sm(.03, .08, eph) * (1 - Sm(.8, .9, eph))
    IF (ety = 5 AND eph >= .27) OR (ety = 3 AND eph >= .84) THEN
        IF stingW <> INT(bt / WLEN) THEN stingW = INT(bt / WLEN): stE = 1: stF = 900
    END IF
    figV = 0
    IF ety = 4 AND eph > .08 AND eph < .92 THEN figV = .03
    rumV = 0
    IF ety = 2 THEN rumV = .25 * (1 - mute)

    ' ---- the sound of the place ----
    wb = INT(bt / WLEN)
    IF wb > 49 THEN wb = 49
    IF wb < 0 THEN wb = 0
    env = scEnv(wb)
    tW = 0: tWa = 0: tH = 0: tT = 0: tC = 0: tD = 0: tHs = 0: cut = .35
    SELECT CASE env
        CASE 0
            tW = 1
            IF wb = 25 OR wb = 9 THEN tC = 1
            IF wb = 43 THEN tD = 1
        CASE 1
            tW = .6
            IF wb = 29 THEN tC = 1
        CASE 2: tWa = 1: tH = .25: cut = .2
        CASE 3: tH = 1: cut = .06
        CASE 4: tT = 1: tHs = .6: cut = .12
        CASE 5: tH = .4: tWa = .4: cut = .15
        CASE 6: tW = .8
        CASE 7: tT = .8: tH = .5: cut = .1
        CASE 8: tC = 1: tW = .3
        CASE 9
            tWa = .5
            IF wb >= 31 THEN tD = 1
        CASE ELSE: tD = 1
    END SELECT
    gWind = gWind + (tW - gWind) * .01: gWater = gWater + (tWa - gWater) * .01
    gHum = gHum + (tH - gHum) * .01: gTick = gTick + (tT - gTick) * .01
    gCrick = gCrick + (tC - gCrick) * .01: gDrone = gDrone + (tD - gDrone) * .01
    gHiss = gHiss + (tHs - gHiss) * .01: mCut = mCut + (cut - mCut) * .01
    windMod = .55 + .45 * SIN(bt * .13) * SIN(bt * .31)
    lapMod = .5 + .5 * SIN(bt * 1.4)
    crGate = 0
    IF (bt * 15 - INT(bt * 15)) < .5 AND (bt * .8 - INT(bt * .8)) < .55 THEN crGate = 1
    IF INT(bt) <> INT(bt - BLK / sr) THEN tickE = 1

    ' ---- the lullaby: slower, darker and more broken in each act ----
    am = INT(bt / 180)
    IF am > 4 THEN am = 4
    fin = 0
    IF bt > 896 THEN am = 0: fin = 1
    bpm = 72 - am * 6
    musPos = musPos + BLK / sr * bpm / 30
    stp = INT(musPos)
    IF stp <> lastStep THEN
        lastStep = stp
        n = melody(stp MOD 64)
        pl = 0
        IF n > 0 THEN pl = 1
        IF am = 3 AND RND < .3 THEN pl = 0
        IF am >= 4 THEN
            IF (stp MOD 4) <> 0 THEN pl = 0
            n = n - 12
        END IF
        IF pl THEN
            IF am >= 2 THEN n = MinorOf(n)
            mbI = NoteInc(n)
            IF am = 3 THEN mbI = mbI * (1 + (RND - .5) * .04)
            mbE = .9: mbP = 0: mbP2 = 0
        END IF
        IF am >= 4 AND fin = 0 THEN
            IF (stp AND 1) = 0 THEN
                hbE = 1: hbI = 70 / sr
            ELSE
                hbE = .5: hbI = 58 / sr
            END IF
        END IF
    END IF
    bar = (lastStep \ 8) MOD 8
    FOR j = 0 TO 3
        n = chordN(bar, j)
        IF am >= 2 THEN n = MinorOf(n)
        ti = NoteInc(n - 12)
        pInc(j) = pInc(j) + (ti - pInc(j)) * .03
        pInc(j + 4) = pInc(j + 4) + (ti * 1.0045 - pInc(j + 4)) * .03
    NEXT
    nb = chordN(bar, 0)
    IF am >= 2 THEN nb = MinorOf(nb)
    bsI = bsI + (NoteInc(nb - 24) - bsI) * .03
    wob = 1 + (.0025 + .003 * am) * SIN(bt * 3.46) + .0007 * SIN(bt * 44)
    padV = .03: mbV = .15: bsV = .07
    IF am >= 4 THEN padV = .022: mbV = .1
    IF fin THEN padV = .05
    musG = (1 - .45 * Cl(cAmp, 0, 1)) * mute
    IF bt > 900 THEN musG = musG * Cl((909 - bt) / 9, 0, 1)

    FOR k = 1 TO BLK
        ' voice
        ph = ph + pinc
        IF ph >= 1 THEN ph = ph - 1
        src = ph + ph - 1
        nz = RND + RND - 1
        ex = src * vAmp + nz * wAmp
        o1 = a1 * ex + b11 * y11 + b12 * y12: y12 = y11: y11 = o1
        o2 = a2 * ex + b21 * y21 + b22 * y22: y22 = y21: y21 = o2
        o3 = a3 * ex + b31 * y31 + b32 * y32: y32 = y31: y31 = o3
        o4 = an * nz * nAmp + bn1 * yn1 + bn2 * yn2: yn2 = yn1: yn1 = o4
        v = (o1 + .7 * o2 + .35 * o3) * 2.2 + o4 * 1.4
        rmS = rmS + rmK * rmC: rmC = rmC - rmK * rmS
        v = v * (1 - rmDepth + rmDepth * rmS) * .75
        envA = envA + ABS(v)

        ' music: warm pads (two detuned saws per note, softened), bass,
        ' music box with an out-of-tune partial, heartbeat
        pad = 0
        FOR j = 0 TO 7
            pPh(j) = pPh(j) + pInc(j) * wob
            IF pPh(j) >= 1 THEN pPh(j) = pPh(j) - 1
            pLp(j) = pLp(j) + (pPh(j) + pPh(j) - 1 - pLp(j)) * .06
            pad = pad + pLp(j)
        NEXT
        bsP = bsP + bsI * wob
        IF bsP >= 1 THEN bsP = bsP - 1
        mbP = mbP + mbI * wob
        IF mbP >= 1 THEN mbP = mbP - 1
        mbP2 = mbP2 + mbI * 2.76 * wob
        IF mbP2 >= 1 THEN mbP2 = mbP2 - 1
        mb = (SinT(mbP) + .3 * SinT(mbP2) * mbE) * mbE
        mbE = mbE * .99994
        hbP = hbP + hbI
        IF hbP >= 1 THEN hbP = hbP - 1
        hb = SinT(hbP) * hbE
        hbE = hbE * .99965: hbI = hbI * .99997
        mus = (pad * padV + SinT(bsP) * bsV + mb * mbV + hb * .45) * musG
        ' heard from another room: a low-pass that depends on the place
        mLpL = mLpL + (mus - mLpL) * mCut

        ' the place
        wLp = wLp + ((RND - .5) - wLp) * .02
        wLp2 = wLp2 + (wLp - wLp2) * .05
        amb = wLp2 * gWind * 2.2 * windMod
        amb = amb + wLp * gWater * 1.2 * lapMod
        drp = 0
        IF gWater > .1 THEN
            IF RND < gWater / (sr * 1.3) THEN drpE = .5: drpI = (1300 + RND * 1300) / sr: drpP = 0
        END IF
        IF drpE > .001 THEN
            drpP = drpP + drpI
            IF drpP >= 1 THEN drpP = drpP - 1
            drp = SinT(drpP) * drpE
            drpE = drpE * .9992: drpI = drpI * 1.00003
        END IF
        hum1 = hum1 + 120 / sr
        IF hum1 >= 1 THEN hum1 = hum1 - 1
        amb = amb + (SinT(hum1) + .5 * SinT(hum1 * 2) + .3 * SinT(hum1 * 3)) * gHum * .018
        IF tickE > .001 THEN amb = amb + (RND - .5) * tickE * gTick * .35: tickE = tickE * .993
        crP = crP + 4400 / sr
        IF crP >= 1 THEN crP = crP - 1
        amb = amb + SinT(crP) * crGate * gCrick * .02
        drP1 = drP1 + 55 / sr
        IF drP1 >= 1 THEN drP1 = drP1 - 1
        drP2 = drP2 + 82.7 / sr
        IF drP2 >= 1 THEN drP2 = drP2 - 1
        amb = amb + (SinT(drP1) + .7 * SinT(drP2)) * gDrone * .05
        amb = amb + (RND - .5) * gHiss * .02
        amb = amb * mute

        ' events
        sg = 0
        IF stE > .001 THEN
            stP = stP + stF / sr
            IF stP >= 1 THEN stP = stP - 1
            stF = stF * .99993
            sg = (SinT(stP) * .6 + (RND - .5) * 1.1) * stE * .8
            stE = stE * .99986
        END IF
        IF figV > 0 THEN
            fgP = fgP + 1830 / sr * (1 + .01 * SIN(bt * 9))
            IF fgP >= 1 THEN fgP = fgP - 1
            sg = sg + SinT(fgP) * figV
        END IF
        IF rumV > 0 THEN
            rmb = rmb + ((RND - .5) - rmb) * .01
            sg = sg + rmb * rumV * 6
        END IF

        ' tape: hiss and the odd crackle
        tape = (RND - .5) * .006
        IF RND < 2 / sr THEN crk = (RND - .5) * .5
        tape = tape + crk: crk = crk * .7

        ' a long reverb on the music, the voice and the drips
        rin = (mLpL * .7 + v * .5 + drp * .6) * .02
        revL = 0: revR = 0
        FOR c = 0 TO 3
            i = rvOff(c) + rvIdx(c)
            y = rvBuf(i)
            rvLp(c) = y * .65 + rvLp(c) * .35
            rvBuf(i) = rin + rvLp(c) * .85
            rvIdx(c) = rvIdx(c) + 1
            IF rvIdx(c) >= rvLen(c) THEN rvIdx(c) = 0
            revL = revL + y
            i = rvOff(c + 6) + rvIdx(c + 6)
            y = rvBuf(i)
            rvLp(c + 4) = y * .65 + rvLp(c + 4) * .35
            rvBuf(i) = rin + rvLp(c + 4) * .85
            rvIdx(c + 6) = rvIdx(c + 6) + 1
            IF rvIdx(c + 6) >= rvLen(c + 6) THEN rvIdx(c + 6) = 0
            revR = revR + y
        NEXT
        FOR c = 4 TO 5
            i = rvOff(c) + rvIdx(c)
            bb = rvBuf(i): rvBuf(i) = revL + bb * .5: revL = bb - revL
            rvIdx(c) = rvIdx(c) + 1
            IF rvIdx(c) >= rvLen(c) THEN rvIdx(c) = 0
            i = rvOff(c + 6) + rvIdx(c + 6)
            bb = rvBuf(i): rvBuf(i) = revR + bb * .5: revR = bb - revR
            rvIdx(c + 6) = rvIdx(c + 6) + 1
            IF rvIdx(c + 6) >= rvLen(c + 6) THEN rvIdx(c + 6) = 0
        NEXT

        lo = mLpL + revL * 1.4 + v * .8 + amb + drp * .3 + sg + tape
        ro = mLpL + revR * 1.4 + v * .8 + amb + drp * .3 + sg + tape
        ' soft limiter
        lo = Cl(lo, -3, 3): ro = Cl(ro, -3, 3)
        lo = lo * (27 + lo * lo) / (27 + 9 * lo * lo)
        ro = ro * (27 + ro * ro) / (27 + 9 * ro * ro)
        IF audioOn THEN _SNDRAW lo * .9, ro * .9
    NEXT
    ev = envA / BLK * 5
    IF ev > 1 THEN ev = 1
    envRing((genS \ BLK) AND 4095) = ev
    genS = genS + BLK
END SUB
