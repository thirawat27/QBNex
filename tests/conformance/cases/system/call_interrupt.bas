$CONSOLE:ONLY
' CALL INTERRUPT: INT 33h answers the mouse driver; other interrupts leave the registers
TYPE RegType
    ax AS INTEGER
    bx AS INTEGER
    cx AS INTEGER
    dx AS INTEGER
    bp AS INTEGER
    si AS INTEGER
    di AS INTEGER
    flags AS INTEGER
END TYPE
TYPE RegTypeX
    ax AS INTEGER
    bx AS INTEGER
    cx AS INTEGER
    dx AS INTEGER
    bp AS INTEGER
    si AS INTEGER
    di AS INTEGER
    flags AS INTEGER
    ds AS INTEGER
    es AS INTEGER
END TYPE
DIM r AS RegType, o AS RegType, x AS RegTypeX
r.ax = 0
CALL INTERRUPT(&H33, r, o)
PRINT "reset:"; o.ax; o.bx
r.ax = 5: r.si = 7: r.di = -2
CALL INTERRUPT(&H21, r, o)
PRINT "int 21h:"; o.ax; o.si; o.di
x.ax = 0: x.ds = 123: x.es = 456
INTERRUPTX &H33, x, x
PRINT "interruptx:"; x.ax; x.bx; x.ds; x.es
SYSTEM
