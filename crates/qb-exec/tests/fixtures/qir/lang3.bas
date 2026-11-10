$CONSOLE:ONLY
DEFDBL D
d1 = 1 / 7
PRINT d1; 1 / 7!; 1## / 3
FOR t! = 0 TO 1 STEP 0.25: PRINT t!;: NEXT: PRINT
FOR k = 10 TO 1 STEP -3: PRINT k;: NEXT: PRINT
FOR a = 1 TO 3
    FOR b = 1 TO 3
        IF b = 2 THEN EXIT FOR
        PRINT a * 10 + b;
    NEXT
NEXT
PRINT
n = 0
DO UNTIL n >= 3: n = n + 1: LOOP: PRINT n
DO: n = n - 1: LOOP WHILE n > 0: PRINT n
PRINT &HFF; &O17; &B1011; &HFFFF%; &HFFFFFFFF&
i% = 32767: i% = i% + 1: PRINT i%
l& = 2147483647: l& = l& + 1: PRINT l&
b~%% = 255: b~%% = b~%% + 1: PRINT b~%%
x# = 1E+300: PRINT x# * 10
PRINT 3 * 2 ^ 2; -2 ^ 2; (-2) ^ 2; 2 ^ -1; 10 MOD 3 * 2
c1 = 1 < 2: c2 = 2 < 1: c3 = "a" < "b": c4 = 1 = 1 AND 2 = 2: PRINT c1; c2; c3; c4
PRINT 5 / 2; 5 \ 2; CINT(5 / 2); 7.5 \ 2
STATIC_test
STATIC_test
REDIM v(1 TO 3) AS INTEGER
v(1) = 1: v(2) = 2: v(3) = 3
REDIM _PRESERVE v(1 TO 5) AS INTEGER
PRINT v(1); v(3); v(5); UBOUND(v)
ERASE v
ON ERROR GOTO 900
PRINT v(1)
PRINT "skip"
10 PRINT "line ten"
GOTO 20
PRINT "never"
20 PRINT "line twenty"
ERROR 77
PRINT "back"
CALL Thrower
PRINT "end"
SYSTEM
900 PRINT "handler"; ERR; ERL
IF ERR = 9 THEN RESUME NEXT
IF ERR = 77 THEN RESUME NEXT
RESUME NEXT

SUB STATIC_test
    STATIC cnt AS INTEGER, arr() AS INTEGER
    cnt = cnt + 1
    PRINT "static"; cnt
END SUB

SUB Thrower
    PRINT "in thrower"
    ERROR 61
    PRINT "after 61"
END SUB
