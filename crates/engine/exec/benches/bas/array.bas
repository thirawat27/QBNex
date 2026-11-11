$CONSOLE:ONLY
n& = 20000000: DIM a&(999)
t# = TIMER(.001)
FOR i& = 1 TO n&: j& = i& MOD 1000: a&(j&) = a&(j&) + i&: NEXT
e# = TIMER(.001) - t#
PRINT a&(7); INT(e# * 1000)
SYSTEM

