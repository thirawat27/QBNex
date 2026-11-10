$CONSOLE:ONLY
DIM a(1 TO 4) AS INTEGER
FOR i = 1 TO 4: a(i) = i * i: NEXT
total = 0
FOR EACH v IN a()
    total = total + v
NEXT v
PRINT total
DIM n$(2)
n$(0) = "x": n$(1) = "y": n$(2) = "z"
FOR EACH s$ IN n$()
    FOR k = 1 TO 2
        PRINT s$;
    NEXT k
NEXT
PRINT
SYSTEM
