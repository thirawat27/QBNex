OPEN "qb_native_io.txt" FOR OUTPUT AS #1
FOR i = 1 TO 5
    PRINT #1, "line"; i; i * i
NEXT
CLOSE #1
OPEN "qb_native_io.txt" FOR APPEND AS #1
WRITE #1, "tail", 3.5
CLOSE #1
OPEN "qb_native_io.txt" FOR INPUT AS #1
n = 0
DO WHILE NOT EOF(1)
    LINE INPUT #1, l$
    n = n + 1
    PRINT n; l$
LOOP
CLOSE #1
PRINT "bytes"; LEN(l$)
KILL "qb_native_io.txt"
