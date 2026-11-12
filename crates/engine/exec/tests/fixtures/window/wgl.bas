SCREEN _NEWIMAGE(320, 200, 32)
DO
    _LIMIT 30
LOOP

SUB _GL
    _glClearColor 0, 0, 1, 1
    _glClear _GL_COLOR_BUFFER_BIT
    _glBegin _GL_TRIANGLES
    _glColor3f 1, 0, 0
    _glVertex2f -0.5, -0.5
    _glVertex2f 0.5, -0.5
    _glVertex2f 0, 0.5
    _glEnd
END SUB
