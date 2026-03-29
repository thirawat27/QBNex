SUB DrawBox (x1, y1, x2, y2, c)
    LINE (x1, y1)-(x2, y2), c, B
END SUB

SCREEN 13
CALL DrawBox(10, 10, 40, 30, 15)
DRAW "BM50,50 C10 R10 D10 L10 U10"
