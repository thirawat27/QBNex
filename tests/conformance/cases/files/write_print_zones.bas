$CONSOLE:ONLY
' WRITE #, PRINT # with ; , and TAB-like zones, then the raw bytes
f$ = "conf_wpz_tmp.txt"
OPEN f$ FOR OUTPUT AS #1
WRITE #1, "q,uote", 1.5, -2, "", 1E+20
PRINT #1, "a", "bb"; 7, -8
PRINT #1, 1; 2;
PRINT #1, "end"
PRINT #1,
WRITE #1,
CLOSE #1
OPEN f$ FOR BINARY AS #1
s$ = SPACE$(LOF(1))
GET #1, , s$
CLOSE #1
FOR i = 1 TO LEN(s$)
    c = ASC(MID$(s$, i, 1))
    IF c = 13 THEN
        PRINT "<CR>";
    ELSEIF c = 10 THEN
        PRINT "<LF>"
    ELSEIF c = 32 THEN
        PRINT "_";
    ELSE
        PRINT MID$(s$, i, 1);
    END IF
NEXT
OPEN f$ FOR APPEND AS #1
PRINT #1, "app"
PRINT LOC(1); LOF(1)
CLOSE #1
OPEN f$ FOR INPUT AS #1
INPUT #1, a$, b, c, d$, e#
PRINT a$; b; c; "["; d$; "]"; e#
CLOSE #1
KILL f$
SYSTEM
