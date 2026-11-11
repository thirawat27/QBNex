$CONSOLE:ONLY
n& = 5000000
t# = TIMER(.001)
FOR i& = 1 TO n&: t& = t& + sq&(i&): NEXT
e# = TIMER(.001) - t#
PRINT t&; INT(e# * 1000)
SYSTEM
FUNCTION sq& (x&)
sq& = x& * x& MOD 1000
END FUNCTION
