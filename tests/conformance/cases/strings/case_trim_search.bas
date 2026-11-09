$CONSOLE:ONLY
' Case, trims, searches, conversions and comparisons on edge inputs.
s$ = "  Hello, World  "
PRINT "["; UCASE$(s$); "]["; LCASE$(s$); "]"
PRINT "["; LTRIM$(s$); "]["; RTRIM$(s$); "]["; _TRIM$(s$); "]"
PRINT "["; LTRIM$(SPACE$(5)); "]["; RTRIM$(""); "]"
PRINT INSTR(s$, "o"); INSTR(7, s$, "o"); INSTR(s$, ""); INSTR(3, s$, ""); INSTR("", "a"); INSTR(s$, "xyz")
PRINT _INSTRREV(s$, "o"); _INSTRREV(9, s$, "o"); _INSTRREV(s$, ""); _INSTRREV("aaa", "aa")
PRINT LEFT$(s$, 0); "|"; LEFT$(s$, 4); "|"; RIGHT$(s$, 4); "|"; RIGHT$(s$, 100); "|"
PRINT MID$(s$, 3, 5); "|"; MID$(s$, 14); "|"; MID$(s$, 40, 2); "|"; MID$(s$, 3, 0); "|"
PRINT ASC("A"); ASC("Zz", 2); LEN(STRING$(5, "q")); STRING$(3, 66); "|"; SPACE$(3); "|"
x$ = "apple": y$ = "apples": z$ = "Apple"
IF x$ < y$ THEN PRINT "lt";
IF y$ > x$ THEN PRINT " gt";
IF z$ < x$ THEN PRINT " upper-first";
IF x$ <= "apple" AND x$ >= "apple" THEN PRINT " eq";
IF "" < x$ THEN PRINT " empty-first";
IF x$ <> z$ THEN PRINT " ne"
PRINT _STRCMP(x$, y$); _STRCMP(y$, x$); _STRCMP(x$, x$); _STRICMP(x$, z$); _STRICMP("abd", "ABC")
FOR i = 0 TO 255 STEP 51: PRINT ASC(CHR$(i)); : NEXT: PRINT
t$ = LCASE$(UCASE$("MiXeD 123 !@#"))
PRINT t$; LEN(t$)
SYSTEM
