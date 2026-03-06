SCREEN 13
COLOR 14
PRINT "Testing Graphics and Modules"

x = 10
y = 10
w = 50
h = 50
c = 4

CALL DrawBox(x, y, w, h, c)

PRINT "Area is "; CalculateArea(w, h)
SLEEP

SUB DrawBox(x, y, w, h, c)
    LINE (x, y)-(x + w, y + h), c, BF
END SUB

FUNCTION CalculateArea(w, h)
    CalculateArea = w * h
END FUNCTION
