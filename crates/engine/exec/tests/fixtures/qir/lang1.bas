$CONSOLE:ONLY
'$INCLUDE: 'inc.bi'
DEFINT A-Z
CONST LIMIT = 5, PI2# = 6.283185307179586#
DIM SHARED total AS LONG
DIM pts(1 TO 3) AS Pt
FOR k = 1 TO 3
    pts(k).x = k * 10: pts(k).y = k / 4: pts(k).nm = "p" + LTRIM$(STR$(k))
NEXT
PRINT GREETING$; LIMIT; PI2#
FOR k = 1 TO 3: PRINT pts(k).x; pts(k).y; "["; pts(k).nm; "]": NEXT
PRINT Fib&(15); Fact#(20)
Counter: Counter: Counter
PRINT "total"; total
a$ = "hello": Upper a$: PRINT a$
x = 7: Twice (x): PRINT x: Twice x: PRINT x
ON ERROR GOTO handler
PRINT Risky(0)
PRINT "after"
GOSUB sub1
i& = 2147483647: i& = i& + 0: PRINT i&; -i& - 1
u~% = 65535: PRINT u~%; u~% + 1
b%% = 127: PRINT b%%
q## = 1.5: PRINT q##
PRINT 7 \ 2; -7 \ 2; 7 MOD -3; -7 MOD 3; 2 ^ 10; 5 AND 3; 5 OR 3; 5 XOR 3; NOT 5; 5 EQV 3; 5 IMP 3
PRINT 1 / 3; 2 / 3#; 1E+20; 123456789; 0.1 + 0.2
s$ = "a,b,,c": n = 0
DO
    p = INSTR(s$, ",")
    IF p = 0 THEN EXIT DO
    n = n + 1: s$ = MID$(s$, p + 1)
LOOP
PRINT n; s$
SYSTEM
handler:
PRINT "err"; ERR; "line"; ERL
RESUME NEXT
sub1:
PRINT "in gosub": RETURN

FUNCTION Fib& (n)
    IF n < 2 THEN Fib& = n: EXIT FUNCTION
    Fib& = Fib&(n - 1) + Fib&(n - 2)
END FUNCTION

FUNCTION Fact# (n)
    IF n <= 1 THEN Fact# = 1 ELSE Fact# = n * Fact#(n - 1)
END FUNCTION

SUB Counter STATIC
    c = c + 1
    total = total + c
END SUB

SUB Upper (t$)
    t$ = UCASE$(t$)
END SUB

SUB Twice (v)
    v = v * 2
END SUB

FUNCTION Risky (z)
    IF z = 0 THEN ERROR 5
END FUNCTION
