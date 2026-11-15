$CONSOLE:ONLY
' Console INPUT asks again ("Redo from start") for an answer that does not fit
' its variables, as QBasic does: a word or a number too large for a numeric
' variable, too many or too few items, text after a closing quote. At the end
' of redirected input the last answer is taken as it is (no endless loop).
INPUT "a"; a%
PRINT a%
INPUT b, c$
PRINT b; c$
INPUT d&
PRINT d&
INPUT s$, t
PRINT s$; t
INPUT e%, f%
PRINT e%; f%
SYSTEM
