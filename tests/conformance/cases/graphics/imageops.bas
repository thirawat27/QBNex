$SCREENHIDE
$CONSOLE
' LOCATE 18 below raises error 5 (as in the legacy build, where the headless
' dialog returned at once and the program went on); skip it explicitly.
ON ERROR GOTO skip_error
SCREEN _NEWIMAGE(200, 150, 32)
CLS , _RGB32(0, 0, 40)
a& = _NEWIMAGE(64, 48, 32)
_DEST a&
LINE (0, 0)-(63, 47), _RGB32(255, 0, 0), BF
CIRCLE (32, 24), 20, _RGB32(0, 255, 0)
PAINT (32, 24), _RGB32(0, 0, 255), _RGB32(0, 255, 0)
_SETALPHA 128, _RGB32(0, 0, 255)
_CLEARCOLOR _RGB32(255, 0, 0)
_DEST 0
_PUTIMAGE (10, 10), a&
_PUTIMAGE (80, 10)-(190, 70), a&
_PUTIMAGE (10, 80)-(60, 140), a&, , (10, 10)-(40, 30)
b& = _COPYIMAGE(a&)
_SETALPHA 255, , b&
_PUTIMAGE (100, 90), b&
_DISPLAY
p& = _NEWIMAGE(40, 30, 13)
_DEST p&
_PALETTECOLOR 5, _RGB32(10, 20, 30)
LINE (0, 0)-(39, 29), 5, BF
LINE (0, 0)-(39, 29), 14
_CLEARCOLOR 14
_DEST 0
_PUTIMAGE (150, 100), p&
_MAPTRIANGLE (0, 0)-(63, 0)-(0, 47), a& TO(0, 149)-(60, 120)-(20, 100)
q& = _NEWIMAGE(60, 50, 13)
_MAPTRIANGLE (0, 0)-(39, 0)-(39, 29), p& TO(59, 0)-(59, 40)-(10, 40), q&
_MAPTRIANGLE _SEAMLESS(0, 0)-(39, 29)-(0, 29), p& TO(0, 0)-(50, 49)-(0, 49), q&
_PUTIMAGE (140, 0), q&
DRAW "BM100,75 C" + STR$(_RGB32(255, 255, 0)) + " R20 D20 L20 U20 BM+5,5 TA45 U10 E5 F5 G5 H5"
COLOR _RGB32(255, 255, 255), _RGB32(0, 0, 0)
LOCATE 18, 2: PRINT "Hi!";
c& = _RGB32(1, 2, 3)
_DEST _CONSOLE
PRINT _RED32(c&); _GREEN(c&, a&); _BLUE(c&); _ALPHA(_RGBA32(1, 2, 3, 4))
PRINT _PALETTECOLOR(5, p&) AND &HFFFFFF; _RGB(255, 0, 0, p&); _RGBA(0, 0, 0, 0, p&); _CLEARCOLOR(p&)
PRINT _WIDTH(a&); _HEIGHT(p&); _PIXELSIZE(p&); _PIXELSIZE(a&)
_FREEIMAGE a&
_FREEIMAGE b&
SYSTEM
skip_error:
RESUME NEXT
