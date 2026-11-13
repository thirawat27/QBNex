$CONSOLE:ONLY
' Exercises MKSMBF$/CVSMBF/MKDMBF$/CVDMBF (qb_runtime::mbf).
DIM vs(9) AS SINGLE
vs(0) = 0: vs(1) = 1: vs(2) = -1: vs(3) = 0.5: vs(4) = 3.25
vs(5) = 1E+30: vs(6) = -1E-30: vs(7) = 123456.7: vs(8) = 0.1: vs(9) = 2 ^ 126
FOR i = 0 TO 9
    m$ = MKSMBF$(vs(i))
    PRINT HexBytes$(m$); CVSMBF(m$)
NEXT
DIM vd(7) AS DOUBLE
vd(0) = 0: vd(1) = 1: vd(2) = -2.5#: vd(3) = 0.1#: vd(4) = 1D+30
vd(5) = -1D-30: vd(6) = 3.14159265358979#: vd(7) = 1D+38
FOR i = 0 TO 7
    m$ = MKDMBF$(vd(i))
    PRINT HexBytes$(m$); CVDMBF(m$)
NEXT
PRINT CVSMBF(CHR$(0) + CHR$(0) + CHR$(0) + CHR$(129))
PRINT CVSMBF(CHR$(0) + CHR$(0) + CHR$(128) + CHR$(130))
PRINT CVDMBF(STRING$(7, 0) + CHR$(132))
SYSTEM

FUNCTION HexBytes$ (s AS STRING)
    FOR i = 1 TO LEN(s): h$ = h$ + RIGHT$("0" + HEX$(ASC(s, i)), 2) + " ": NEXT
    HexBytes$ = h$
END FUNCTION
