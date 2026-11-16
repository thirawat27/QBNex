$CONSOLE:ONLY
' Method syntax wherever a value of a TYPE is: a SUB parameter, an array element,
' a field that is itself a TYPE (also inside an array element), a FUNCTION
' parameter in an expression. A field named like a procedure stays the field,
' and KEY LIST still compiles now that LIST is not a reserved word.
TYPE Counter
    total AS LONG
END TYPE

TYPE Box
    inner AS Counter
    Size AS LONG
    tag AS STRING
END TYPE

DIM c AS Counter
DIM many(1 TO 3) AS Counter
DIM b AS Box
DIM boxes(0 TO 1) AS Box

AddTwice c, 4
PRINT "param"; c.total; Doubled&(c)
FOR i = 1 TO 3
    many(i).Add i * 10
    IF i > 1 THEN many(i).Add 1
NEXT
PRINT "array"; many(1).total; many(2).Value(0); many(3).Value&(0) + many(1).Value(0)
b.inner.Add 7
b.inner.Add(1)
CALL b.inner.Add(2)
PRINT "field"; b.inner.total; b.inner.Value(5)
boxes(1).inner.Add 9
PRINT "array field"; boxes(1).inner.Value(0); boxes(0).inner.Value(0)

' Size is a field of Box: b.Size is the field, not a call of Box_Size
b.Size = 3
b.tag = "t"
PRINT "fields"; b.Size; b.tag; Box_Size&(b)

' KEY LIST is still a statement (never run here: it would print the key list)
IF b.Size = 0 THEN KEY LIST
SYSTEM

SUB AddTwice (p AS Counter, n AS LONG)
    p.Add n
    p.Add(n)
END SUB

FUNCTION Doubled& (p AS Counter)
    Doubled& = p.Value(0) * 2
END FUNCTION

SUB Counter_Add (self AS Counter, n AS LONG)
    self.total = self.total + n
END SUB

FUNCTION Counter_Value& (self AS Counter, extra AS LONG)
    Counter_Value& = self.total + extra
END FUNCTION

FUNCTION Box_Size& (self AS Box)
    Box_Size& = -1
END FUNCTION
