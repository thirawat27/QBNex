$CONSOLE:ONLY
' ERASE of a static array clears it where it is: an element passed by reference still
' names the element (it read freed memory and a store to it crashed). The TYPE makes
' the program keep its arrays in memory blocks.
TYPE pt
    x AS LONG
END TYPE
DIM SHARED a(1000) AS LONG
DIM SHARED u AS pt
a(3) = 1
CALL p(a(3))
PRINT "a(3) ="; a(3); "UBOUND ="; UBOUND(a)
SYSTEM
SUB p (x AS LONG)
    ERASE a
    PRINT "x after ERASE ="; x
    x = 5
END SUB
