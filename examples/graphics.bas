' graphics.bas
SCREEN 12  ' 640x480, 16 colors
COLOR 15, 1  ' White on blue

CLS
PRINT "QBNex Graphics Demo"
PRINT "Press any key to continue..."
SLEEP

' Draw shapes
LINE (50, 50)-(300, 200), 14, B  ' Yellow box
CIRCLE (400, 125), 75, 12  ' Red circle
LINE (100, 300)-(500, 400), 10, BF  ' Filled green rectangle

' Draw pattern
FOR i = 0 TO 639 STEP 20
    LINE (i, 0)-(639 - i, 479), 9
NEXT i

LOCATE 25, 1
PRINT "Graphics demo complete. Press any key..."
SLEEP

SCREEN 0  ' Return to text mode
