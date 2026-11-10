$CONSOLE:ONLY
ON ERROR GOTO handler
DIM a(-3 TO 4, 10 TO 12) AS LONG
PRINT LBOUND(a); UBOUND(a); LBOUND(a, 2); UBOUND(a, 2)
REDIM b(5) AS INTEGER
PRINT LBOUND(b); UBOUND(b)
REDIM b(2 TO 9) AS INTEGER
PRINT LBOUND(b); UBOUND(b)
ERASE b
PRINT UBOUND(b)
PRINT LBOUND(a, 3)
DIM bits(20) AS _BIT * 3, ubits(20) AS _UNSIGNED _BIT * 5
FOR k = 0 TO 20: bits(k) = k - 10: ubits(k) = k * 3: NEXT
FOR k = 0 TO 20: PRINT bits(k); ubits(k);: NEXT: PRINT
PRINT HEX$(-1); " "; HEX$(-32769); " "; HEX$(255); " "; HEX$(-1&&); " "; HEX$(0)
PRINT OCT$(-1); " "; OCT$(8); " "; OCT$(-8&); " "; _BIN$(5); " "; _BIN$(-200)
PRINT HEX$(2.5); " "; HEX$(3.5); " "; OCT$(-1.5); " "; _BIN$(1E3)
PRINT FIX(-2.7); FIX(2.7); INT(-2.7); LOG(1); SQR(2); EXP(1)
PRINT "done"
SYSTEM
handler:
PRINT "error"; ERR
RESUME NEXT
