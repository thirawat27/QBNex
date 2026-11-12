$CONSOLE:ONLY
' CHAIN inside a SUB passes the module-level COMMON variables and arrays, also those that
' are not SHARED (the SUB's own n% is another variable); chain_sub_child.bas prints them.
' No TYPE or STRING * n here, so the program stays out of memory mode.
DIM v(1 TO 3) AS LONG
COMMON n%, total#, s$, big&&, sm!, ub~%%, sb%%
COMMON v() AS LONG, names$(), m#()
n% = -7: total# = 2.5: s$ = "text": big&& = 123456789012: sm! = 1.25: ub~%% = 200: sb%% = -100
v(1) = 10: v(2) = -20: v(3) = 30
REDIM names$(-1 TO 1): names$(-1) = "one": names$(1) = "three"
REDIM m#(2, 1 TO 2): m#(2, 2) = 1.5: m#(1, 1) = -3
GoOn
PRINT "not reached"

SUB GoOn
    DIM n%
    n% = 99
    CHAIN "chain_sub_child"
END SUB
