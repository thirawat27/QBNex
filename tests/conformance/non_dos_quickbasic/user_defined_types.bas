TYPE Pair
    left AS INTEGER
    right AS INTEGER
END TYPE

DIM pair AS Pair

pair.left = 10
pair.right = 20

PRINT pair.left
PRINT pair.right
PRINT pair.left + pair.right
