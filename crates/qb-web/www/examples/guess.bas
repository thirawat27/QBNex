RANDOMIZE TIMER
COLOR 11: PRINT "Guess the number between 1 and 100": COLOR 7
target = INT(RND * 100) + 1
tries = 0
DO
    INPUT "Your guess"; g
    tries = tries + 1
    IF g < target THEN PRINT "Too low": SOUND 300, 2
    IF g > target THEN PRINT "Too high": SOUND 600, 2
LOOP UNTIL g = target
COLOR 10: PRINT "Got it in"; tries; "tries!": COLOR 7
PLAY "T180 L8 C E G > C"
