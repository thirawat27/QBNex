' One-argument math builtins on SINGLE, DOUBLE and integer values, and their errors.
' DOUBLE results of the libm functions print as SINGLE: the last digit of a DOUBLE
' SIN(2.5) differs between the math libraries of Linux and macOS.
$CONSOLE:ONLY
ON ERROR GOTO fail
DIM s AS SINGLE, d AS DOUBLE, k AS INTEGER, q AS _INTEGER64
DATA 0,1,-1,2.5,-2.5,0.1,1E10,-7.75,3.141592653589793
FOR n = 1 TO 9
    READ d
    s = d: k = CINT(d / 1E9 * 0 + d - INT(d)): q = d
    PRINT "x="; d
    PRINT ABS(s); ABS(d); SGN(s); SGN(d); SGN(q)
    PRINT INT(s); INT(d); FIX(s); FIX(d)
    PRINT SIN(s); CSNG(SIN(d)); COS(s); CSNG(COS(d))
    PRINT TAN(s); CSNG(TAN(d)); ATN(s); CSNG(ATN(d))
    PRINT SQR(ABS(s)); SQR(ABS(d))
    IF d > 0 THEN PRINT LOG(s); CSNG(LOG(d))
    IF d < 30 THEN PRINT EXP(s); CSNG(EXP(d))
NEXT
PRINT "errors"
PRINT SQR(-1)
PRINT LOG(0)
PRINT LOG(-2#)
s = 100: PRINT EXP(s)
d = 1000: PRINT EXP(d)
PRINT "done"
END
fail:
PRINT "error"; ERR
RESUME NEXT
