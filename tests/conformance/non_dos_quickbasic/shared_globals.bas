DIM SHARED total AS INTEGER

FOR i = 1 TO 3
    CALL AddOne
NEXT i

PRINT total

SUB AddOne
    total = total + 1
END SUB
