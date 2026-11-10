$CONSOLE:ONLY
DIM g(1 TO 2, 0 TO 2) AS INTEGER
FOR r = 1 TO 2: FOR c = 0 TO 2: g(r, c) = r * 10 + c: NEXT c, r
FOR EACH v IN g()
    PRINT v;
NEXT v
PRINT
REDIM h$(1, 1)
h$(0, 0) = "a": h$(1, 1) = "d"
FOR EACH s$ IN h$(): PRINT "["; s$; "]";: NEXT
PRINT
SYSTEM
