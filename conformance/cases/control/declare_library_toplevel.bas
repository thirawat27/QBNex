$CONSOLE:ONLY
' Main-module code before and after a DECLARE LIBRARY block must run in
' source order and share the same variables (legacy defect L-00).
x = 1
PRINT "before"; x
DECLARE LIBRARY
    FUNCTION getpid& ()
END DECLARE
PRINT "after"; x
y = x + 1
PRINT "y"; y
SYSTEM
