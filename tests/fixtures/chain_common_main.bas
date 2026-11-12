$CONSOLE:ONLY
' CHAIN passes the COMMON variables by position (chain_common_child.bas prints them)
TYPE Pt
    x AS INTEGER
    y AS INTEGER
END TYPE
COMMON n%, total#, s$, f AS STRING * 4, p AS Pt
n% = 7: total# = 2.5: s$ = "text": f = "abcd": p.x = 3: p.y = -4
CHAIN "chain_common_child"
PRINT "not reached"
