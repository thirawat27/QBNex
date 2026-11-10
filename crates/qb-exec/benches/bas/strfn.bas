$CONSOLE:ONLY
n& = 2000000
t# = TIMER(.001)
FOR i& = 1 TO n&: b$ = MID$("hello world", i& MOD 5 + 1, 3) + CHR$(65): k& = k& + LEN(b$): NEXT
e# = TIMER(.001) - t#
PRINT k&; INT(e# * 1000)
SYSTEM

