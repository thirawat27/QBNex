$CONSOLE:ONLY
' RANDOM files: FIELD + LSET/RSET + GET/PUT without a variable, variable-length
' string records with their length header, LOC/SEEK/LOF in records
f$ = "conf_rnd_tmp.dat"
OPEN f$ FOR RANDOM AS #1 LEN = 12
FIELD #1, 4 AS a$, 8 AS b$
LSET a$ = "ab"
RSET b$ = "xyz"
PUT #1, 2
LSET a$ = "toolongvalue"
RSET b$ = "123456789"
PUT #1
PRINT LOF(1); LOC(1); SEEK(1)
GET #1, 2
PRINT "["; a$; "]["; b$; "]"
GET #1
PRINT "["; a$; "]["; b$; "]"; LOC(1)
GET #1, 1
PRINT ASC(a$); ASC(b$)
CLOSE #1
OPEN f$ FOR RANDOM AS #1 LEN = 10
v$ = "hi there"
PUT #1, 1, v$
w$ = ""
PUT #1, 2, w$
GET #1, 1, r$
PRINT "["; r$; "]"; LEN(r$)
GET #1, 2, r$
PRINT "["; r$; "]"; LEN(r$)
GET #1, 9, r$
PRINT "["; r$; "]"; LOC(1)
n% = 513: PUT #1, 4, n%
GET #1, 4, m%
PRINT m%; LOF(1)
SEEK #1, 2
PRINT SEEK(1); LOC(1)
CLOSE #1
OPEN f$ FOR BINARY AS #1
s$ = SPACE$(12): GET #1, 1, s$
FOR i = 1 TO 12: PRINT ASC(MID$(s$, i, 1));: NEXT
PRINT
CLOSE #1
KILL f$
SYSTEM
