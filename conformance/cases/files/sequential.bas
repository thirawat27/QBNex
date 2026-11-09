$CONSOLE:ONLY
f$ = "conf_seq_tmp.txt"
OPEN f$ FOR OUTPUT AS #1
PRINT #1, "line one"
PRINT #1, 1; 2; 3
WRITE #1, "quoted", 4.5, -6
CLOSE #1
OPEN f$ FOR APPEND AS #1
PRINT #1, "appended"
CLOSE #1
OPEN f$ FOR INPUT AS #1
DO UNTIL EOF(1)
    LINE INPUT #1, l$
    PRINT "["; l$; "]"
LOOP
CLOSE #1
OPEN f$ FOR INPUT AS #1
LINE INPUT #1, l$
INPUT #1, a, b, c
INPUT #1, q$, r, t
PRINT a + b + c; q$; r; t
CLOSE #1
KILL f$
PRINT _FILEEXISTS(f$)
SYSTEM
