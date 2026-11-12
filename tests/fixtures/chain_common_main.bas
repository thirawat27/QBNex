$CONSOLE:ONLY
' CHAIN passes the COMMON variables and arrays by position (chain_common_child.bas prints them)
TYPE Pt
    x AS INTEGER
    y AS INTEGER
END TYPE
DIM v(1 TO 3) AS INTEGER, pts(1) AS Pt, fs(1) AS STRING * 3
COMMON n%, total#, s$, f AS STRING * 4, p AS Pt
COMMON v() AS INTEGER, pts() AS Pt, fs() AS STRING * 3, names$(), m#()
n% = 7: total# = 2.5: s$ = "text": f = "abcd": p.x = 3: p.y = -4
v(1) = 10: v(2) = 20: v(3) = 30: pts(1).x = 5: pts(1).y = 6: fs(0) = "abc": fs(1) = "xyz"
REDIM names$(-1 TO 1): names$(-1) = "one": names$(1) = "three"
REDIM m#(2, 1 TO 2): m#(2, 2) = 1.5
CHAIN "chain_common_child"
PRINT "not reached"
