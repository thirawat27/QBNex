DIM SHARED c&
FOR i% = 1 TO 50
    sq i%
NEXT
PRINT "no"
SUB sq (n%)
    GOSUB g
    PRINT c&;
    IF n% = 40 THEN END
    EXIT SUB
g:
    c& = c& + n%
    RETURN
END SUB
