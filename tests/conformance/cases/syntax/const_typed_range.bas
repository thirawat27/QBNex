$CONSOLE:ONLY
' CONST values at the edges of each suffix type are accepted (one past the edge is
' "Overflow", see tests/fixtures/robustness/const_overflow*.bas). A negative value for
' an unsigned type stands for its two's complement bits, as in QB64.
CONST a% = 32767, b% = -32768, c~% = 65535, d& = -2147483648, e~& = &HFFFFFFFF, f%% = -128, g~%% = 255
CONST h&& = 9223372036854775807, i~&& = 18446744073709551615, k% = &HFFFF, l~% = &HFFFF, m% = 2.4
CONST o# = 1D+300, p~& = -1
PRINT a; b; c; d; e; f; g
PRINT h; i; k; l; m
PRINT o; p
' NOT after a binary operator, and brackets, in CONST (were "Invalid CONST expression")
CONST n1 = 1 + NOT 0, n2 = 2 * NOT 0, n3 = 3 AND 1 OR 4 XOR 2, n4 = 17714 AND 17714 OR 0 XOR 0
CONST n5 = (1 + 2) * (3 + (4 - 1)) / 2, n6 = 2 * NOT 0 + 1, n7 = NOT 0 AND 3, n8 = 1 + NOT 0 AND 3
PRINT n1; n2; n3; n4; n5; n6; n7; n8
x = 0
PRINT 1 + NOT x; 2 * NOT x; 3 AND 1 OR 4 XOR 2; 17714 AND 17714 OR x XOR x; (1 + 2) * (3 + (4 - 1)) / 2; 2 * NOT x + 1; NOT x AND 3; 1 + NOT x AND 3
CONST m1 = 10 \ NOT 1, m2 = 7 MOD NOT 2, m3 = 10 / NOT 1, m4 = 4 = NOT 0 = 5, m5 = 3 + NOT 2 * 4, m6 = 1 OR NOT 2 AND 3
PRINT m1; m2; m3; m4; m5; m6
y = 1: z = 2
PRINT 10 \ NOT y; 7 MOD NOT z; 10 / NOT y; 4 = NOT 0 = 5; 3 + NOT z * 4; 1 OR NOT z AND 3
SYSTEM
