$CONSOLE:ONLY
' Fixed-length strings: padding, truncation, TYPE fields, arrays, comparisons.
TYPE rec
    nm AS STRING * 8
    id AS INTEGER
    tag AS STRING * 3
END TYPE
DIM r(3) AS rec
FOR i = 0 TO 3
    r(i).nm = "name" + LTRIM$(STR$(i * 111))
    r(i).id = i
    r(i).tag = CHR$(65 + i) + CHR$(66 + i) + CHR$(67 + i) + "overflow"
NEXT
FOR i = 0 TO 3: PRINT "["; r(i).nm; "]["; r(i).tag; "]"; LEN(r(i).nm): NEXT
DIM f AS STRING * 6, g AS STRING * 6
f = "ab": g = "ab    "
IF f = g THEN PRINT "f=g";
IF f = "ab" THEN PRINT " f=ab";
IF RTRIM$(f) = "ab" THEN PRINT " rtrim=ab"
PRINT
DIM arr(2) AS STRING * 4
arr(0) = "x": arr(1) = "yyyyyy": arr(2) = arr(0) + arr(1)
FOR i = 0 TO 2: PRINT "<"; arr(i); ">": NEXT
v$ = f + "|" + g + "|" + arr(2)
PRINT v$; LEN(v$)
f = UCASE$("mixed case"): PRINT f
f = "": PRINT "["; f; "]"; ASC(f)
SYSTEM
