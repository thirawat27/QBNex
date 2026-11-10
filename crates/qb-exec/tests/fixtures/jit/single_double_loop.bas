FOR x! = 1 TO 0 STEP -.1
    s! = s! + x! * x!
    d# = d# + 1 / (x! + 1)
NEXT
PRINT s!; d#; x!
