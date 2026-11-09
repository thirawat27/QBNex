$CONSOLE:ONLY
' _MEMFREE of a _MEMNEW block. Legacy freed the buffer twice and corrupted the
' heap (L-08, exit 0xC0000374); the Rust runtime frees it once (D-06).
DIM m AS _MEM
m = _MEMNEW(16)
_MEMFILL m, m.OFFSET, 16, 65 AS _UNSIGNED _BYTE
PRINT _MEMGET(m, m.OFFSET + 3, _UNSIGNED _BYTE); _MEMEXISTS(m)
_MEMFREE m
PRINT _MEMEXISTS(m)
FOR i = 1 TO 1000
    m = _MEMNEW(100)
    _MEMFREE m
NEXT
PRINT "done"
SYSTEM
