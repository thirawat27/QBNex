$CONSOLE:ONLY
' VAL parsing (qbrt::val); only forms where legacy and QBasic agree.
PRINT VAL("42"); VAL("-42"); VAL("+7"); VAL("007"); VAL("1 2 3"); VAL(" 12abc")
PRINT VAL("12.5"); VAL(".5"); VAL("-0.25"); VAL("1.5E3"); VAL("2D2"); VAL("1.25E-1")
PRINT VAL("&H10"); VAL("&hff"); VAL("&O17"); VAL("&B1010"); VAL("&H"); VAL("&X1")
PRINT VAL("9223372036854775807")
PRINT VAL("x"); VAL(""); VAL("."); VAL("E5"); VAL("-")
PRINT USING "##.##################"; VAL("0.1")
x## = VAL("3.3621031431120935063E-4932")
PRINT x## > 0
SYSTEM
