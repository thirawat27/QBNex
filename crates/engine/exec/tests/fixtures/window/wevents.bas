' Events of the window: a new size, focus, a dropped file, the keyboard and the
' mouse as devices, and mouse pipes. Run headless with a script that sends them.
$RESIZE:ON
SCREEN _NEWIMAGE(320, 200, 32)
OPEN "events.txt" FOR OUTPUT AS #1
ON ERROR GOTO oops
_ACCEPTFILEDROP
PRINT #1, "drop"; _ACCEPTFILEDROP; _TOTALDROPPEDFILES
PRINT #1, "devices"; _DEVICES; _DEVICE$(1); " "; _DEVICE$(2)
PRINT #1, "last"; _LASTBUTTON(1); _LASTBUTTON(2); _LASTAXIS(2); _LASTWHEEL(2)
PRINT #1, "handle"; _WINDOWHANDLE
_FPS 60
_FPS _AUTO
_FPS 0
_MOUSESHOW "link"
_MOUSESHOW "nonsense"
_MOUSEHIDE
' the first frames give the window its size: that is not a resize by the user
_DISPLAY
_DELAY .3
x = _RESIZE
_AUTODISPLAY
focus = -1
t! = TIMER
DO
    IF _RESIZE THEN PRINT #1, "resize"; _RESIZEWIDTH; _RESIZEHEIGHT
    IF _WINDOWHASFOCUS <> focus THEN focus = _WINDOWHASFOCUS: PRINT #1, "focus"; focus
    DO WHILE _TOTALDROPPEDFILES
        PRINT #1, "file "; _DROPPEDFILE$
    LOOP
    d = _DEVICEINPUT
    DO WHILE d
        IF d = 1 THEN
            ' scan code 31 is the S key
            IF _BUTTONCHANGE(32) THEN PRINT #1, "key"; _BUTTON(32); _BUTTONCHANGE(32)
        END IF
        IF d = 2 THEN
            IF _BUTTONCHANGE(1) THEN PRINT #1, "button"; _BUTTON(1); _BUTTONCHANGE(1)
            IF _WHEEL(3) <> 0 THEN PRINT #1, "wheel"; _WHEEL(3)
            IF _AXIS(1) <> ax! OR _AXIS(2) <> ay! THEN ax! = _AXIS(1): ay! = _AXIS(2): PRINT #1, "axis"; ax!; ay!
        END IF
        d = _DEVICEINPUT
    LOOP
    k$ = INKEY$
    _LIMIT 200
LOOP UNTIL k$ = CHR$(27) OR TIMER - t! > 15
PRINT #1, "select"; _DEVICEINPUT(2); _DEVICE$
' a pipe gets the mouse messages first and hands on the ones it chooses
WHILE _MOUSEINPUT: WEND
p = _MOUSEPIPEOPEN
_MOUSEMOVE 10, 20
_DELAY .2
n = 0
WHILE _MOUSEINPUT: n = n + 1: WEND
PRINT #1, "default"; n
n = 0
WHILE _MOUSEINPUT(p)
    n = n + 1
    IF n = 1 THEN mx = _MOUSEMOVEMENTX(p): my = _MOUSEMOVEMENTY(p)
    _MOUSEINPUTPIPE p
WEND
PRINT #1, "pipe"; n; _MOUSEX(p); _MOUSEY(p); mx; my
n = 0
WHILE _MOUSEINPUT: n = n + 1: WEND
PRINT #1, "handed on"; n; _MOUSEX; _MOUSEY
_MOUSEPIPECLOSE p
_MOUSEMOVE 30, 40
_DELAY .2
WHILE _MOUSEINPUT: WEND
PRINT #1, "closed"; _MOUSEX; _MOUSEY
x = _MOUSEX(p)
_MOUSEMOVE 320, 0
_RESIZE OFF
PRINT #1, "off"; _RESIZE
CLOSE #1
SYSTEM
oops:
PRINT #1, "error"; ERR
RESUME NEXT
