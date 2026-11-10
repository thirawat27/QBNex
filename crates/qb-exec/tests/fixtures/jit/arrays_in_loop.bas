DIM a&(100)
FOR i% = 1 TO 100
    a&(i%) = i% * i%
NEXT
FOR i% = 1 TO 100
    t& = t& + a&(i%)
NEXT
PRINT t&
