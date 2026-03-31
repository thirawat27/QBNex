REDIM values(1 TO 3) AS INTEGER
DIM i AS INTEGER

FOR i = 1 TO 3
    values(i) = i * 7
NEXT i

ERASE values
REDIM values(1 TO 2) AS INTEGER

PRINT LBOUND(values)
PRINT UBOUND(values)
PRINT values(1)
PRINT values(2)
