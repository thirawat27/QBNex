$CONSOLE:ONLY
' Concatenation in loops: arena compaction and growth, many live strings.
a$ = ""
FOR i = 1 TO 20000
    a$ = a$ + CHR$(65 + (i MOD 26))
    IF i MOD 7 = 0 THEN a$ = a$ + STR$(i)
NEXT
PRINT LEN(a$); LEFT$(a$, 30); RIGHT$(a$, 12)
n = 0: p = INSTR(a$, "XYZ")
DO WHILE p
    n = n + 1
    p = INSTR(p + 1, a$, "XYZ")
LOOP
PRINT "XYZ count"; n
DIM s(500) AS STRING
FOR i = 0 TO 500
    s(i) = STRING$(i MOD 37, 48 + (i MOD 10)) + "|" + LTRIM$(STR$(i))
NEXT
FOR k = 1 TO 3
    FOR i = 0 TO 500 STEP 3
        s(i) = s(i) + s((i + k) MOD 501)
    NEXT
NEXT
t& = 0
FOR i = 0 TO 500: t& = t& + LEN(s(i)): NEXT
PRINT "total"; t&; s(0); " "; RIGHT$(s(500), 20)
b$ = SPACE$(100000)
FOR i = 1 TO 100000 STEP 9973: MID$(b$, i, 3) = "abc": NEXT
PRINT LEN(b$); INSTR(b$, "abc"); _INSTRREV(b$, "abc")
c$ = ""
FOR i = 1 TO 300: c$ = c$ + c$ + "x": IF LEN(c$) > 200000 THEN EXIT FOR
NEXT
PRINT "doubling"; i; LEN(c$)
SYSTEM
