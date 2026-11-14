$CONSOLE:ONLY
' EXIT FOR leaves a FOR EACH over an array of two dimensions altogether.
DIM g(1 TO 2, 1 TO 3) AS INTEGER
FOR i = 1 TO 2: FOR j = 1 TO 3: g(i, j) = i * 10 + j: NEXT j, i
FOR EACH v IN g()
    IF v = 12 THEN EXIT FOR
    PRINT v;
NEXT
PRINT "| left the loop"
n = 0
FOR EACH v IN g()
    n = n + 1
NEXT
PRINT n
SYSTEM
