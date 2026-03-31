DIM a$ AS STRING
OPEN "field.dat" FOR RANDOM AS #1 LEN = 4
FIELD #1, 4 AS a$
LSET a$ = "XY"
PUT #1, 1
LSET a$ = ""
GET #1, 1
CLOSE #1
KILL "field.dat"

PRINT LEFT$(a$, 2)
