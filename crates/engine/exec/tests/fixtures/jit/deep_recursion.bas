FOR k% = 1 TO 3
    t& = t& + down&(3000)
NEXT
PRINT t&
FUNCTION down& (n&)
    IF n& > 0 THEN down& = down&(n& - 1) + 1
END FUNCTION
