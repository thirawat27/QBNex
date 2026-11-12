$CONSOLE:ONLY
' Single-line DEF FN (QBasic): parameters, a module variable read by the
' expression, a string function, a call from a SUB and a call before the DEF line.
DECLARE SUB Show ()
k = 10
PRINT FnSq(3); FnAdd(1, 2)
DEF FnSq (x) = x * x
DEF FnAdd (a, b) = a + b + k ' reads the module variable k
DEF FnTwice$ (s$) = s$ + s$
DEF FnPi# = 4 * ATN(1#)
k = 20
PRINT FnAdd(1, 2); FnTwice$("ab"); FnPi# > 3.14
Show
SYSTEM

SUB Show
    PRINT FnSq(4)
END SUB