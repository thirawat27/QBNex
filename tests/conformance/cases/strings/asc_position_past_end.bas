$CONSOLE:ONLY
' ASC(s$, position): a position past the end is an illegal function call (it gave -1)
ON ERROR GOTO h
a$ = "abc"
PRINT ASC(a$, 1); ASC(a$, 3)
PRINT ASC(a$, 4)
PRINT ASC(a$, 0)
PRINT ASC("", 1)
PRINT "done"
SYSTEM
h:
PRINT "error"; ERR
RESUME NEXT
