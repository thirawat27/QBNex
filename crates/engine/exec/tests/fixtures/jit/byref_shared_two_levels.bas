DIM SHARED g&
FOR i% = 1 TO 100
    first
NEXT
PRINT g&
SUB first
    second g&
END SUB
SUB second (x&)
    x& = x& * 3 MOD 1000 + 1
END SUB
