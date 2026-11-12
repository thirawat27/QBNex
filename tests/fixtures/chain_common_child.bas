$CONSOLE:ONLY
' run by chain_common_main.bas: its COMMON values and arrays arrive here in the same order
TYPE Pt
    x AS INTEGER
    y AS INTEGER
END TYPE
DIM w(1 TO 3) AS INTEGER, r(1) AS Pt, g(1) AS STRING * 3
COMMON a%, b#, c$, d AS STRING * 4, q AS Pt
COMMON w() AS INTEGER, r() AS Pt, g() AS STRING * 3, t$(), z#()
PRINT a%; b#; c$; " "; d; q.x; q.y
PRINT w(1); w(2); w(3); r(1).x; r(1).y; " "; g(0); g(1)
PRINT LBOUND(t$); UBOUND(t$); " "; t$(-1); "|"; t$(0); "|"; t$(1)
PRINT UBOUND(z#, 1); LBOUND(z#, 2); UBOUND(z#, 2); z#(2, 2)
SYSTEM
