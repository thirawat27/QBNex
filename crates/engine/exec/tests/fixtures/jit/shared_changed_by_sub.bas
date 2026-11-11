DIM SHARED t&
t& = 5
FOR i% = 1 TO 40
    t& = t& + 1
    addto i%
NEXT
PRINT t&
SUB addto (n%)
    t& = (t& * 2) MOD 1000 + n%
END SUB
