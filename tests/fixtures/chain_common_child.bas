$CONSOLE:ONLY
' run by chain_common_main.bas: its COMMON values arrive here in the same order
TYPE Pt
    x AS INTEGER
    y AS INTEGER
END TYPE
COMMON a%, b#, c$, d AS STRING * 4, q AS Pt
PRINT a%; b#; c$; " "; d; q.x; q.y
SYSTEM
