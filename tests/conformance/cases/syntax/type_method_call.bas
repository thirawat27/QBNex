$CONSOLE:ONLY
' Method syntax on a variable of a user TYPE: x.Name(args) calls T_Name(x, args).
' The bracket form, the statement form without brackets (with and without
' arguments), the expression form without brackets, CALL, and a one-line
' IF ... ELSE with a method call on each side.
TYPE Counter
    total AS LONG
    label AS STRING
END TYPE

DIM c AS Counter
c.label = "hits"
c.Add 5
c.Add(2)
CALL c.Add(3)
c.Reset2
c.Add 1: c.Add 1
PRINT c.label; c.total; c.Value&(10); c.Twice; c.Value(0) + c.Twice * 2
IF c.total > 1 THEN c.Add 100 ELSE c.Reset2
PRINT c.total
IF c.total > 1000 THEN c.Add 1 ELSE c.Reset2
PRINT c.total
IF c.Twice = 0 THEN PRINT "zero" ELSE PRINT "not zero"
n& = c.Value(7): PRINT n&; c.Describe$("count")
PRINT c.Describe$("x" + "y"); LEN(c.Describe$(c.label))
SYSTEM

SUB Counter_Add (self AS Counter, n AS LONG)
    self.total = self.total + n
END SUB

SUB Counter_Reset2 (self AS Counter)
    self.total = 0
END SUB

FUNCTION Counter_Value& (self AS Counter, extra AS LONG)
    Counter_Value& = self.total + extra
END FUNCTION

FUNCTION Counter_Twice& (self AS Counter)
    Counter_Twice& = self.total * 2
END FUNCTION

FUNCTION Counter_Describe$ (self AS Counter, what AS STRING)
    Counter_Describe$ = what + "=" + LTRIM$(STR$(self.total))
END FUNCTION
