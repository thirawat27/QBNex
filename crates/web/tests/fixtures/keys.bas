PRINT "press"
k$ = ""
DO
    a$ = INKEY$
    IF a$ <> "" THEN k$ = k$ + a$
LOOP UNTIL LEN(k$) >= 3
PRINT "keys:"; k$
REM mouse: _MOUSEINPUT loop, then PRINT _MOUSEX; _MOUSEY; _MOUSEBUTTON(1) (QIR edited by hand: the qb-ir.exe build used lacks _MOUSE*)
LINE INPUT "name? "; n$
PRINT "[" + n$ + "]"
