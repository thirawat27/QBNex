' Stackfall: a full-screen falling-block puzzle with live-synthesised music
' and sound effects.
'   Left/Right move, Down soft drop, Space hard drop, Up or X rotate right,
'   Z or Ctrl rotate left, C or Shift hold, P or Esc pause, M music on/off,
'   F11 full screen or a window.
' Shows a desktop-sized SCREEN with _FULLSCREEN, a sample-by-sample _SNDRAW
' synthesiser that mixes a looping song with sound effects, pre-rendered
' block images, scaled text through _PUTIMAGE, particles and a saved
' high score.
CONST BW = 10, BH = 22, HID = 2, LOCKDELAY = 30, CLEART = 24
CONST NPART = 800, NVOICE = 10, NMSG = 5, NFLOAT = 18, NSTEP = 128
CONST KLEFT = 1, KRIGHT = 2, KDOWN = 3, KUP = 4, KROTR = 5, KROTL = 6, KDROP = 7
CONST KHOLD = 8, KESC = 9, KPAUSE = 10, KENTER = 11, KMUTE = 12, KFULL = 13, NKEY = 13

' screen layout
DIM SHARED SW, SH, CS, BX, BY, TS, TL, LB
' board, pieces (cells of each rotation), SRS wall kicks, colours
DIM SHARED board(BW - 1, BH - 1), pcx(7, 3, 3), pcy(7, 3, 3)
DIM SHARED kickX(1, 3, 4), kickY(1, 3, 4), cr(9), cg(9), cb(9)
' images
DIM SHARED blk(9) AS LONG, lblk(7) AS LONG, letter(9) AS LONG, txtImg AS LONG, bg AS LONG
' game state
DIM SHARED bag(7), bagN, nxt(4), curP, curR, curX, curY, holdP, holdUsed
DIM SHARED score AS LONG, hiScore AS LONG, totLines, level, combo, b2b, newHi
DIM SHARED fallAcc, lockT, lockN, lowY, dasDir, dasT, lastDir
DIM SHARED clrN, clrRow(3), clrT, state, frame AS LONG, overT, quitNow
DIM SHARED shake, shx, shy, keyCur(NKEY), keyPrev(NKEY), fullOn
' particles, floating messages, drifting background shapes
DIM SHARED ptX(NPART), ptY(NPART), ptVX(NPART), ptVY(NPART), ptGr(NPART)
DIM SHARED ptL(NPART), ptM(NPART), ptC(NPART), ptS(NPART), ptNext
DIM SHARED msgS(NMSG) AS STRING, msgT(NMSG), msgC(NMSG)
DIM SHARED flP(NFLOAT), flR(NFLOAT), flX(NFLOAT), flY(NFLOAT), flV(NFLOAT), flS(NFLOAT)
' audio: song steps, synth voices, sound-effect voices
DIM SHARED arate AS LONG, musicOn, musGain, musTarget
DIM SHARED melF(NSTEP), melOn(NSTEP), basF(NSTEP)
DIM SHARED musStep AS LONG, musCnt AS LONG, stepLen AS LONG
DIM SHARED lPh, lEnv, lF, lGate, bPh, bEnv, bF, kPh, kEnv, kF, sEnv, hEnv
DIM SHARED vF(NVOICE), vSl(NVOICE), vAmp(NVOICE), vLen(NVOICE) AS LONG
DIM SHARED vTot(NVOICE) AS LONG, vDel(NVOICE) AS LONG, vW(NVOICE), vPh(NVOICE), vNext

SW = _DESKTOPWIDTH: SH = _DESKTOPHEIGHT
IF SW < 800 OR SH < 600 THEN SW = 1280: SH = 720
SCREEN _NEWIMAGE(SW, SH, 32)
_TITLE "QBNex Stackfall"
_RESIZE ON , _SMOOTH ' as a window it can be resized or maximised
_FULLSCREEN _SQUAREPIXELS: fullOn = -1
_MOUSEHIDE
RANDOMIZE TIMER

CS = INT(SH * 0.88 / 20) ' cell size: 20 visible rows fill 88% of the height
BX = (SW - BW * CS) \ 2: BY = (SH - 20 * CS) \ 2
TS = CS \ 22: IF TS < 1 THEN TS = 1
TL = CS \ 15: IF TL < 2 THEN TL = 2

' Pieces: box size, then the four cells of the spawn rotation. The other
' rotations turn the box clockwise, which gives the standard rotation states.
FOR p = 1 TO 7
    READ n
    FOR k = 0 TO 3: READ pcx(p, 0, k), pcy(p, 0, k): NEXT
    FOR r = 1 TO 3
        FOR k = 0 TO 3
            pcx(p, r, k) = n - 1 - pcy(p, r - 1, k)
            pcy(p, r, k) = pcx(p, r - 1, k)
        NEXT
    NEXT
NEXT
DATA 4, 0,1, 1,1, 2,1, 3,1
DATA 2, 0,0, 1,0, 0,1, 1,1
DATA 3, 1,0, 0,1, 1,1, 2,1
DATA 3, 1,0, 2,0, 0,1, 1,1
DATA 3, 0,0, 1,0, 1,1, 2,1
DATA 3, 0,0, 0,1, 1,1, 2,1
DATA 3, 2,0, 0,1, 1,1, 2,1

' Wall kicks for a clockwise turn out of rotation 0..3 (y points up here);
' a counter-clockwise turn uses the negated kicks of the turn it undoes.
' First four rows: the three-wide pieces, last four: the long bar.
FOR t = 0 TO 1: FOR r = 0 TO 3: FOR i = 1 TO 4: READ kickX(t, r, i), kickY(t, r, i): NEXT: NEXT: NEXT
DATA -1,0, -1,1, 0,-2, -1,-2
DATA 1,0, 1,-1, 0,2, 1,2
DATA 1,0, 1,1, 0,-2, 1,-2
DATA -1,0, -1,-1, 0,2, -1,2
DATA -2,0, 1,0, -2,-1, 1,2
DATA -1,0, 2,0, -1,2, 2,-1
DATA 2,0, -1,0, 2,1, -1,-2
DATA 1,0, -2,0, 1,-2, -2,1

' Colours of the seven pieces, then the clear flash and the game-over grey
FOR c = 1 TO 9: READ cr(c), cg(c), cb(c): NEXT
DATA 240,80,170, 250,200,60, 40,200,190, 130,220,70, 245,95,80, 90,110,245, 250,140,40
DATA 235,235,245, 90,95,115

' The song: an original 16-bar tune in eighth notes ("-" holds, "." rests)
FOR bar = 0 TO 15
    READ m$
    FOR j = 0 TO 7
        t$ = Tok$(m$, j): s = bar * 8 + j
        IF t$ = "-" THEN
            melOn(s) = 0
        ELSEIF t$ = "." THEN
            melOn(s) = -1
        ELSE
            melOn(s) = 1: melF(s) = NoteF(t$)
        END IF
    NEXT
NEXT
DATA "A4 C5 E5 - D5 - C5 D5", "E5 - G5 - E5 D5 C5 -", "F4 A4 C5 - B4 - A4 B4", "C5 - E5 - D5 - . ."
DATA "A4 C5 E5 - D5 - C5 D5", "E5 - G5 - A5 G5 E5 -", "F5 - E5 D5 C5 - B4 C5", "A4 - - - . . E5 G5"
DATA "A5 - G5 E5 - D5 E5 -", "F5 - A5 - G5 - E5 -", "D5 - F5 E5 D5 - C5 -", "B4 - D5 - E5 - - -"
DATA "A5 - G5 E5 - D5 E5 -", "F5 - A5 - C6 - B5 A5", "G5 - E5 C5 D5 - B4 -", "A4 - - - . . . ."
' Bass roots, one per bar, played as root / octave / fifth / octave
FOR bar = 0 TO 15
    READ r$: rf = NoteF(r$)
    FOR j = 0 TO 7
        basF(bar * 8 + j) = rf * 2 ^ (VAL(MID$("0012071200120712", j * 2 + 1, 2)) / 12)
    NEXT
NEXT
DATA "A2", "C3", "F2", "G2", "A2", "C3", "D3", "A2"
DATA "A2", "F2", "D3", "E2", "A2", "F2", "G2", "A2"

' Pre-rendered images: blocks, the background, the text scratch image, the logo
FOR c = 1 TO 9: blk(c) = MakeBlock(c, CS): NEXT
txtImg = _NEWIMAGE(800, 16, 32): _DEST txtImg: _PRINTMODE _KEEPBACKGROUND: _DEST 0
bg = MakeBackground
LB = INT(SW * 0.7 / 72): IF LB > INT(SH * 0.3 / 12) THEN LB = INT(SH * 0.3 / 12)
IF LB < 3 THEN LB = 3
FOR c = 1 TO 7: lblk(c) = MakeBlock(c, LB): NEXT
FOR i = 1 TO 9 ' each logo letter is its font glyph rebuilt from small blocks
    _DEST txtImg: CLS , 0: COLOR _RGB32(255, 255, 255): _PRINTSTRING (0, 0), MID$("STACKFALL", i, 1)
    _SOURCE txtImg
    letter(i) = _NEWIMAGE(8 * LB, 16 * LB, 32): _DEST letter(i): CLS , 0
    c = ((i - 1) MOD 7) + 1
    FOR y = 0 TO 15
        FOR x = 0 TO 7
            IF _RED32(POINT(x, y)) > 100 THEN _PUTIMAGE (x * LB, y * LB), lblk(c)
        NEXT
    NEXT
NEXT
_SOURCE 0: _DEST 0

IF _FILEEXISTS("stackfall.hi") THEN
    OPEN "stackfall.hi" FOR INPUT AS #1
    IF NOT EOF(1) THEN INPUT #1, hiScore
    CLOSE #1
END IF
FOR i = 0 TO NFLOAT - 1: ResetFloater i, -1: NEXT
arate = _SNDRATE: musicOn = -1: musTarget = 0.8: level = 1: state = 0

DO
    frame = frame + 1
    PollKeys
    IF Hit(KMUTE) THEN musicOn = NOT musicOn
    IF Hit(KFULL) THEN
        fullOn = NOT fullOn
        IF fullOn THEN _FULLSCREEN _SQUAREPIXELS ELSE _FULLSCREEN _OFF
    END IF
    SELECT CASE state
        CASE 0: TitleUpdate
        CASE 1: GameUpdate
        CASE 2: PauseUpdate
        CASE 3: OverUpdate
    END SELECT
    shx = 0: shy = 0
    IF shake > 0.5 THEN
        shx = (RND - 0.5) * 2 * shake: shy = (RND - 0.5) * 2 * shake: shake = shake * 0.82
    ELSE
        shake = 0
    END IF
    _DONTBLEND: _PUTIMAGE (0, 0), bg: _BLEND ' a straight copy, the background is opaque
    DrawFloaters
    IF state = 0 THEN DrawTitle ELSE DrawGame
    DrawParticles
    IF state = 2 THEN DrawPause
    IF state = 3 THEN DrawOver
    AudioPump
    _DISPLAY
    _LIMIT 60
LOOP UNTIL quitNow
IF score > hiScore THEN hiScore = score: SaveHi
SYSTEM

' ---------------------------------------------------------------- input

SUB PollKeys
    FOR i = 1 TO NKEY: keyPrev(i) = keyCur(i): NEXT
    keyCur(KLEFT) = _KEYDOWN(19200) <> 0
    keyCur(KRIGHT) = _KEYDOWN(19712) <> 0
    keyCur(KDOWN) = _KEYDOWN(20480) <> 0
    keyCur(KUP) = _KEYDOWN(18432) <> 0
    keyCur(KROTR) = (_KEYDOWN(120) OR _KEYDOWN(88)) <> 0
    keyCur(KROTL) = (_KEYDOWN(122) OR _KEYDOWN(90) OR _KEYDOWN(100305) OR _KEYDOWN(100306)) <> 0
    keyCur(KDROP) = _KEYDOWN(32) <> 0
    keyCur(KHOLD) = (_KEYDOWN(99) OR _KEYDOWN(67) OR _KEYDOWN(100303) OR _KEYDOWN(100304)) <> 0
    keyCur(KESC) = _KEYDOWN(27) <> 0
    keyCur(KPAUSE) = (_KEYDOWN(112) OR _KEYDOWN(80)) <> 0
    keyCur(KENTER) = _KEYDOWN(13) <> 0
    keyCur(KMUTE) = (_KEYDOWN(109) OR _KEYDOWN(77)) <> 0
    keyCur(KFULL) = _KEYDOWN(34048) <> 0
    DO WHILE INKEY$ <> "": LOOP
END SUB

' True only on the frame the key goes down
FUNCTION Hit (i)
    Hit = keyCur(i) AND NOT keyPrev(i)
END FUNCTION

' ---------------------------------------------------------------- game states

SUB TitleUpdate
    IF Hit(KENTER) OR Hit(KDROP) THEN NewGame
    IF Hit(KESC) THEN quitNow = -1
END SUB

SUB PauseUpdate
    IF Hit(KPAUSE) OR Hit(KENTER) THEN state = 1: musTarget = 1: EXIT SUB
    IF Hit(KESC) THEN
        state = 0: musTarget = 0.8
        IF score > hiScore THEN hiScore = score: SaveHi
    END IF
END SUB

SUB OverUpdate
    overT = overT + 1
    IF overT MOD 2 = 0 THEN ' grey the stack out from the bottom up
        y = BH - overT \ 2
        IF y >= 0 THEN
            FOR x = 0 TO BW - 1
                IF board(x, y) THEN board(x, y) = 9
            NEXT
        END IF
    END IF
    IF overT > 50 THEN
        IF Hit(KENTER) OR Hit(KDROP) THEN NewGame: EXIT SUB
        IF Hit(KESC) THEN state = 0: musTarget = 0.8
    END IF
END SUB

SUB NewGame
    FOR y = 0 TO BH - 1: FOR x = 0 TO BW - 1: board(x, y) = 0: NEXT: NEXT
    bagN = 0
    FOR i = 0 TO 4: nxt(i) = BagNext: NEXT
    holdP = 0: holdUsed = 0: score = 0: totLines = 0: level = 1: combo = -1: b2b = 0
    clrT = 0: clrN = 0: dasDir = 0: newHi = 0
    FOR i = 0 TO NMSG - 1: msgT(i) = 0: NEXT
    state = 1: musTarget = 1: musStep = 0: musCnt = 0
    Sfx 523, 1, 0.14, 0.1, 0, 0: Sfx 659, 1, 0.14, 0.1, 0, 0.08
    Sfx 784, 1, 0.14, 0.1, 0, 0.16: Sfx 1047, 1, 0.14, 0.3, 0, 0.24
    SpawnNext
END SUB

SUB GameOver
    state = 3: overT = 0: curP = 0: clrT = 0: newHi = 0
    IF score > hiScore THEN hiScore = score: newHi = -1: SaveHi
    musTarget = 0
    Sfx 523, 0.8, 0.2, 0.25, 0, 0: Sfx 392, 0.8, 0.2, 0.25, 0, 0.25
    Sfx 330, 0.8, 0.2, 0.25, 0, 0.5: Sfx 262, 0.5, 0.22, 0.7, 0, 0.75
END SUB

SUB SaveHi
    OPEN "stackfall.hi" FOR OUTPUT AS #1
    PRINT #1, hiScore
    CLOSE #1
END SUB

SUB GameUpdate
    IF Hit(KESC) OR Hit(KPAUSE) THEN
        state = 2: musTarget = 0.25
        Sfx 660, 1, 0.14, 0.06, 0, 0: Sfx 440, 1, 0.14, 0.08, 0, 0.07
        EXIT SUB
    END IF
    IF clrT > 0 THEN ' rows are flashing away; the next piece waits
        clrT = clrT + 1
        IF clrT >= CLEART THEN clrT = 0: CollapseRows: SpawnNext
        EXIT SUB
    END IF
    IF curP = 0 THEN EXIT SUB
    IF Hit(KHOLD) THEN
        DoHold
        IF curP = 0 OR state <> 1 THEN EXIT SUB
    END IF
    IF Hit(KUP) OR Hit(KROTR) THEN r = TryRotate(1)
    IF Hit(KROTL) THEN r = TryRotate(-1)
    ' sideways: one step on the press, then auto-repeat after a short delay
    IF Hit(KLEFT) THEN lastDir = -1
    IF Hit(KRIGHT) THEN lastDir = 1
    d = 0
    IF keyCur(KLEFT) THEN d = -1
    IF keyCur(KRIGHT) THEN d = 1
    IF keyCur(KLEFT) AND keyCur(KRIGHT) THEN d = lastDir
    IF d <> dasDir THEN
        dasDir = d: dasT = 0
        IF d <> 0 THEN r = TryShift(d)
    ELSEIF d <> 0 THEN
        dasT = dasT + 1
        IF dasT >= 10 AND (dasT - 10) MOD 2 = 0 THEN r = TryShift(d)
    END IF
    IF Hit(KDROP) THEN HardDrop: EXIT SUB
    ' gravity, faster while Down is held
    g = GravRows
    soft = keyCur(KDOWN)
    IF soft AND g < 0.5 THEN g = 0.5
    fallAcc = fallAcc + g
    n = 0
    DO WHILE fallAcc >= 1 AND n < BH
        fallAcc = fallAcc - 1: n = n + 1
        IF Fits(curP, curR, curX, curY + 1) THEN
            curY = curY + 1
            IF soft THEN score = score + 1
            IF curY > lowY THEN lowY = curY: lockN = 0: lockT = 0
        ELSE
            fallAcc = 0
            EXIT DO
        END IF
    LOOP
    ' lock delay: a resting piece locks after half a second
    IF Fits(curP, curR, curX, curY + 1) THEN
        lockT = 0
    ELSE
        lockT = lockT + 1
        IF lockT >= LOCKDELAY THEN LockPiece
    END IF
END SUB

' Rows fallen per frame at the current level
FUNCTION GravRows
    lv = level: IF lv > 20 THEN lv = 20
    sec = (0.8 - (lv - 1) * 0.007) ^ (lv - 1)
    GravRows = 1 / (sec * 60)
END FUNCTION

' ---------------------------------------------------------------- piece logic

FUNCTION Fits (p, r, x, y)
    Fits = 0
    FOR k = 0 TO 3
        cx = x + pcx(p, r, k): cy = y + pcy(p, r, k)
        IF cx < 0 OR cx >= BW OR cy >= BH THEN EXIT FUNCTION
        IF cy >= 0 THEN
            IF board(cx, cy) <> 0 THEN EXIT FUNCTION
        END IF
    NEXT
    Fits = -1
END FUNCTION

FUNCTION BagNext ' all seven pieces in a shuffled bag, then a new bag
    IF bagN <= 0 THEN
        FOR i = 1 TO 7: bag(i) = i: NEXT
        FOR i = 7 TO 2 STEP -1: j = INT(RND * i) + 1: SWAP bag(i), bag(j): NEXT
        bagN = 7
    END IF
    BagNext = bag(bagN): bagN = bagN - 1
END FUNCTION

SUB SpawnNext
    p = nxt(0)
    FOR i = 0 TO 3: nxt(i) = nxt(i + 1): NEXT
    nxt(4) = BagNext
    Spawn p
END SUB

SUB Spawn (p)
    curP = p: curR = 0: curX = 3: IF p = 2 THEN curX = 4
    curY = 1
    IF NOT Fits(curP, curR, curX, curY) THEN
        curY = 0
        IF NOT Fits(curP, curR, curX, curY) THEN GameOver: EXIT SUB
    END IF
    lowY = curY: lockT = 0: lockN = 0: fallAcc = 0
END SUB

' A successful move or turn on the ground restarts the lock delay, 15 times at most
SUB AfterMove
    IF curY > lowY THEN lowY = curY: lockN = 0
    IF NOT Fits(curP, curR, curX, curY + 1) THEN
        IF lockN < 15 THEN lockT = 0: lockN = lockN + 1
    END IF
END SUB

FUNCTION TryShift (d)
    TryShift = 0
    IF Fits(curP, curR, curX + d, curY) THEN
        curX = curX + d: AfterMove: Sfx 900, 0.7, 0.06, 0.02, 0, 0: TryShift = -1
    END IF
END FUNCTION

FUNCTION TryRotate (dir)
    TryRotate = 0
    IF curP = 2 THEN EXIT FUNCTION ' the square looks the same every way round
    nr = (curR + dir + 4) MOD 4
    t = 0: IF curP = 1 THEN t = 1
    FOR i = 0 TO 4
        IF i = 0 THEN
            dx = 0: dy = 0
        ELSEIF dir = 1 THEN
            dx = kickX(t, curR, i): dy = -kickY(t, curR, i)
        ELSE
            dx = -kickX(t, nr, i): dy = kickY(t, nr, i)
        END IF
        IF Fits(curP, nr, curX + dx, curY + dy) THEN
            curX = curX + dx: curY = curY + dy: curR = nr
            AfterMove
            Sfx 520, 2.2, 0.12, 0.05, 1, 0
            TryRotate = -1
            EXIT FUNCTION
        END IF
    NEXT
END FUNCTION

SUB DoHold
    IF holdUsed THEN Sfx 140, 1, 0.08, 0.05, 0, 0: EXIT SUB
    t = holdP: holdP = curP
    IF t = 0 THEN SpawnNext ELSE Spawn t
    holdUsed = -1
    Sfx 440, 1.8, 0.12, 0.06, 1, 0: Sfx 660, 1.8, 0.12, 0.06, 1, 0.05
END SUB

SUB HardDrop
    d = 0
    DO WHILE Fits(curP, curR, curX, curY + 1): curY = curY + 1: d = d + 1: LOOP
    score = score + 2 * d
    FOR k = 0 TO 3 ' a streak of sparks along the path of the drop
        x = BX + (curX + pcx(curP, curR, k)) * CS
        y = BY + (curY + pcy(curP, curR, k) - HID) * CS
        FOR j = 1 TO 5
            Spark x + RND * CS, y - RND * d * CS * 0.8, 0, -0.5 - RND * 1.5, 0, 18 + RND * 14, curP, CS / 10 + 1
        NEXT
    NEXT
    shake = CS * 0.1 + d * 0.25: IF shake > CS * 0.3 THEN shake = CS * 0.3
    Sfx 160, 0.25, 0.4, 0.16, 3, 0: Sfx 1, 1, 0.12, 0.07, 2, 0
    LockPiece
END SUB

SUB LockPiece
    hidden = -1
    FOR k = 0 TO 3
        x = curX + pcx(curP, curR, k): y = curY + pcy(curP, curR, k)
        IF y >= 0 THEN board(x, y) = curP
        IF y >= HID THEN hidden = 0
    NEXT
    curP = 0: holdUsed = 0: lockT = 0
    Sfx 230, 0.4, 0.18, 0.07, 1, 0
    IF hidden THEN GameOver: EXIT SUB ' locked entirely above the well
    clrN = 0
    FOR y = 0 TO BH - 1
        full = -1
        FOR x = 0 TO BW - 1
            IF board(x, y) = 0 THEN full = 0
        NEXT
        IF full THEN clrRow(clrN) = y: clrN = clrN + 1
    NEXT
    IF clrN = 0 THEN combo = -1: SpawnNext: EXIT SUB
    SELECT CASE clrN
        CASE 1: pts = 100
        CASE 2: pts = 300: Msg "DOUBLE", 4
        CASE 3: pts = 500: Msg "TRIPLE", 7
        CASE ELSE: pts = 800: Msg "QUAD!", 1
    END SELECT
    IF clrN = 4 THEN
        IF b2b THEN pts = pts * 3 / 2: Msg "BACK-TO-BACK", 2
        b2b = -1
    ELSE
        b2b = 0
    END IF
    combo = combo + 1
    score = score + pts * level
    IF combo > 0 THEN score = score + 50 * combo * level: Msg "COMBO x" + LTRIM$(STR$(combo)), 3
    old = level
    totLines = totLines + clrN
    level = 1 + totLines \ 10
    IF level > old THEN Msg "LEVEL " + LTRIM$(STR$(level)), 6: LevelSound
    ClearSound clrN
    FOR i = 0 TO clrN - 1 ' every cleared cell bursts into sparks
        y = clrRow(i)
        FOR x = 0 TO BW - 1
            c = board(x, y)
            FOR j = 1 TO 4
                a = RND * 6.2832: v = (1 + RND * 4) * CS / 30
                Spark BX + x * CS + CS / 2, BY + (y - HID) * CS + CS / 2, COS(a) * v * 1.6, SIN(a) * v - CS / 18, CS / 220, 30 + RND * 30, c, CS / 8 + 1
            NEXT
        NEXT
    NEXT
    IF clrN = 4 THEN shake = CS * 0.3
    clrT = 1
END SUB

SUB CollapseRows
    dst = BH - 1
    FOR src = BH - 1 TO 0 STEP -1
        gone = 0
        FOR i = 0 TO clrN - 1
            IF clrRow(i) = src THEN gone = -1
        NEXT
        IF NOT gone THEN
            IF dst <> src THEN
                FOR x = 0 TO BW - 1: board(x, dst) = board(x, src): NEXT
            END IF
            dst = dst - 1
        END IF
    NEXT
    FOR y = dst TO 0 STEP -1
        FOR x = 0 TO BW - 1: board(x, y) = 0: NEXT
    NEXT
    clrN = 0
    empty = -1
    FOR y = 0 TO BH - 1
        FOR x = 0 TO BW - 1
            IF board(x, y) THEN empty = 0
        NEXT
    NEXT
    IF empty THEN score = score + 2000 * level: Msg "ALL CLEAR", 5: LevelSound
END SUB

SUB Msg (t$, c)
    slot = 0: best = 1E+9
    FOR i = 0 TO NMSG - 1
        IF msgT(i) <= 0 THEN slot = i: EXIT FOR
        IF msgT(i) < best THEN best = msgT(i): slot = i
    NEXT
    msgS(slot) = t$: msgT(slot) = 110: msgC(slot) = c
END SUB

' ---------------------------------------------------------------- drawing

SUB DrawGame
    x0 = BX + shx: y0 = BY + shy: wpx = BW * CS
    DrawWell x0, y0
    FOR y = HID TO BH - 1
        FOR x = 0 TO BW - 1
            c = board(x, y)
            IF c THEN _PUTIMAGE (x0 + x * CS, y0 + (y - HID) * CS), blk(c)
        NEXT
    NEXT
    IF clrT > 0 THEN ' cleared rows flash white, then close in from both sides
        f = clrT / CLEART
        FOR i = 0 TO clrN - 1
            IF clrRow(i) >= HID THEN
                ry = y0 + (clrRow(i) - HID) * CS
                cover = f * f * wpx / 2
                LINE (x0, ry)-(x0 + cover, ry + CS - 1), _RGBA32(4, 6, 18, 255), BF
                LINE (x0 + wpx - 1 - cover, ry)-(x0 + wpx - 1, ry + CS - 1), _RGBA32(4, 6, 18, 255), BF
                LINE (x0 + cover, ry)-(x0 + wpx - 1 - cover, ry + CS - 1), _RGBA32(255, 255, 255, 230 * (1 - f)), BF
            END IF
        NEXT
    END IF
    IF curP > 0 THEN
        c = curP
        gy = curY ' ghost: where the piece would land
        DO WHILE Fits(curP, curR, curX, gy + 1): gy = gy + 1: LOOP
        FOR k = 0 TO 3
            y = gy + pcy(c, curR, k)
            IF y >= HID THEN
                px = x0 + (curX + pcx(c, curR, k)) * CS: py = y0 + (y - HID) * CS
                LINE (px + 2, py + 2)-(px + CS - 3, py + CS - 3), _RGBA32(cr(c), cg(c), cb(c), 40), BF
                LINE (px + 2, py + 2)-(px + CS - 3, py + CS - 3), _RGBA32(cr(c), cg(c), cb(c), 170), B
            END IF
        NEXT
        fl = lockT / LOCKDELAY * 110
        FOR k = 0 TO 3
            y = curY + pcy(c, curR, k)
            IF y >= HID - 1 THEN
                px = x0 + (curX + pcx(c, curR, k)) * CS: py = y0 + (y - HID) * CS
                _PUTIMAGE (px, py), blk(c)
                IF fl > 0 THEN LINE (px, py)-(px + CS - 1, py + CS - 1), _RGBA32(255, 255, 255, fl), BF
            END IF
        NEXT
    END IF

    ' left column: hold box and the numbers
    lw = INT(CS * 6): lx = BX - INT(CS * 0.7) - lw: rx = BX + wpx + INT(CS * 0.7)
    Panel lx, BY, lw, CS * 4.2
    Txt lx + CS * 0.4, BY + CS * 0.35, TS, "HOLD", _RGB32(150, 160, 215)
    IF holdP THEN
        hc = holdP: IF holdUsed THEN hc = 9
        DrawPiece holdP, hc, lx + lw / 2, BY + CS * 2.55, CS * 0.8
    END IF
    sy = BY + CS * 4.8
    Panel lx, sy, lw, BY + 20 * CS - sy
    hs& = hiScore: IF score > hs& THEN hs& = score
    Stat lx + CS * 0.4, sy + CS * 0.4, "SCORE", RIGHT$("0000000" + LTRIM$(STR$(score)), 7)
    Stat lx + CS * 0.4, sy + CS * 3, "HI SCORE", RIGHT$("0000000" + LTRIM$(STR$(hs&)), 7)
    Stat lx + CS * 0.4, sy + CS * 5.6, "LEVEL", LTRIM$(STR$(level))
    Stat lx + CS * 0.4, sy + CS * 8.2, "LINES", LTRIM$(STR$(totLines))
    IF musicOn THEN m$ = "MUSIC ON" ELSE m$ = "MUSIC OFF"
    Txt lx + CS * 0.4, BY + 20 * CS - CS * 0.4 - 16 * TS, TS, m$, _RGB32(110, 120, 170)

    ' right column: next pieces and the keys
    Panel rx, BY, lw, CS * 13.2
    Txt rx + CS * 0.4, BY + CS * 0.35, TS, "NEXT", _RGB32(150, 160, 215)
    DrawPiece nxt(0), nxt(0), rx + lw / 2, BY + CS * 2.6, CS * 0.85
    FOR i = 1 TO 4
        DrawPiece nxt(i), nxt(i), rx + lw / 2, BY + CS * (5.2 + (i - 1) * 2.1), CS * 0.6
    NEXT
    cy = BY + CS * 13.8
    Panel rx, cy, lw, BY + 20 * CS - cy
    k$ = "ARROWS|MOVE|DOWN|SOFT DROP|SPACE|HARD DROP|UP X Z|ROTATE|C|HOLD|P|PAUSE|M|MUSIC"
    FOR i = 0 TO 6
        a$ = Part$(k$, i * 2): b$ = Part$(k$, i * 2 + 1)
        ty = cy + CS * 0.4 + i * (16 * TS + CS \ 8)
        Txt rx + CS * 0.4, ty, TS, a$, _RGB32(255, 220, 120)
        Txt rx + lw - CS * 0.4 - LEN(b$) * 8 * TS, ty, TS, b$, _RGB32(190, 195, 225)
    NEXT

    ' floating messages over the well
    FOR i = 0 TO NMSG - 1
        IF msgT(i) > 0 THEN
            IF state = 1 THEN msgT(i) = msgT(i) - 1
            a = msgT(i) * 6: IF a > 255 THEN a = 255
            c = msgC(i)
            y = BY + 5 * CS + i * (16 * TL + CS / 4) - (110 - msgT(i)) * CS / 60
            TxtC BX + wpx / 2, y, TL, msgS(i), _RGBA32(Lt(cr(c)), Lt(cg(c)), Lt(cb(c)), a)
        END IF
    NEXT
END SUB

SUB DrawWell (x0, y0)
    w = BW * CS: h = 20 * CS
    gc = ((level - 1) MOD 7) + 1 ' the glow takes the colour of the level
    FOR i = 10 TO 1 STEP -1
        LINE (x0 - i, y0 - i)-(x0 + w - 1 + i, y0 + h - 1 + i), _RGBA32(cr(gc), cg(gc), cb(gc), 90 - i * 8), B
    NEXT
    LINE (x0, y0)-(x0 + w - 1, y0 + h - 1), _RGBA32(4, 6, 18, 220), BF
    FOR x = 1 TO BW - 1: LINE (x0 + x * CS, y0)-(x0 + x * CS, y0 + h - 1), _RGBA32(255, 255, 255, 14): NEXT
    FOR y = 1 TO 19: LINE (x0, y0 + y * CS)-(x0 + w - 1, y0 + y * CS), _RGBA32(255, 255, 255, 14): NEXT
    LINE (x0 - 1, y0 - 1)-(x0 + w, y0 + h), _RGBA32(210, 220, 255, 170), B
END SUB

SUB DrawPiece (p, c, cx, cy, size)
    sz = INT(size)
    mnx = 9: mxx = -9: mny = 9: mxy = -9
    FOR k = 0 TO 3
        IF pcx(p, 0, k) < mnx THEN mnx = pcx(p, 0, k)
        IF pcx(p, 0, k) > mxx THEN mxx = pcx(p, 0, k)
        IF pcy(p, 0, k) < mny THEN mny = pcy(p, 0, k)
        IF pcy(p, 0, k) > mxy THEN mxy = pcy(p, 0, k)
    NEXT
    w = (mxx - mnx + 1) * sz: h = (mxy - mny + 1) * sz
    FOR k = 0 TO 3
        x = INT(cx - w / 2 + (pcx(p, 0, k) - mnx) * sz)
        y = INT(cy - h / 2 + (pcy(p, 0, k) - mny) * sz)
        _PUTIMAGE (x, y)-(x + sz - 1, y + sz - 1), blk(c)
    NEXT
END SUB

SUB Panel (x, y, w, h)
    LINE (x, y)-(x + w - 1, y + h - 1), _RGBA32(10, 12, 34, 190), BF
    LINE (x, y)-(x + w - 1, y + h - 1), _RGBA32(130, 150, 255, 120), B
    LINE (x + 3, y + 3)-(x + w - 4, y + h - 4), _RGBA32(130, 150, 255, 35), B
END SUB

SUB Stat (x, y, label$, value$)
    Txt x, y, TS, label$, _RGB32(150, 160, 215)
    TxtSh x, y + 16 * TS + CS * 0.12, TL, value$, _RGB32(255, 255, 255)
END SUB

SUB DrawTitle
    w = 9 * 8 * LB: x0 = (SW - w) / 2: y0 = SH * 0.07
    FOR i = 1 TO 9
        _PUTIMAGE (x0 + (i - 1) * 8 * LB, y0 + SIN(frame / 18 + i * 0.7) * LB * 0.8), letter(i)
    NEXT
    TxtC SW / 2, SH * 0.45, TS, "A FALLING-BLOCK PUZZLE MADE WITH QBNEX", _RGB32(180, 190, 240)
    IF (frame \ 30) MOD 2 = 0 THEN TxtC SW / 2, SH * 0.55, TL, "PRESS ENTER TO PLAY", _RGB32(255, 225, 110)
    TxtC SW / 2, SH * 0.64, TS, "HI SCORE  " + RIGHT$("0000000" + LTRIM$(STR$(hiScore)), 7), _RGB32(255, 255, 255)
    y = SH * 0.74: dy = 16 * TS * 1.6
    TxtC SW / 2, y, TS, "ARROWS MOVE     DOWN SOFT DROP     SPACE HARD DROP", _RGB32(170, 180, 220)
    TxtC SW / 2, y + dy, TS, "UP / X ROTATE     Z ROTATE BACK     C HOLD", _RGB32(170, 180, 220)
    TxtC SW / 2, y + dy * 2, TS, "P PAUSE     M MUSIC ON/OFF     F11 FULL SCREEN     ESC QUIT", _RGB32(170, 180, 220)
END SUB

SUB DrawPause
    LINE (0, 0)-(SW - 1, SH - 1), _RGBA32(0, 0, 10, 160), BF
    TxtC SW / 2, SH * 0.36, TL * 2, "PAUSED", _RGB32(255, 255, 255)
    TxtC SW / 2, SH * 0.52, TS, "P OR ENTER  RESUME", _RGB32(255, 225, 110)
    TxtC SW / 2, SH * 0.52 + 24 * TS, TS, "ESC  QUIT TO TITLE", _RGB32(180, 190, 240)
END SUB

SUB DrawOver
    IF overT < 30 THEN EXIT SUB
    wpx = BW * CS: cx = BX + wpx / 2
    Panel BX + CS * 0.5, BY + CS * 6, wpx - CS, CS * 8
    TxtC cx, BY + CS * 6.6, TL, "GAME OVER", _RGB32(255, 120, 110)
    TxtC cx, BY + CS * 8.3, TS, "SCORE", _RGB32(150, 160, 215)
    TxtC cx, BY + CS * 8.3 + 18 * TS, TL, LTRIM$(STR$(score)), _RGB32(255, 255, 255)
    IF newHi AND (frame \ 20) MOD 2 = 0 THEN TxtC cx, BY + CS * 10.6, TS, "NEW HIGH SCORE!", _RGB32(255, 215, 80)
    IF overT > 50 THEN
        TxtC cx, BY + CS * 11.8, TS, "ENTER  PLAY AGAIN", _RGB32(255, 225, 110)
        TxtC cx, BY + CS * 11.8 + 20 * TS, TS, "ESC  TITLE", _RGB32(180, 190, 240)
    END IF
END SUB

SUB DrawFloaters ' big faint pieces drifting down behind everything
    FOR i = 0 TO NFLOAT - 1
        flY(i) = flY(i) + flV(i)
        IF flY(i) > SH + flS(i) THEN ResetFloater i, 0
        p = flP(i): r = flR(i): s = flS(i)
        FOR k = 0 TO 3
            x = flX(i) + pcx(p, r, k) * s: y = flY(i) + pcy(p, r, k) * s
            LINE (x + 1, y + 1)-(x + s - 2, y + s - 2), _RGBA32(cr(p), cg(p), cb(p), 24), BF
            LINE (x + 1, y + 1)-(x + s - 2, y + s - 2), _RGBA32(cr(p), cg(p), cb(p), 48), B
        NEXT
    NEXT
END SUB

SUB ResetFloater (i, first)
    flP(i) = INT(RND * 7) + 1: flR(i) = INT(RND * 4)
    flS(i) = CS * (0.6 + RND * 1.1): flV(i) = 0.3 + RND * 0.9
    flX(i) = RND * SW - flS(i) * 2
    IF first THEN flY(i) = RND * SH ELSE flY(i) = -flS(i) * 4 - RND * SH * 0.3
END SUB

SUB Spark (x, y, vx, vy, gr, life, c, size)
    i = ptNext: ptNext = (ptNext + 1) MOD NPART
    ptX(i) = x: ptY(i) = y: ptVX(i) = vx: ptVY(i) = vy: ptGr(i) = gr
    ptL(i) = life: ptM(i) = life: ptC(i) = c: ptS(i) = size
END SUB

SUB DrawParticles
    FOR i = 0 TO NPART - 1
        IF ptL(i) > 0 THEN
            ptX(i) = ptX(i) + ptVX(i): ptY(i) = ptY(i) + ptVY(i): ptVY(i) = ptVY(i) + ptGr(i)
            ptL(i) = ptL(i) - 1
            c = ptC(i): s = ptS(i)
            LINE (ptX(i), ptY(i))-(ptX(i) + s, ptY(i) + s), _RGBA32(Lt(cr(c)), Lt(cg(c)), Lt(cb(c)), 255 * ptL(i) / ptM(i)), BF
        END IF
    NEXT
END SUB

' ---------------------------------------------------------------- text

' Draws t$ with the 8x16 font scaled s times
SUB Txt (x, y, s, t$, col AS _UNSIGNED LONG)
    n = LEN(t$): IF n = 0 OR n > 100 THEN EXIT SUB
    _DEST txtImg: CLS , 0: COLOR col: _PRINTSTRING (0, 0), t$: _DEST 0
    _PUTIMAGE (x, y)-(x + n * 8 * s - 1, y + 16 * s - 1), txtImg, 0, (0, 0)-(n * 8 - 1, 15)
END SUB

SUB TxtSh (x, y, s, t$, col AS _UNSIGNED LONG)
    Txt x + s, y + s, s, t$, _RGBA32(0, 0, 0, _ALPHA32(col) * 0.65)
    Txt x, y, s, t$, col
END SUB

SUB TxtC (cx, y, s, t$, col AS _UNSIGNED LONG)
    TxtSh INT(cx - LEN(t$) * 4 * s), y, s, t$, col
END SUB

FUNCTION Lt (v)
    Lt = v + (255 - v) * 0.35
END FUNCTION

' The n-th (from 0) item of a "|"-separated list
FUNCTION Part$ (s$, n)
    a = 1
    FOR i = 1 TO n
        a = INSTR(a, s$, "|") + 1
        IF a = 1 THEN Part$ = "": EXIT FUNCTION
    NEXT
    b = INSTR(a, s$, "|"): IF b = 0 THEN b = LEN(s$) + 1
    Part$ = MID$(s$, a, b - a)
END FUNCTION

' The n-th (from 0) space-separated word of s$
FUNCTION Tok$ (s$, n)
    k = 0: w$ = ""
    FOR i = 1 TO LEN(s$) + 1
        ch$ = MID$(s$, i, 1)
        IF ch$ = " " OR ch$ = "" THEN
            IF w$ <> "" THEN
                IF k = n THEN Tok$ = w$: EXIT FUNCTION
                k = k + 1: w$ = ""
            END IF
        ELSE
            w$ = w$ + ch$
        END IF
    NEXT
    Tok$ = ""
END FUNCTION

' ---------------------------------------------------------------- images

FUNCTION Cl (v)
    x = v: IF x < 0 THEN x = 0
    IF x > 255 THEN x = 255
    Cl = x
END FUNCTION

FUNCTION MakeBlock& (c, size)
    DIM img AS LONG
    img = _NEWIMAGE(size, size, 32): _DEST img
    r = cr(c): g = cg(c): b = cb(c)
    FOR y = 0 TO size - 1
        f = 1.2 - 0.45 * y / size
        LINE (0, y)-(size - 1, y), _RGB32(Cl(r * f), Cl(g * f), Cl(b * f))
    NEXT
    e = size \ 8: IF e < 1 THEN e = 1
    FOR i = 0 TO e - 1 ' bevel: light top and left, dark bottom and right
        LINE (i, i)-(size - 1 - i, i), _RGBA32(255, 255, 255, 120)
        LINE (i, i + 1)-(i, size - 1 - i), _RGBA32(255, 255, 255, 60)
        LINE (i + 1, size - 1 - i)-(size - 1 - i, size - 1 - i), _RGBA32(0, 0, 0, 120)
        LINE (size - 1 - i, i + 1)-(size - 1 - i, size - 2 - i), _RGBA32(0, 0, 0, 80)
    NEXT
    LINE (e + 1, e + 1)-(e + size \ 4, e + 1 + size \ 10), _RGBA32(255, 255, 255, 140), BF
    LINE (0, 0)-(size - 1, size - 1), _RGB32(r * 0.3, g * 0.3, b * 0.3), B
    _DEST 0
    MakeBlock& = img
END FUNCTION

FUNCTION MakeBackground&
    DIM img AS LONG
    img = _NEWIMAGE(SW, SH, 32): _DEST img
    FOR y = 0 TO SH - 1
        t = y / SH
        LINE (0, y)-(SW - 1, y), _RGB32(16 + 6 * t, 12 + 14 * t, 42 - 10 * t)
    NEXT
    Glow SW * 0.18, SH * 0.25, SH * 0.45, 120, 60, 200
    Glow SW * 0.85, SH * 0.75, SH * 0.5, 30, 140, 190
    Glow SW * 0.5, SH * 1.05, SH * 0.4, 200, 70, 150
    FOR y = CS \ 2 TO SH STEP CS
        FOR x = CS \ 2 TO SW STEP CS: PSET (x, y), _RGBA32(255, 255, 255, 22): NEXT
    NEXT
    _DEST 0
    MakeBackground& = img
END FUNCTION

SUB Glow (gx, gy, rad, r0, g0, b0) ' a soft round light from stacked faint discs
    FOR layer = 1 TO 6
        rr = rad * layer / 6
        FOR dy = -rr TO rr
            w = SQR(ABS(rr * rr - dy * dy))
            LINE (gx - w, gy + dy)-(gx + w, gy + dy), _RGBA32(r0, g0, b0, 7)
        NEXT
    NEXT
END SUB

' ---------------------------------------------------------------- audio

FUNCTION NoteF (t$) ' "A4", "C#5" ... to a frequency in Hz
    semi = INSTR("C D EF G A B", LEFT$(t$, 1)) - 1
    o$ = MID$(t$, 2)
    IF LEFT$(o$, 1) = "#" THEN semi = semi + 1: o$ = MID$(o$, 2)
    m = 12 * (VAL(o$) + 1) + semi
    NoteF = 440 * 2 ^ ((m - 69) / 12)
END FUNCTION

' One sound effect: start frequency, pitch change per second (x times),
' loudness, length and delay in seconds, wave 0 square 1 triangle 2 noise 3 sine
SUB Sfx (f, slidePerSec, amp, dur, wave, delay)
    IF arate <= 0 THEN EXIT SUB
    v = -1
    FOR i = 0 TO NVOICE - 1
        IF vLen(i) <= 0 AND vDel(i) <= 0 THEN v = i: EXIT FOR
    NEXT
    IF v < 0 THEN v = vNext: vNext = (vNext + 1) MOD NVOICE
    vF(v) = f: vSl(v) = slidePerSec ^ (1 / arate): vAmp(v) = amp
    vTot(v) = dur * arate: IF vTot(v) < 1 THEN vTot(v) = 1
    vLen(v) = vTot(v): vDel(v) = delay * arate: vW(v) = wave: vPh(v) = 0
END SUB

SUB ClearSound (n) ' a rising sparkle, longer for more rows
    last = n + 1: IF n = 4 THEN last = 7
    FOR i = 0 TO last
        f = 784 * 2 ^ (VAL(MID$("00040709121619212428", i * 2 + 1, 2)) / 12)
        Sfx f, 1, 0.12, 0.12, 0, i * 0.045
    NEXT
    Sfx 1, 1, 0.06 + n * 0.02, 0.25, 2, 0
    IF n = 4 THEN Sfx 110, 0.5, 0.3, 0.45, 3, 0
END SUB

SUB LevelSound
    FOR i = 0 TO 5
        Sfx 523 * 2 ^ (VAL(MID$("000407121619", i * 2 + 1, 2)) / 12), 1, 0.12, 0.14, 1, 0.3 + i * 0.06
    NEXT
END SUB

' Keeps about 70 ms of sound queued: the song (lead, bass, drums) plus effects,
' worked out one sample at a time and sent with _SNDRAW.
SUB AudioPump
    IF arate <= 0 THEN EXIT SUB
    bpm = 126 + (level - 1) * 4: IF bpm > 176 THEN bpm = 176
    IF state = 0 THEN bpm = 116
    stepLen = arate * 60 / bpm / 2
    IF musicOn THEN tg = musTarget ELSE tg = 0
    chunks = 0
    DO WHILE _SNDRAWLEN < 0.07 AND chunks < 16
        chunks = chunks + 1
        FOR n = 1 TO 256
            IF musCnt <= 0 THEN ' next eighth note
                musCnt = stepLen
                s = musStep
                IF melOn(s) = 1 THEN lF = melF(s): lEnv = 1: lGate = -1
                IF melOn(s) = -1 THEN lGate = 0
                bF = basF(s): bEnv = 1
                b = s MOD 8
                IF b = 0 OR b = 3 OR b = 6 THEN kEnv = 1: kF = 150: kPh = 0
                IF b = 4 THEN sEnv = 1
                IF b MOD 2 = 1 THEN hEnv = 1
                musStep = (musStep + 1) MOD NSTEP
            END IF
            musCnt = musCnt - 1
            ' lead: a pulse mixed with a triangle
            lPh = lPh + lF / arate: IF lPh >= 1 THEN lPh = lPh - 1
            IF lGate THEN lEnv = 0.55 + (lEnv - 0.55) * 0.99985 ELSE lEnv = lEnv * 0.9993
            IF lPh < 0.25 THEN sq = 1 ELSE sq = -1
            lead = (sq * 0.45 + (4 * ABS(lPh - 0.5) - 1) * 0.55) * lEnv
            ' bass: a plucked triangle
            bPh = bPh + bF / arate: IF bPh >= 1 THEN bPh = bPh - 1
            bEnv = 0.35 + (bEnv - 0.35) * 0.9998
            bass = (4 * ABS(bPh - 0.5) - 1) * bEnv
            ' drums: a falling sine kick, a noise snare and a short hi-hat
            dr = 0
            IF kEnv > 0.002 THEN
                kPh = kPh + kF / arate: kF = kF * 0.99969: dr = SIN(kPh * 6.2832) * kEnv * 0.35: kEnv = kEnv * 0.99975
            END IF
            IF sEnv > 0.002 OR hEnv > 0.002 THEN
                nz = RND * 2 - 1
                dr = dr + nz * (sEnv * 0.1 + hEnv * 0.04)
                sEnv = sEnv * 0.9993: hEnv = hEnv * 0.9965
            END IF
            musGain = musGain + (tg - musGain) * 0.0005
            o = (lead * 0.13 + bass * 0.18 + dr) * musGain
            ' sound effects
            FOR v = 0 TO NVOICE - 1
                IF vDel(v) > 0 THEN
                    vDel(v) = vDel(v) - 1
                ELSEIF vLen(v) > 0 THEN
                    vPh(v) = vPh(v) + vF(v) / arate: IF vPh(v) >= 1 THEN vPh(v) = vPh(v) - INT(vPh(v))
                    vF(v) = vF(v) * vSl(v)
                    SELECT CASE vW(v)
                        CASE 0: IF vPh(v) < 0.5 THEN w = 1 ELSE w = -1
                        CASE 1: w = 4 * ABS(vPh(v) - 0.5) - 1
                        CASE 2: w = RND * 2 - 1
                        CASE ELSE: w = SIN(vPh(v) * 6.2832)
                    END SELECT
                    o = o + w * vAmp(v) * vLen(v) / vTot(v) * 0.5
                    vLen(v) = vLen(v) - 1
                END IF
            NEXT
            IF o > 0.95 THEN o = 0.95
            IF o < -0.95 THEN o = -0.95
            _SNDRAW o
        NEXT
    LOOP
END SUB
