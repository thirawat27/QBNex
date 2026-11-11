$SCREENHIDE
$CONSOLE
' SUB _GL runs at display frames with an OpenGL context (headless: an
' offscreen framebuffer of the screen's size). It draws the software screen
' behind, then the program's GL calls; _glReadPixels reads them back.
' Needs an OpenGL driver with framebuffer objects (GL 3.0).
DIM SHARED frames AS LONG, px AS _UNSIGNED LONG, bg AS _UNSIGNED LONG, vp(3) AS LONG
SCREEN _NEWIMAGE(160, 100, 32)
CLS , _RGB32(0, 255, 0)
t! = TIMER
DO: _LIMIT 60: LOOP UNTIL frames >= 3 OR TIMER - t! > 5
_DEST _CONSOLE
PRINT "frames>=3:"; frames >= 3
PRINT "viewport:"; vp(2); vp(3)
PRINT "behind: "; HEX$(bg AND &HFFFFFF)
PRINT "drawn: "; HEX$(px AND &HFFFFFF)
SYSTEM

SUB _GL
    _glGetIntegerv _GL_VIEWPORT, _OFFSET(vp(0))
    _glReadPixels 80, 50, 1, 1, _GL_BGRA_EXT, _GL_UNSIGNED_BYTE, _OFFSET(bg)
    _glEnable _GL_SCISSOR_TEST
    _glScissor 0, 0, 40, 100
    _glClearColor 0, 0, 1, 1
    _glClear _GL_COLOR_BUFFER_BIT
    _glDisable _GL_SCISSOR_TEST
    _glReadPixels 10, 50, 1, 1, _GL_BGRA_EXT, _GL_UNSIGNED_BYTE, _OFFSET(px)
    frames = frames + 1
END SUB
