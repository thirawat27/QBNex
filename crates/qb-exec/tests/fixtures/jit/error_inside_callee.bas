ON ERROR GOTO h
FOR i% = 1 TO 30
    s& = s& + f&(i%)
NEXT
PRINT s&; e%
END
h:
e% = e% + 1
RESUME NEXT
FUNCTION f& (n%)
    f& = 100
    f& = 1000 \ (n% MOD 4)
END FUNCTION
