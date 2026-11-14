' A real window (window_smoke.sh): draws, says it is ready, then waits for Esc or 10 s.
SCREEN _NEWIMAGE(320, 200, 32)
_TITLE "QBNex window smoke"
CLS , _RGB32(0, 0, 128)
LINE (10, 10)-(110, 60), _RGB32(255, 0, 0), BF
CIRCLE (200, 100), 40, _RGB32(0, 255, 0)
PAINT (200, 100), _RGB32(0, 255, 0)
COLOR _RGB32(255, 255, 255)
LOCATE 10, 2: PRINT "Hello QBNex"
_DISPLAY
OPEN COMMAND$ + ".ready" FOR OUTPUT AS #1: PRINT #1, _WIDTH; _HEIGHT: CLOSE #1
t! = TIMER
DO: _LIMIT 30: k$ = INKEY$: LOOP UNTIL k$ = CHR$(27) OR ABS(TIMER - t!) > 10
OPEN COMMAND$ + ".done" FOR OUTPUT AS #1: PRINT #1, k$ = CHR$(27): CLOSE #1
SYSTEM
