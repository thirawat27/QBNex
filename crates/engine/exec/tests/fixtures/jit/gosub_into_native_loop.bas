GOSUB s
PRINT x&; y&
END
s:
FOR i& = 1 TO 1000
    x& = x& + i&
NEXT
GOSUB t
RETURN
t:
y& = y& + 1
RETURN
