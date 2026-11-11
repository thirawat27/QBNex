$CONSOLE:ONLY
' _OFFSET of arrays: _OFFSET(a()) is the first element, _OFFSET(a(i)) the element;
' the addresses are real (memcpy from the C runtime reads and writes through them).
DECLARE LIBRARY
    SUB memcpy (BYVAL dst AS _OFFSET, BYVAL src AS _OFFSET, BYVAL bytes AS _OFFSET)
END DECLARE
DIM a(1 TO 5) AS LONG, b(1 TO 5) AS LONG
FOR i = 1 TO 5: a(i) = i * 10: NEXT
PRINT _OFFSET(a(2)) - _OFFSET(a(1)); _OFFSET(a(5)) - _OFFSET(a()); (_OFFSET(a()) = _OFFSET(a(1)))
memcpy _OFFSET(b()), _OFFSET(a()), 20
memcpy _OFFSET(b(5)), _OFFSET(a(1)), 4
FOR i = 1 TO 5: PRINT b(i);: NEXT: PRINT

' first index varies fastest, as in QBasic
DIM d(0 TO 2, 0 TO 3) AS INTEGER
PRINT _OFFSET(d(1, 0)) - _OFFSET(d(0, 0)); _OFFSET(d(0, 1)) - _OFFSET(d(0, 0)); (_OFFSET(d()) = _OFFSET(d(0, 0)))
d(2, 3) = 1234
DIM w AS INTEGER
memcpy _OFFSET(w), _OFFSET(d(2, 3)), 2
PRINT w

' a STRING element: the address of its characters
DIM s(1 TO 2) AS STRING
s(1) = "ABCD": s(2) = "xy"
DIM x AS LONG
memcpy _OFFSET(x), _OFFSET(s(1)), 4
PRINT HEX$(x)
DIM t(0 TO 3) AS _UNSIGNED _BYTE
memcpy _OFFSET(t()), _OFFSET(s(2)), 2
PRINT t(0); t(1); t(2)

' arrays in a SUB (BYREF array parameter) and a UDT array
TYPE pt
    x AS LONG
    y AS LONG
END TYPE
DIM p(0 TO 2) AS pt
p(1).y = 77
PRINT _OFFSET(p(1)) - _OFFSET(p()); _OFFSET(p(1).y) - _OFFSET(p(1))
show a()
SYSTEM

SUB show (arr() AS LONG)
    DIM v AS LONG
    memcpy _OFFSET(v), _OFFSET(arr(3)), 4
    PRINT v; _OFFSET(arr(4)) - _OFFSET(arr())
END SUB
