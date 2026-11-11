$CONSOLE:ONLY
' ON (expression) GOTO/GOSUB: the selector in parentheses, also a relational
ST = 0
ON (1 - (ST < 1)) GOTO 10, 20
10 PRINT "ten": GOTO 30
20 PRINT "twenty"
30 k = 2
ON (k) GOSUB 100, 200
ON (k - 1) * 1 GOSUB 100, 200
SYSTEM
100 PRINT "g100": RETURN
200 PRINT "g200": RETURN
