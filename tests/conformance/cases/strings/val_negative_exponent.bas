$CONSOLE:ONLY
' VAL with a negative exponent and more than one integer digit.
' Printed as integers (x * 10000) so PRINT's float formatting does not matter.
PRINT CLNG(VAL("123E-2") * 10000)
PRINT CLNG(VAL("0.5E-1") * 10000)
PRINT CLNG(VAL("12E-1") * 10000)
PRINT CLNG(VAL("-250E-2") * 10000)
PRINT CLNG(VAL("0.05E-1") * 100000)
SYSTEM
