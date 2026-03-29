SCREEN 13
DIM sprite(512)
PSET (5, 5), 10
LINE (5, 5)-(15, 15), 12, BF
GET (5, 5)-(15, 15), sprite
CLS
PUT (20, 20), sprite, XOR
