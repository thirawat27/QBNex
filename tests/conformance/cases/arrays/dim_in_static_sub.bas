$CONSOLE:ONLY
' DIM inside a SUB ... STATIC runs again on the second call: the array keeps
' its values and is not dimensioned twice (no "Duplicate definition").
S
S
PRINT "ok"
SYSTEM
SUB S STATIC
DIM E(7): E(1) = E(1) + 1: PRINT E(1);
END SUB
