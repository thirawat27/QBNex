DIM i AS INTEGER
FOR i = 1 TO 3
    CALL Bump
NEXT i

FOR i = 1 TO 3
    PRINT NextCount#
NEXT i

SUB Bump STATIC
    DIM count AS INTEGER
    count = count + 1
    PRINT count
END SUB

FUNCTION NextCount# STATIC
    DIM fcount AS DOUBLE
    fcount = fcount + 1
    NextCount# = fcount
END FUNCTION
