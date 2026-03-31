OPTION BASE 1
DIM numbers(1 TO 3) AS INTEGER
DIM i AS INTEGER

FOR i = LBOUND(numbers) TO UBOUND(numbers)
    numbers(i) = i * 5
NEXT

REDIM PRESERVE numbers(1 TO 4) AS INTEGER
numbers(4) = 99

PRINT LBOUND(numbers)
PRINT UBOUND(numbers)

FOR i = 1 TO 4
    PRINT numbers(i)
NEXT
