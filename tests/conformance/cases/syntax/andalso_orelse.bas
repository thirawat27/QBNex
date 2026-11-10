$CONSOLE:ONLY
DIM a(1 TO 3) AS INTEGER
i = 4
IF i <= 3 _ANDALSO a(i) = 0 THEN PRINT "bad" ELSE PRINT "skipped index 4"
IF i > 3 _ORELSE a(i) = 0 THEN PRINT "short-circuit or"
PRINT (1 _ANDALSO 2); (0 _ORELSE 0); (0 _ORELSE 5)
CONST both = 3 _ANDALSO 4
PRINT both
SYSTEM
