DIM SHARED t&
FOR i% = 1 TO 50
    addto i%
NEXT
PRINT t&
SUB addto (n%)
    t& = t& + n% * 2
END SUB
