$CONSOLE:ONLY
' _OFFSET(x): machine address of a variable, of a STRING's characters, of an array element
DIM a AS LONG, s AS STRING, m AS _MEM, o AS _OFFSET
DIM b(5) AS INTEGER
a = 123456
m = _MEM(_OFFSET(a), 4)
PRINT _MEMGET(m, m.OFFSET, LONG)
_MEMPUT m, m.OFFSET, -7 AS LONG
PRINT a
s = "HELLO"
m = _MEM(_OFFSET(s), LEN(s))
PRINT CHR$(_MEMGET(m, m.OFFSET + 1, _UNSIGNED _BYTE)); CHR$(_MEMGET(m, m.OFFSET + 4, _UNSIGNED _BYTE))
b(2) = 7: b(3) = 9
o = _OFFSET(b(2))
m = _MEM(o, 4)
PRINT _MEMGET(m, m.OFFSET, INTEGER); _MEMGET(m, m.OFFSET + 2, INTEGER)
PRINT _OFFSET(b(3)) - _OFFSET(b(2))
SYSTEM
