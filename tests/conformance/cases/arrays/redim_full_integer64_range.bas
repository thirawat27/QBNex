$CONSOLE:ONLY
' Bounds whose element count does not fit 64 bits: out of memory, not an
' empty array that accepts every subscript.
ON ERROR GOTO h
DIM lo AS _INTEGER64, hi AS _INTEGER64
lo = -9223372036854775807 - 1
hi = 9223372036854775807
REDIM a(lo TO hi) AS INTEGER
PRINT "done"
SYSTEM
h:
PRINT "error"; ERR
RESUME NEXT
