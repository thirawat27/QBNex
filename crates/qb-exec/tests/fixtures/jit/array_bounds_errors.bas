DIM a%(5)
DIM m#(2, 3)
ON ERROR GOTO h
FOR i% = 0 TO 8
    a%(i%) = i%
    s% = s% + a%(i%)
    m#(1, i%) = i%
    t# = t# + m#(1, i%)
NEXT
PRINT s%; t#; e%
END
h:
e% = e% + 1
RESUME NEXT
