$CONSOLE:ONLY
' A SUB that hands its parameters to CALL ABSOLUTE is called from another SUB
' with a plain local variable (the classic mouse-driver wrapper shape).
DECLARE SUB SetWord (v%)
DECLARE SUB Caller ()
Caller
SYSTEM

SUB SetWord (v%)
    ' push bp / mov bp,sp / mov bx,[bp+6] / mov word [bx],1234 / pop bp / retf 2
    code$ = CHR$(&H55) + CHR$(&H89) + CHR$(&HE5) + CHR$(&H8B) + CHR$(&H5E) + CHR$(6)
    code$ = code$ + CHR$(&HC7) + CHR$(7) + CHR$(&HD2) + CHR$(4) + CHR$(&H5D) + CHR$(&HCA) + CHR$(2) + CHR$(0)
    DEF SEG = VARSEG(code$)
    CALL ABSOLUTE(v%, SADD(code$))
    DEF SEG
END SUB

SUB Caller
    x% = 7
    SetWord x%
    PRINT x%
END SUB
