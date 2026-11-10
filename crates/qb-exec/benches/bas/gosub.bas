$CONSOLE:ONLY
n& = 5000000
t# = TIMER(.001)
FOR i& = 1 TO n&: GOSUB g: NEXT
e# = TIMER(.001) - t#
PRINT c&; INT(e# * 1000)
SYSTEM
g:
c& = c& + 1: RETURN
