FOR i% = 1 TO 12
    s& = s& + fact&(i% MOD 8)
NEXT
PRINT s&
stopper
PRINT "no"
FUNCTION fact& (n&)
    IF n& <= 1 THEN fact& = 1 ELSE fact& = n& * fact&(n& - 1)
END FUNCTION
SUB stopper
    END
END SUB
