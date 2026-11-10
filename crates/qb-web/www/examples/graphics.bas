SCREEN 13
RANDOMIZE TIMER
DIM x(20), y(20), dx(20), dy(20), c(20)
FOR i = 1 TO 20
    x(i) = RND * 300 + 10: y(i) = RND * 170 + 20
    dx(i) = RND * 4 - 2: dy(i) = RND * 4 - 2: c(i) = 32 + i * 3
NEXT
FOR frame = 1 TO 900
    LINE (0, 0)-(319, 199), 0, BF
    FOR i = 1 TO 20
        x(i) = x(i) + dx(i): y(i) = y(i) + dy(i)
        IF x(i) < 6 OR x(i) > 313 THEN dx(i) = -dx(i)
        IF y(i) < 14 OR y(i) > 193 THEN dy(i) = -dy(i)
        CIRCLE (x(i), y(i)), 5, c(i)
        PAINT (x(i), y(i)), c(i), c(i)
    NEXT
    LOCATE 1, 1: PRINT "SCREEN 13 - press a key to stop"
    _DISPLAY
    _LIMIT 60
    IF INKEY$ <> "" THEN EXIT FOR
NEXT
SOUND 440, 3
