$CONSOLE:ONLY
' An array used without DIM is name(10) from the OPTION BASE on.
OPTION BASE 1
ON ERROR GOTO h
c(5) = 3
PRINT LBOUND(c); UBOUND(c)
c(0) = 7
PRINT "end"
SYSTEM
h:
PRINT "error"; ERR
RESUME NEXT
