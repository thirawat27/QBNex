$CONSOLE:ONLY
OPTION BASE 1
TYPE Inner
    a AS INTEGER
    b AS DOUBLE
END TYPE
TYPE Outer
    id AS LONG
    in AS Inner
    tag AS STRING * 4
END TYPE
DIM SHARED g$
DIM o AS Outer, o2 AS Outer
o.id = 9: o.in.a = -3: o.in.b = 2.25: o.tag = "abcdef"
o2 = o: o2.in.a = 4
PRINT o.id; o.in.a; o.in.b; o.tag; o2.in.a; LEN(o)
DIM names$(3)
names$(1) = "pear": names$(2) = "apple": names$(3) = "fig"
SortStrings names$()
FOR i = LBOUND(names$) TO UBOUND(names$): PRINT names$(i); " ";: NEXT: PRINT
REDIM m(2, 3) AS LONG
FOR r = 1 TO 2: FOR c = 1 TO 3: m(r, c) = r * 10 + c: NEXT c, r
PRINT m(2, 3); UBOUND(m, 2)
g$ = "shared!"
ShowShared
PRINT Rev$("stressed")
FOR k = 1 TO 4
    SELECT CASE k * 3
        CASE IS < 4: PRINT "small";
        CASE 5 TO 7: PRINT "mid";
        CASE 9, 12: PRINT "nine-or-twelve";
        CASE ELSE: PRINT "other";
    END SELECT
NEXT
PRINT
SELECT CASE "kiwi"
    CASE "apple" TO "lemon": PRINT "in range"
    CASE ELSE: PRINT "out"
END SELECT
FOR k = 1 TO 3: ON k GOSUB s1, s2, s3: NEXT
w = 0
WHILE w < 5
    w = w + 1
    IF w = 2 THEN _CONTINUE
    IF w = 4 THEN EXIT WHILE
    PRINT "w"; w;
WEND
PRINT
DO
    READ v$
    IF v$ = "end" THEN EXIT DO
    PRINT "["; v$; "]";
LOOP
PRINT
a$ = "0123456789": MID$(a$, 3, 2) = "ab": PRINT a$
x = 1: y = 2: SWAP x, y: PRINT x; y
PRINT USING "##.# | &"; 3.14159; "pi"
PRINT USING "###,###.##"; 1234567.891
WRITE "a", 1, 2.5
PRINT CHR$(65); STRING$(3, "z"); SPACE$(2); "|"; HEX$(255); OCT$(8); LCASE$("ABC")
PRINT VAL("12abc"); VAL("&HFF"); STR$(-0.5); INT(-2.5); FIX(-2.5); CINT(2.5); CINT(3.5); ABS(-4); SGN(-9)
SYSTEM
DATA alpha, "be,ta", 3, end
s1: PRINT "one";: RETURN
s2: PRINT "two";: RETURN
s3: PRINT "three": RETURN

SUB SortStrings (arr$())
    FOR i = LBOUND(arr$) TO UBOUND(arr$) - 1
        FOR j = i + 1 TO UBOUND(arr$)
            IF arr$(j) < arr$(i) THEN SWAP arr$(i), arr$(j)
        NEXT
    NEXT
END SUB

SUB ShowShared
    PRINT g$
END SUB

FUNCTION Rev$ (s$)
    IF LEN(s$) <= 1 THEN Rev$ = s$ ELSE Rev$ = Rev$(MID$(s$, 2)) + LEFT$(s$, 1)
END FUNCTION
