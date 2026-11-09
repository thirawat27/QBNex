$CONSOLE
_DEST _CONSOLE
PRINT "start"
t! = TIMER
DO: _LIMIT 30: LOOP UNTIL TIMER - t! > 1
PRINT "end"
SYSTEM

SUB _GL
    _glClearColor 0, 0, 1, 1
    _glClear _GL_COLOR_BUFFER_BIT
END SUB
