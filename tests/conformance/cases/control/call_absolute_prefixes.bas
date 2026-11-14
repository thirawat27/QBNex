$CONSOLE:ONLY
' CALL ABSOLUTE: prefixes in the order assemblers write them (segment override
' first), and a 32-bit address with a SIB byte and a displacement on the stack.
DIM x AS INTEGER
' mov eax, es:[bx] (26 66 8B 07) / retf
code$ = CHR$(&H26) + CHR$(&H66) + CHR$(&H8B) + CHR$(&H07) + CHR$(&HCB) + CHR$(0)
DEF SEG = VARSEG(code$)
CALL ABSOLUTE(SADD(code$))
PRINT "prefixes ok"
' mov bx, [esp+4] (67 8B 5C 24 04) / mov word [bx], 1234 / retf 2
x = 7
code$ = CHR$(&H67) + CHR$(&H8B) + CHR$(&H5C) + CHR$(&H24) + CHR$(4)
code$ = code$ + CHR$(&HC7) + CHR$(7) + CHR$(&HD2) + CHR$(4) + CHR$(&HCA) + CHR$(2) + CHR$(0)
DEF SEG = VARSEG(code$)
CALL ABSOLUTE(x, SADD(code$))
DEF SEG
PRINT x
SYSTEM
