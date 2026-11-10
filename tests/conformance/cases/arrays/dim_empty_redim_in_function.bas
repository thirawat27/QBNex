$CONSOLE:ONLY
' DIM SHARED name() declares a dynamic array; a REDIM inside a FUNCTION
' must size that shared array, not a new local one.
DIM SHARED cells%()
DIM SHARED names() AS STRING
PRINT Fill%(5); Cell%(5); UBOUND(cells%)
REDIM names(1 TO 2)
names(2) = "two"
PRINT LBOUND(names); UBOUND(names); names(2)
SYSTEM

FUNCTION Fill% (n AS INTEGER)
    REDIM cells%(1 TO n)
    cells%(n) = n * 3
    Fill% = cells%(n)
END FUNCTION

FUNCTION Cell% (i AS INTEGER)
    Cell% = cells%(i)
END FUNCTION
