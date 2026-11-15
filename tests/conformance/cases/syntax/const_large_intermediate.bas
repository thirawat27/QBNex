$CONSOLE:ONLY
' CONST folding with an intermediate value of 1E+15 or more: the rest of the expression
' was dropped (the first CONST was 9000000000000000000)
CONST c1 = 3000000000 * 3000000000 + 1 - 3000000000 * 3000000000
CONST c2 = 100000000 * 100000000 - 99999999 * 100000000
CONST c3 = 2000000 * 2000000 * 1000 - 1
CONST c4 = -(40000000 * 40000000) + 5
' whole numbers keep every digit, and OR is not the OR inside XOR
CONST c5&& = 1234567890123456789 + 1
CONST c6&& = 2 ^ 62 + 1
CONST c7 = 3 AND 1 OR 4 XOR 2
CONST c8 = 17714 AND 17714 OR 0 XOR 0
CONST c9 = 1E+20 / 4
PRINT c1
PRINT c2
PRINT c3
PRINT c4
PRINT c5&&; c6&&
PRINT c7; c8; c9
SYSTEM
