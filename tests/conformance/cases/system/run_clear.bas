$CONSOLE:ONLY
' CLEAR: variables back to 0 / "", DATA from the start, files closed.
' RUN [label]: the program starts over with fresh variables (also from a SUB);
' the runs in between report through a file.
DECLARE SUB again ()
DIM a(5)
x = x + 1
PRINT "start"; x
IF _FILEEXISTS("run_clear.tmp") THEN GOTO second
a(3) = 7: y = 5: s$ = "abc"
READ d
OPEN "run_clear.tmp" FOR OUTPUT AS #1
CLEAR
PRINT y; LEN(s$); a(3); d
READ d: PRINT d
x = 41
RUN later
second:
OPEN "run_clear.tmp" FOR INPUT AS #1
DO UNTIL EOF(1): LINE INPUT #1, l$: PRINT l$: LOOP
CLOSE #1
KILL "run_clear.tmp"
PRINT "end"; x; y
SYSTEM
later:
OPEN "run_clear.tmp" FOR APPEND AS #2: PRINT #2, "later"; x: CLOSE #2
again
DATA 11, 22

SUB again
OPEN "run_clear.tmp" FOR APPEND AS #2: PRINT #2, "again": CLOSE #2
RUN
END SUB
