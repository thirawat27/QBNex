COMMON SHARED total AS INTEGER
DIM i AS INTEGER

FOR i = 1 TO 4
    CALL AddTwo
NEXT i

PRINT total

SUB AddTwo
    total = total + 2
END SUB
