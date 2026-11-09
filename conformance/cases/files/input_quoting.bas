$CONSOLE:ONLY
' INPUT # item rules: quotes, commas, leading/trailing blanks, CR/LF pairs, CHR$(26)
f$ = "conf_inq_tmp.txt"
OPEN f$ FOR OUTPUT AS #1
PRINT #1, "  plain text  ,"; CHR$(34); "a, quoted"; CHR$(34); " tail,next"
PRINT #1, "x"; CHR$(9); "y"; CHR$(13); "lf-only"; CHR$(10); "z"
PRINT #1, " 12 , -3.5E2,&HFF,&O17,&B101, .5D1 ,1e,abc"
PRINT #1, "after"; CHR$(26); "hidden"
CLOSE #1
OPEN f$ FOR INPUT AS #1
FOR i = 1 TO 6
    INPUT #1, a$
    PRINT "["; a$; "]"
NEXT
FOR i = 1 TO 8
    INPUT #1, v#
    PRINT v#;
NEXT
PRINT
INPUT #1, a$
PRINT "["; a$; "]"; EOF(1)
CLOSE #1
OPEN f$ FOR INPUT AS #1
LINE INPUT #1, l$
PRINT LEN(l$); "["; l$; "]"
PRINT LOC(1); SEEK(1); LOF(1)
CLOSE #1
KILL f$
SYSTEM
