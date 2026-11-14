$CONSOLE:ONLY
' CONST: a minus sign in front of a power negates the power, a negative constant
' keeps its sign under ^, and brackets may be nested three deep.
CONST k = -6
CONST c = 2 * k ^ 2
CONST n1 = -2 ^ 2
CONST n2 = -(2 + 3) ^ 2 + 1
CONST n3 = 3 * -2 ^ 2
CONST e1 = 2 ^ -1
CONST p = (((1 + 2) * 3) + 1)
PRINT c; 2 * k ^ 2
PRINT n1; -2 ^ 2
PRINT n2; -(2 + 3) ^ 2 + 1
PRINT n3; 3 * -2 ^ 2
PRINT e1; p
SYSTEM
