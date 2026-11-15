$CONSOLE:ONLY
' A SUB written on one line, followed by module code
a
SUB a: PRINT "a": END SUB
PRINT "after"; f(2)
FUNCTION f (n): f = n * 2: END FUNCTION
PRINT "end"
SYSTEM
