$CONSOLE:ONLY
' _ASINH, _ACOSH, _ATANH (computed in double like the other hyperbolic functions)
PRINT USING "#.######"; _ASINH(1)
PRINT USING "#.######"; _ACOSH(2)
PRINT USING "#.######"; _ATANH(.5)
PRINT USING "##.######"; _ASINH(-2.5)
PRINT _ACOSH(1); _ATANH(0); _ASINH(0)
x# = _ATANH(.25)
PRINT USING "#.############"; x#
SYSTEM
