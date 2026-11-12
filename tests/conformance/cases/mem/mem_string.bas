$CONSOLE:ONLY
' _MEM of a variable-length STRING (an extension; QB64 rejects it): a block over
' the string's current characters, one byte per element, valid until the string
' is assigned again (like _OFFSET of the string)
DIM m AS _MEM
a$ = "Hello"
m = _MEM(a$)
PRINT m.SIZE; m.ELEMENTSIZE; m.TYPE; m.OFFSET = _OFFSET(a$)
PRINT _MEMGET(m, m.OFFSET + 1, _UNSIGNED _BYTE)
_MEMPUT m, m.OFFSET, 74 AS _UNSIGNED _BYTE
PRINT a$
_MEMFILL m, m.OFFSET + 1, 4, "ab"
PRINT a$
b$ = SPACE$(3)
_MEMGET m, m.OFFSET + 2, b$
PRINT b$
e$ = ""
m = _MEM(e$)
PRINT m.SIZE
SYSTEM
