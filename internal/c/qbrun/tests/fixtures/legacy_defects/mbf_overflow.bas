$CONSOLE:ONLY
ON ERROR GOTO ov
m$ = MKSMBF$(3E+38)
PRINT "single 3E+38 len"; LEN(m$)
m$ = MKDMBF$(1D+300)
PRINT "double 1D+300 len"; LEN(m$)
m$ = MKDMBF$(-0#)
PRINT "double -0 len"; LEN(m$)
SYSTEM
ov:
PRINT "overflow error"; ERR
RESUME NEXT
