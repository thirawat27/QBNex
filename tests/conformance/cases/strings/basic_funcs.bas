$CONSOLE:ONLY
a$ = "Hello, World"
PRINT LEN(a$); LEFT$(a$, 5); "|"; RIGHT$(a$, 5); "|"; MID$(a$, 8, 3)
PRINT UCASE$(a$); " "; LCASE$(a$)
PRINT INSTR(a$, "o"); INSTR(6, a$, "o"); INSTR(a$, "zz")
PRINT "["; LTRIM$("   x  "); "]["; RTRIM$("  y   "); "]"
PRINT STRING$(5, "*"); SPACE$(3); "|"; STRING$(3, 65)
PRINT ASC("A"); CHR$(66); ASC(a$, 2)
MID$(a$, 1, 5) = "Jello"
PRINT a$
PRINT STR$(42); "|"; STR$(-42); "|"; STR$(3.25)
PRINT VAL("  12.5abc"); VAL("&H10"); VAL("-3e2"); VAL("x")
b$ = ""
FOR i = 1 TO 1000: b$ = b$ + CHR$(48 + i MOD 10): NEXT
PRINT LEN(b$); RIGHT$(b$, 10)
PRINT _TRIM$("  both  "); "|"
SYSTEM
