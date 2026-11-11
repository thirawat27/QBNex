$CONSOLE:ONLY
' A line label with the name of a SUB: "title:" is a label (not a call)
' once a GOTO / ON ERROR GOTO refers to it (the compiler's repass).
DECLARE SUB title ()
DECLARE SUB oops ()
ON ERROR GOTO oops
title:
n = n + 1
CALL title
IF n < 2 THEN GOTO title
PRINT "n ="; n
ERROR 5
PRINT "after"
SYSTEM
oops:
PRINT "handled"; ERR
RESUME NEXT

SUB title
PRINT "t"
END SUB

SUB oops
PRINT "sub oops"
END SUB
