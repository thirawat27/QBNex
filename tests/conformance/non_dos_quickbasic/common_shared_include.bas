COMMON SHARED total AS INTEGER
DIM i AS INTEGER

'$INCLUDE:'common_shared_include_worker.bi'

FOR i = 1 TO 3
    CALL AddThree
NEXT i

PRINT total
