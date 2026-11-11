SCREEN 13
DIM SHARED MouseDATA$
DATA 55,89,E5,8B,5E,0C,8B,07,50,8B,5E,0A,8B,07,50,8B,5E,08,8B
DATA 0F,8B,5E,06,8B,17,5B,58,1E,07,CD,33,53,8B,5E,0C,89,07,58
DATA 8B,5E,0A,89,07,8B,5E,08,89,0F,8B,5E,06,89,17,5D,CA,08,00
MouseDATA$ = SPACE$(57)
FOR i = 1 TO 57
READ h$
Hexxer$ = CHR$(VAL("&H" + h$))
MID$(MouseDATA$, i, 1) = Hexxer$
NEXT i
LB% = 0: RB% = 0: mX% = 0: mY% = 0
DEF SEG = VARSEG(MouseDATA$)
Mouse = SADD(MouseDATA$)
CALL ABSOLUTE(LB%, RB%, mX%, mY%, Mouse)
OPEN "mouse.txt" FOR OUTPUT AS #1
PRINT #1, LB%; RB%
t! = TIMER: DO: LOOP UNTIL TIMER - t! > 1
LB% = 3: RB% = 0: mX% = 0: mY% = 0
CALL ABSOLUTE(LB%, RB%, mX%, mY%, Mouse)
PRINT #1, RB%; mX%; mY%
CLOSE #1
SYSTEM
