$CONSOLE:ONLY
n& = 2000000
t# = TIMER(.001)
FOR i& = 1 TO n&: a$ = a$ + "x": NEXT
e# = TIMER(.001) - t#
PRINT LEN(a$); INT(e# * 1000)
SYSTEM

