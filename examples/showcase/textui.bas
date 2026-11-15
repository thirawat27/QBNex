' Classic DOS-style text application: menu bar, windows with shadows,
' a data table and a dialog box, built from CP437 box-drawing characters.
SCREEN _NEWIMAGE(640, 400, 32) ' 80 x 25 text cells with the 8x16 VGA font
_TITLE "QBNex text UI"
DIM SHARED pal(15) AS _UNSIGNED LONG
FOR i = 0 TO 15: READ r, g, b: pal(i) = _RGB32(r, g, b): NEXT
DATA 0,0,0, 0,0,170, 0,170,0, 0,170,170, 170,0,0, 170,0,170, 170,85,0, 170,170,170
DATA 85,85,85, 85,85,255, 85,255,85, 85,255,255, 255,85,85, 255,85,255, 255,255,85, 255,255,255

' Desktop
Fill 1, 1, 25, 80, 1, 3, CHR$(176)
' Menu bar and status line
Fill 1, 1, 1, 80, 0, 7, " "
At 1, 3, 0, 7, "File  Edit  View  Search  Run  Debug  Options": At 1, 76, 0, 7, "Help"
At 1, 3, 4, 7, "F": At 1, 9, 4, 7, "E": At 1, 15, 4, 7, "V": At 1, 21, 4, 7, "S"
Fill 25, 1, 25, 80, 0, 3, " "
At 25, 2, 0, 3, "F1=Help  F2=Save  F5=Run  Alt+X=Exit": At 25, 66, 0, 3, "00018:024"

' Main window: an inventory table
Box 3, 3, 21, 62, 15, 1, " INVENTORY.DAT "
At 4, 5, 14, 1, "Code   Item                    Qty    Price    Total"
At 5, 5, 7, 1, STRING$(54, 196)
FOR i = 1 TO 9
    READ code$, item$, qty, price
    fg = 7: bg = 1
    IF i = 4 THEN fg = 0: bg = 3: Fill 5 + i, 4, 5 + i, 61, fg, bg, " "
    At 5 + i, 5, fg, bg, code$ + "  " + LEFT$(item$ + SPACE$(22), 22) + RIGHT$(SPACE$(5) + STR$(qty), 5) + RIGHT$(SPACE$(9) + STR$(price), 9) + RIGHT$(SPACE$(10) + STR$(qty * price), 10)
    total = total + qty * price
NEXT
DATA "A-100","Floppy disks (10)",42,12, "A-101","Mouse, 2-button",18,25, "B-200","VGA monitor 14in",6,299
DATA "B-210","Keyboard, 101 keys",25,45, "C-300","Modem 14.4k",11,129, "C-310","Sound card",9,89
DATA "D-400","QuickBASIC 4.5",30,99, "D-410","Joystick",14,35, "E-500","Dot-matrix printer",4,249
At 16, 5, 7, 1, STRING$(54, 196)
At 17, 5, 15, 1, "Items: 9" + SPACE$(29) + "Grand total:" + RIGHT$(SPACE$(6) + STR$(total), 6)
At 19, 5, 11, 1, "Enter=Edit  Ins=Add  Del=Remove  /=Search"

' Side panel
Box 3, 64, 14, 78, 0, 7, " Stock "
FOR i = 0 TO 4
    READ lbl$, v
    At 5 + i * 2, 66, 0, 7, lbl$: At 5 + i * 2, 68, 2, 7, STRING$(v, 219) + STRING$(8 - v, 250)
NEXT
DATA "A ",7, "B ",3, "C ",5, "D ",8, "E ",2
Box 16, 64, 21, 78, 15, 5, " Clock "
At 17, 67, 14, 5, "1994-11-15": At 18, 68, 15, 5, "09:41:07": At 20, 66, 11, 5, "Mem 512 KB"

' Modal dialog
Box 9, 20, 15, 52, 0, 7, " Save changes? "
At 11, 23, 0, 7, "INVENTORY.DAT was modified."
At 13, 25, 15, 2, " <Yes> ": At 13, 34, 0, 7, " <No> ": At 13, 42, 0, 7, "<Cancel>"
SLEEP
SYSTEM

SUB At (row, col, fg, bg, s$)
    COLOR pal(fg), pal(bg): LOCATE row, col: PRINT s$;
END SUB

SUB Fill (r1, c1, r2, c2, fg, bg, ch$)
    FOR r = r1 TO r2: At r, c1, fg, bg, STRING$(c2 - c1 + 1, ch$): NEXT
END SUB

SUB Box (r1, c1, r2, c2, fg, bg, title$)
    Fill r1, c1, r2, c2, fg, bg, " "
    At r1, c1, fg, bg, CHR$(201) + STRING$(c2 - c1 - 1, 205) + CHR$(187)
    FOR r = r1 + 1 TO r2 - 1
        At r, c1, fg, bg, CHR$(186): At r, c2, fg, bg, CHR$(186)
    NEXT
    At r2, c1, fg, bg, CHR$(200) + STRING$(c2 - c1 - 1, 205) + CHR$(188)
    At r1, c1 + (c2 - c1 + 1 - LEN(title$)) \ 2, fg, bg, title$
    ' Drop shadow
    Fill r2 + 1, c1 + 2, r2 + 1, c2 + 2, 8, 0, " "
    Fill r1 + 1, c2 + 1, r2 + 1, c2 + 2, 8, 0, " "
END SUB
