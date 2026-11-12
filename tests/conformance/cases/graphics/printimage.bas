' _PRINTIMAGE of the screen and of an 8-bit image (the runner records each image)
SCREEN _NEWIMAGE(4, 2, 32)
PSET (0, 0), _RGB32(255, 0, 0)
PSET (3, 1), _RGB32(0, 0, 255)
_PRINTIMAGE 0
img& = _NEWIMAGE(3, 3, 256)
_DEST img&
PSET (1, 1), 4
_DEST 0
_PRINTIMAGE img&
SYSTEM
