$CONSOLE:ONLY
TYPE Rec
    n AS LONG
    s AS STRING * 6
END TYPE
f$ = "conf_bin_tmp.dat"
OPEN f$ FOR BINARY AS #1
v& = 305419896
PUT #1, , v&
w$ = "ABCD"
PUT #1, , w$
PRINT LOF(1); LOC(1)
GET #1, 1, r&
PRINT HEX$(r&)
SEEK #1, 5
x$ = SPACE$(2): GET #1, , x$
PRINT x$
CLOSE #1
KILL f$
DIM rec AS Rec
OPEN f$ FOR RANDOM AS #2 LEN = LEN(rec)
FOR i = 1 TO 3: rec.n = i * 100: rec.s = "r" + STR$(i): PUT #2, i, rec: NEXT
GET #2, 2, rec
PRINT rec.n; "["; rec.s; "]"; LOF(2)
CLOSE #2
KILL f$
SYSTEM
