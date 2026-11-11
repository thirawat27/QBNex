FOR i% = 1 TO 300
    GOSUB ga
    IF i% MOD 7 = 0 THEN GOSUB gb
NEXT
PRINT c&; d&
END
ga:
c& = c& + i%
RETURN
gb:
d& = d& + 1
GOSUB ga
RETURN
