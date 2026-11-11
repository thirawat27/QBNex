$CONSOLE:ONLY
n& = 5000000
t# = TIMER(.001)
FOR i& = 1 TO n&: inc c&: NEXT
e# = TIMER(.001) - t#
PRINT c&; INT(e# * 1000)
SYSTEM
SUB inc (n&)
n& = n& + 1
END SUB
