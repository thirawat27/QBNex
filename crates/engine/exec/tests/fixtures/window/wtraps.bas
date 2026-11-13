' Joystick button traps (ON STRIG) and ALT+ENTER. A script presses the buttons of
' two scripted pads and the keys; a letter key ends each step.
DIM SHARED noted AS LONG
SCREEN 13
OPEN "traps.txt" FOR OUTPUT AS #1
ON STRIG(0) GOSUB fire
ON STRIG(4) GOSUB second
ON STRIG(0, 2) note 7
STRIG(0) ON
STRIG(4) ON
STRIG(0, 2) ON
want$ = "a": GOSUB waitkey
PRINT #1, "on"; fired; seconds; noted
' stopped: the presses wait, and one of them is kept
STRIG(0) STOP
want$ = "b": GOSUB waitkey
PRINT #1, "stopped"; fired
STRIG(0) ON
t! = TIMER
DO: _LIMIT 200: LOOP UNTIL TIMER - t! > .4
PRINT #1, "on again"; fired
STRIG(0) OFF
want$ = "c": GOSUB waitkey
PRINT #1, "off"; fired; STRIG(1); STRIG(0)
' ALT+ENTER: every mode in turn, then only what _ALLOWFULLSCREEN leaves
want$ = "d": GOSUB waitkey
PRINT #1, "all"; _FULLSCREEN; _FULLSCREENSMOOTH
_ALLOWFULLSCREEN _SQUAREPIXELS , _SMOOTH
want$ = "e": GOSUB waitkey
PRINT #1, "square smooth"; _FULLSCREEN; _FULLSCREENSMOOTH
want$ = "f": GOSUB waitkey
PRINT #1, "window"; _FULLSCREEN; _FULLSCREENSMOOTH
_ALLOWFULLSCREEN _OFF
want$ = "g": GOSUB waitkey
PRINT #1, "not allowed"; _FULLSCREEN; enters
CLOSE #1
SYSTEM

waitkey:
t! = TIMER
DO
    k$ = INKEY$
    IF k$ = CHR$(13) OR k$ = CHR$(0) + CHR$(28) THEN enters = enters + 1
    _LIMIT 200
LOOP UNTIL k$ = want$ OR TIMER - t! > 10
RETURN

fire:
fired = fired + 1
RETURN

second:
seconds = seconds + 1
RETURN

SUB note (n AS LONG)
    noted = noted + n
END SUB
