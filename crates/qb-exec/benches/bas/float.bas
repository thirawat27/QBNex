$CONSOLE:ONLY
n& = 20000000
t# = TIMER(.001)
FOR i& = 1 TO n&: x# = x# + i& * 0.5: NEXT
e# = TIMER(.001) - t#
PRINT x#; INT(e# * 1000)
SYSTEM

