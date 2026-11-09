$CONSOLE:ONLY
' BINARY: GET/PUT of numbers and strings, past-EOF reads, LOF/LOC/SEEK/EOF, INPUT$
f$ = "conf_binops_tmp.dat"
OPEN f$ FOR BINARY AS #1
PRINT LOF(1); LOC(1); SEEK(1); EOF(1)
a% = -2: PUT #1, , a%
b& = 70000: PUT #1, 10, b&
s$ = "Hello": PUT #1, , s$
PRINT LOF(1); LOC(1); SEEK(1)
SEEK #1, 1
GET #1, , a%
PRINT a%; SEEK(1)
t$ = SPACE$(3): GET #1, 3, t$
PRINT LEN(t$); ASC(t$); ASC(MID$(t$, 3))
GET #1, 10, b&
PRINT b&
u$ = SPACE$(8): GET #1, 14, u$
PRINT "["; LEFT$(u$, 5); "]"; ASC(MID$(u$, 6)); EOF(1); LOC(1)
SEEK #1, 3
PRINT INPUT$(4, #1); "|"; INPUT$(30, #1); "|"; SEEK(1)
SEEK #1, 100
PRINT LOC(1); EOF(1); LOF(1)
x% = 7: GET #1, , x%
PRINT x%; EOF(1); LOF(1)
CLOSE #1
PRINT FREEFILE
OPEN f$ FOR BINARY AS #3
LINE INPUT #3, l$
PRINT LEN(l$)
CLOSE #3
KILL f$
' LINE INPUT # on BINARY
OPEN f$ FOR OUTPUT AS #1: PRINT #1, "one": PRINT #1, "two"; CHR$(13); CHR$(10); "x"; : CLOSE #1
OPEN f$ FOR BINARY AS #1
DO UNTIL EOF(1)
    LINE INPUT #1, l$
    PRINT "["; l$; "]"; SEEK(1)
LOOP
CLOSE #1
KILL f$
SYSTEM
