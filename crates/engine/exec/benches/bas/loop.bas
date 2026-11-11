$CONSOLE:ONLY
n& = 50000000
t# = TIMER(.001)
FOR i& = 1 TO n&: s& = s& + i& MOD 7: NEXT
e# = TIMER(.001) - t#
PRINT s&; INT(e# * 1000)
SYSTEM

