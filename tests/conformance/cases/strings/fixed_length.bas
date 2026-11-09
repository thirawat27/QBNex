$CONSOLE:ONLY
DIM f AS STRING * 5
f = "ab"
PRINT "["; f; "]"; LEN(f)
f = "abcdefgh"
PRINT "["; f; "]"
DIM l AS STRING * 6
LSET l = "xy": PRINT "["; l; "]"
RSET l = "xy": PRINT "["; l; "]"
SYSTEM
