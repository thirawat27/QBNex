$CONSOLE:ONLY
' A BYREF parameter is the variable that was passed: a SHARED variable
' changed under its own name is changed for the parameter too (also in a
' procedure further down), two parameters bound to one variable are one
' variable, and so are a parameter and the element reached through its array.
DIM SHARED n AS INTEGER
DIM SHARED t AS STRING
DIM SHARED v(5) AS DOUBLE
DIM SHARED cnt AS LONG
n = 1
CALL p(n)
PRINT "after p:"; n
x& = 3
CALL twice(x&, x&)
PRINT "after twice:"; x&
t = "ab"
CALL s(t)
PRINT "after s: "; t
v(2) = 1.5
CALL e(v(2), v())
PRINT "after e:"; v(2)
CALL deep(n)
PRINT "after deep:"; n
CALL bump(cnt)
PRINT "after bump:"; cnt
SYSTEM

SUB p (k AS INTEGER)
    n = 5
    PRINT "k ="; k
    k = k + 1
    PRINT "n ="; n
END SUB

SUB twice (a AS LONG, b AS LONG)
    a = a * 10
    PRINT "b ="; b
    b = b + 1
    PRINT "a ="; a
END SUB

SUB s (x AS STRING)
    t = t + "cd"
    PRINT "x = "; x
    x = x + "ef"
    PRINT "t = "; t
END SUB

SUB e (y AS DOUBLE, w() AS DOUBLE)
    w(2) = w(2) * 2
    PRINT "y ="; y
    y = y + 1
    PRINT "w(2) ="; w(2)
END SUB

SUB deep (k AS INTEGER)
    CALL inner(k)
    PRINT "k ="; k
END SUB

SUB inner (j AS INTEGER)
    n = 100
    j = j + 1
END SUB

SUB bump (c AS LONG)
    DIM i AS LONG
    FOR i = 1 TO 100000
        c = c + 1
        cnt = cnt + 1
    NEXT
END SUB
