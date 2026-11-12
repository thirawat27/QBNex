$CONSOLE:ONLY
' _MEMFILL with a variable and no AS, _MEMGET into a variable-length STRING,
' _MEM of a fixed-length STRING array
DIM m AS _MEM, w AS _MEM
m = _MEMNEW(8)
v% = 513
_MEMFILL m, m.OFFSET, 8, v%
FOR i = 0 TO 7: PRINT _MEMGET(m, m.OFFSET + i, _UNSIGNED _BYTE);: NEXT
PRINT
_MEMFILL m, m.OFFSET, 8, "AB"
s$ = SPACE$(5)
_MEMGET m, m.OFFSET + 1, s$
PRINT "["; s$; "]"
DIM f(1 TO 3) AS STRING * 2, g AS STRING * 2
f(1) = "ab": f(2) = "cd": f(3) = "ef"
w = _MEM(f())
PRINT w.SIZE; w.ELEMENTSIZE
_MEMGET w, w.OFFSET + 2, g
PRINT g
_MEMFREE m
SYSTEM
