$CONSOLE:ONLY
' Module code after a SUB is module code: it has the module's variables and arrays.
DIM arr(3) AS INTEGER
x = 5
arr(2) = 9
show
SUB show
    PRINT "show"
END SUB
PRINT "x ="; x; arr(2)
SYSTEM
