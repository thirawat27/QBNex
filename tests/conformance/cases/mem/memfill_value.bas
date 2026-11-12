$CONSOLE:ONLY
' _MEMFILL with a value that is not a variable and no AS (an extension): the
' value's own type gives the pattern: its suffix, the literal's type, or the type of
' the expression (integer arithmetic is _INTEGER64)
DIM m AS _MEM
m = _MEMNEW(8)
_MEMFILL m, m.OFFSET, 8, 258%
PRINT _MEMGET(m, m.OFFSET, INTEGER); _MEMGET(m, m.OFFSET + 6, INTEGER)
_MEMFILL m, m.OFFSET, 8, &H01020304&
PRINT HEX$(_MEMGET(m, m.OFFSET + 4, LONG))
_MEMFILL m, m.OFFSET, 8, 1.5#
PRINT _MEMGET(m, m.OFFSET, DOUBLE)
x% = 3
_MEMFILL m, m.OFFSET, 8, x% * 2 + 1
PRINT _MEMGET(m, m.OFFSET, _INTEGER64)
_MEMFILL m, m.OFFSET, 8, 7
FOR i = 0 TO 7: PRINT _MEMGET(m, m.OFFSET + i, _UNSIGNED _BYTE);: NEXT
PRINT
_MEMFREE m
SYSTEM
