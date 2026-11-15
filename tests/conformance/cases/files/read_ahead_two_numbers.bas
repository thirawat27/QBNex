$CONSOLE:ONLY
' LINE INPUT # and INPUT # read the file ahead of their position in blocks;
' what another file number writes into the same file is read all the same,
' and SEEK, EOF and the end of the file behave as with single bytes.
OPEN "ra_t.txt" FOR OUTPUT AS #1
FOR i = 1 TO 3000: PRINT #1, "line"; i: NEXT
CLOSE #1
OPEN "ra_t.txt" FOR INPUT AS #1
OPEN "ra_t.txt" FOR BINARY AS #2
LINE INPUT #1, a$: PRINT a$
w$ = "LINE": PUT #2, SEEK(1), w$
LINE INPUT #1, a$: PRINT a$
INPUT #1, b$: PRINT b$
SEEK #1, 1
LINE INPUT #1, a$: PRINT a$
n = 0
DO WHILE NOT EOF(1)
    LINE INPUT #1, a$
    n = n + 1
LOOP
PRINT n; a$
CLOSE
KILL "ra_t.txt"
SYSTEM
