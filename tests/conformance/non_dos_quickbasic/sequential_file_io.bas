DIM first$ AS STRING
DIM second$ AS STRING
OPEN "sample.txt" FOR OUTPUT AS #1
PRINT #1, "line-one"
PRINT #1, "line-two"
CLOSE #1

OPEN "sample.txt" FOR INPUT AS #1
LINE INPUT #1, first$
LINE INPUT #1, second$
CLOSE #1

KILL "sample.txt"

PRINT first$
PRINT second$
PRINT "done"
