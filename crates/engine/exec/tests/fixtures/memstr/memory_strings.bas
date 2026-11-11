$CONSOLE:ONLY
DIM SHARED g$
DIM a$(3)
FOR i = 1 TO 40: a$(1) = a$(1) + "ab": NEXT
a$(2) = a$(1): a$(1) = a$(1)
PRINT LEN(a$(1)); a$(1) = a$(2); INSTR(a$(1), "ba")
t
t
g$ = "xy": g$ = g$ + g$: g$ = g$
PRINT g$
SUB t
    STATIC k$
    FOR j = 1 TO 3: k$ = k$ + "z": NEXT
    k$ = k$
    PRINT k$; LEN(k$)
END SUB