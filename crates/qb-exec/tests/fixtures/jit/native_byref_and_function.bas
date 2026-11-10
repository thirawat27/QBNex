FOR i& = 1 TO 2000
    inc c&
    t& = t& + sq&(i&)
NEXT
PRINT c&; t&
SUB inc (n&)
    n& = n& + 1
END SUB
FUNCTION sq& (x&)
    sq& = x& * x& MOD 1000
END FUNCTION
