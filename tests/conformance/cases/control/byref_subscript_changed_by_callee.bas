$CONSOLE:ONLY
' A BYREF array element is copied back to the element that was passed, even
' when the callee changes the (SHARED) subscript variable.
DIM SHARED i AS INTEGER
DIM a(10) AS INTEGER
i = 1
f a(i)
PRINT a(1); a(2)
SYSTEM

SUB f (x AS INTEGER)
    i = 2
    x = 7
END SUB
