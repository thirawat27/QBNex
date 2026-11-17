' ============================================================================
'  sort3d.bas  -  the classic QuickBASIC SORTDEMO, with 3D bars, for QBNex 2.0
'                 (preview.4 or newer, needs the GPU 3D layer)
'
'  Six sorting algorithms like in Microsoft's SORTDEMO.BAS: Insertion,
'  Bubble, Heap, Exchange, Shell and Quick sort. The bars stand as lit 3D
'  columns on a floor, the camera sways slowly, and every change plays a
'  tone like in the original.
'
'  Keys:  I B H E S Q = sort   T = sound on/off   < > = slower/faster
'         Esc = stop a running sort / end the program
'
'  Written by Gokipp (https://github.com/Gokipp).
' ============================================================================
CONST PI = 3.14159265
CONST NBARS = 40
DIM SHARED SW, SH
SW = 1100: SH = 660
SCREEN _NEWIMAGE(SW, SH, 32)
_TITLE "QuickBASIC Sorting Demo 3D"
_PRINTMODE _KEEPBACKGROUND
RANDOMIZE TIMER

IF _GPU3D = 0 THEN
    PRINT "No GPU for 3D found - SORT3D needs QBNex 2.0 (preview.4 or newer)."
    PRINT "Press a key ...": SLEEP: SYSTEM
END IF

DIM SHARED barLen(NBARS), backup(NBARS), hiA, hiB
DIM SHARED cube&(15), floor&, lamp&, lamp2&
DIM SHARED pauseTicks, soundOn, selection, startTime#, aborted, camT#
DIM SHARED optTitle$(9), optTime$(6)
RESTORE options
FOR i = 1 TO 9: READ optTitle$(i): NEXT

options:
DATA "Insertion","Bubble","Heap","Exchange","Shell","Quick","Toggle Sound: ","<   (Slower)",">   (Faster)"

' --- 3D things: one cube per EGA colour -----------------------------------
FOR c = 1 TO 15
    cube&(c) = _MESHCUBE(1)
    _MESHCOLOR cube&(c), EgaColour~&(c)
NEXT
floor& = _MESHPLANE(27, 4): _MESHCOLOR floor&, _RGB32(60, 66, 100)
lamp& = _LIGHT(1, -0.4, -1, -0.6, _RGB32(255, 250, 240), 0.9)
lamp2& = _LIGHT(1, 0.7, -0.3, 0.5, _RGB32(150, 170, 255), 0.35)
_AMBIENT _RGB32(255, 255, 255), 0.3

pauseTicks = 2: soundOn = -1
' bars of lengths 1..NBARS in random order (like SORTDEMO's Initialize)
FOR i = 1 TO NBARS: backup(i) = i: NEXT
FOR i = NBARS TO 2 STEP -1
    j = INT(RND * i) + 1: SWAP backup(i), backup(j)
NEXT
Reinit

DO
    k$ = UCASE$(INKEY$)
    selection = 0: IF LEN(k$) = 1 THEN selection = INSTR("IBHESQ", k$)
    IF selection > 0 THEN
        Reinit
        FOR i = 1 TO 6: optTime$(i) = "": NEXT
        aborted = 0
        startTime# = TIMER(.001)
        SELECT CASE selection
            CASE 1: InsertionSort
            CASE 2: BubbleSort
            CASE 3: HeapSort
            CASE 4: ExchangeSort
            CASE 5: ShellSort
            CASE 6: QuickSort 1, NBARS
        END SELECT
        hiA = 0: hiB = 0
        ShowTime
        selection = 0
    ELSEIF k$ = "T" THEN
        soundOn = NOT soundOn
    ELSEIF k$ = "<" OR k$ = "," THEN
        IF pauseTicks < 12 THEN pauseTicks = pauseTicks + 1
    ELSEIF k$ = ">" OR k$ = "." THEN
        IF pauseTicks > 0 THEN pauseTicks = pauseTicks - 1
    ELSEIF k$ = CHR$(27) THEN
        EXIT DO
    END IF
    _LIMIT 60
    Render
LOOP
SYSTEM

FUNCTION EgaColour~& (c)
    SELECT CASE c
        CASE 1: EgaColour~& = _RGB32(0, 0, 170)
        CASE 2: EgaColour~& = _RGB32(0, 170, 0)
        CASE 3: EgaColour~& = _RGB32(0, 170, 170)
        CASE 4: EgaColour~& = _RGB32(170, 0, 0)
        CASE 5: EgaColour~& = _RGB32(170, 0, 170)
        CASE 6: EgaColour~& = _RGB32(170, 85, 0)
        CASE 7: EgaColour~& = _RGB32(170, 170, 170)
        CASE 8: EgaColour~& = _RGB32(85, 85, 85)
        CASE 9: EgaColour~& = _RGB32(85, 85, 255)
        CASE 10: EgaColour~& = _RGB32(85, 255, 85)
        CASE 11: EgaColour~& = _RGB32(85, 255, 255)
        CASE 12: EgaColour~& = _RGB32(255, 85, 85)
        CASE 13: EgaColour~& = _RGB32(255, 85, 255)
        CASE 14: EgaColour~& = _RGB32(255, 255, 85)
        CASE ELSE: EgaColour~& = _RGB32(255, 255, 255)
    END SELECT
END FUNCTION

SUB Reinit
    FOR i = 1 TO NBARS: barLen(i) = backup(i): NEXT
    hiA = 0: hiB = 0
END SUB

' ---- drawing ------------------------------------------------------------------
SUB Render
    ' 2D part: background and the menu box on the right
    LINE (0, 0)-(SW - 1, SH - 1), _RGB32(8, 10, 24), BF
    FOR y = 0 TO SH - 1 STEP 4
        c = 10 + y / SH * 40
        LINE (0, y)-(SW - 1, y + 3), _RGB32(c * 0.5, c * 0.6, c * 1.5), BF
    NEXT
    DrawMenu

    ' 3D part: the bars stand on a floor, the camera sways a little
    camT# = camT# + 1 / 60
    yaw = 0.2 * SIN(camT# * 0.35)
    cx = 5.2: cy = 6.2
    _CAMERA (cx + 30 * SIN(yaw), cy + 9, 30 * COS(yaw))TO(cx, cy, 0), 42
    _CLEAR3D 0
    _DRAW3D floor&, 0, -0.05, 0
    FOR i = 1 TO NBARS
        x = (i - (NBARS + 1) / 2) * 0.62
        h = barLen(i) * 0.33
        ' every bar keeps its colour, like in the old SORTDEMO
        _DRAW3D cube&((barLen(i) MOD 15) + 1), x, h / 2, 0, 0, 0, 0, 0.5, h, 0.5
    NEXT
    _DISPLAY
END SUB

SUB DrawMenu
    x = SW - 300: y = 40: w = 270: hh = 380
    LINE (x - 10, y - 10)-(x + w, y + hh), _RGBA32(0, 0, 0, 170), BF
    LINE (x - 10, y - 10)-(x + w, y + hh), _RGB32(255, 255, 255), B
    LINE (x - 6, y - 6)-(x + w - 4, y + hh - 4), _RGB32(255, 255, 255), B
    COLOR _RGB32(255, 255, 255)
    _PRINTSTRING (x + 20, y + 12), "QUICKBASIC SORTING DEMO"
    COLOR _RGB32(255, 255, 85): _PRINTSTRING (x + 106, y + 30), "in 3D"
    FOR i = 1 TO 9
        yy = y + 58 + (i - 1) * 22
        IF i >= 7 THEN yy = yy + 16
        IF i >= 8 THEN yy = yy + 6
        t$ = optTitle$(i)
        IF i = 7 THEN IF soundOn THEN t$ = t$ + "ON" ELSE t$ = t$ + "OFF"
        IF i = selection THEN
            LINE (x + 8, yy - 3)-(x + w - 14, yy + 15), _RGB32(170, 170, 170), BF
            COLOR _RGB32(0, 0, 0)
        ELSE
            COLOR _RGB32(170, 170, 170)
        END IF
        _PRINTSTRING (x + 14, yy), t$
        IF i <= 6 THEN
            IF optTime$(i) <> "" THEN _PRINTSTRING (x + 110, yy), optTime$(i)
        END IF
        IF i = 9 AND pauseTicks = 0 THEN
            COLOR _RGB32(90, 90, 90): _PRINTSTRING (x + 14, yy), t$
        END IF
    NEXT
    COLOR _RGB32(170, 170, 170)
    _PRINTSTRING (x + 14, y + 300), "Type first character of"
    _PRINTSTRING (x + 14, y + 318), "choice ( I B H E S Q T < > )"
    _PRINTSTRING (x + 14, y + 336), "or ESC key to end program:"
    COLOR _RGB32(110, 110, 150)
    _PRINTSTRING (x + 14, y + 360), "Pause:" + STR$(pauseTicks) + " ticks"
END SUB

SUB ShowTime
    IF selection < 1 THEN EXIT SUB
    e# = TIMER(.001) - startTime#
    IF e# < 0 THEN e# = e# + 86400
    ms& = INT(e# * 1000 + 0.5)
    t$ = LTRIM$(STR$(ms& \ 1000)) + "." + RIGHT$("00" + LTRIM$(STR$(ms& MOD 1000)), 3)
    optTime$(selection) = RIGHT$(SPACE$(8) + t$, 8) + " seconds"
    IF aborted THEN optTime$(selection) = "  stopped"
END SUB

' one step of a sort: show it, play the tone and wait like the original
SUB Step2 (a, b)
    hiA = a: hiB = b
    t0# = TIMER(.001)
    IF soundOn AND pauseTicks > 0 THEN SOUND 60 * barLen(a) + 100, pauseTicks / 2
    ShowTime
    Render
    IF pauseTicks > 0 THEN
        DO WHILE TIMER(.001) - t0# < pauseTicks / 18.2 AND TIMER(.001) >= t0#
            _LIMIT 200
        LOOP
    END IF
    k$ = INKEY$
    IF k$ = CHR$(27) THEN aborted = -1
    IF k$ = "<" OR k$ = "," THEN IF pauseTicks < 12 THEN pauseTicks = pauseTicks + 1
    IF k$ = ">" OR k$ = "." THEN IF pauseTicks > 0 THEN pauseTicks = pauseTicks - 1
    IF UCASE$(k$) = "T" THEN soundOn = NOT soundOn
END SUB

SUB SwapBars (a, b)
    SWAP barLen(a), barLen(b)
    Step2 a, b
END SUB

' ---- the six algorithms ------------------------------------------------------
SUB InsertionSort
    FOR r = 2 TO NBARS
        v = barLen(r)
        j = r
        DO WHILE j > 1
            IF barLen(j - 1) <= v THEN EXIT DO
            barLen(j) = barLen(j - 1)
            Step2 j, j - 1
            IF aborted THEN EXIT SUB
            j = j - 1
        LOOP
        barLen(j) = v
        Step2 j, r
        IF aborted THEN EXIT SUB
    NEXT
END SUB

SUB BubbleSort
    limit = NBARS
    DO
        swp = 0
        FOR r = 1 TO limit - 1
            IF barLen(r) > barLen(r + 1) THEN
                SwapBars r, r + 1
                IF aborted THEN EXIT SUB
                swp = r
            END IF
        NEXT
        limit = swp
    LOOP WHILE swp
END SUB

SUB ExchangeSort
    FOR r = 1 TO NBARS - 1
        small = r
        FOR j = r + 1 TO NBARS
            IF barLen(j) < barLen(small) THEN small = j
        NEXT
        IF small > r THEN
            SwapBars r, small
            IF aborted THEN EXIT SUB
        END IF
    NEXT
END SUB

SUB ShellSort
    offset = NBARS \ 2
    DO WHILE offset > 0
        limit = NBARS - offset
        DO
            swp = 0
            FOR r = 1 TO limit
                IF barLen(r) > barLen(r + offset) THEN
                    SwapBars r, r + offset
                    IF aborted THEN EXIT SUB
                    swp = r
                END IF
            NEXT
            limit = swp - offset
        LOOP WHILE swp
        offset = offset \ 2
    LOOP
END SUB

SUB HeapSort
    FOR i = 2 TO NBARS
        PercolateUp i
        IF aborted THEN EXIT SUB
    NEXT
    FOR i = NBARS TO 2 STEP -1
        SwapBars 1, i
        IF aborted THEN EXIT SUB
        PercolateDown i - 1
        IF aborted THEN EXIT SUB
    NEXT
END SUB

SUB PercolateUp (maxLevel)
    i = maxLevel
    DO UNTIL i = 1
        parent = i \ 2
        IF barLen(i) > barLen(parent) THEN
            SwapBars parent, i
            IF aborted THEN EXIT SUB
            i = parent
        ELSE
            EXIT DO
        END IF
    LOOP
END SUB

SUB PercolateDown (maxLevel)
    i = 1
    DO
        child = 2 * i
        IF child > maxLevel THEN EXIT DO
        IF child + 1 <= maxLevel THEN
            IF barLen(child + 1) > barLen(child) THEN child = child + 1
        END IF
        IF barLen(i) < barLen(child) THEN
            SwapBars i, child
            IF aborted THEN EXIT SUB
            i = child
        ELSE
            EXIT DO
        END IF
    LOOP
END SUB

SUB QuickSort (low, high)
    IF low >= high OR aborted THEN EXIT SUB
    IF high - low = 1 THEN
        IF barLen(low) > barLen(high) THEN SwapBars low, high
        EXIT SUB
    END IF
    ' pick a random pivot and move it to the end
    p = INT(RND * (high - low + 1)) + low
    SwapBars high, p
    IF aborted THEN EXIT SUB
    pivot = barLen(high)
    i = low: j = high
    DO
        DO WHILE i < j AND barLen(i) <= pivot
            i = i + 1
        LOOP
        DO WHILE j > i AND barLen(j) >= pivot
            j = j - 1
        LOOP
        IF i < j THEN
            SwapBars i, j
            IF aborted THEN EXIT SUB
        END IF
    LOOP WHILE i < j
    SwapBars i, high
    IF aborted THEN EXIT SUB
    ' the smaller part first, like the original
    IF i - low < high - i THEN
        QuickSort low, i - 1
        QuickSort i + 1, high
    ELSE
        QuickSort i + 1, high
        QuickSort low, i - 1
    END IF
END SUB
