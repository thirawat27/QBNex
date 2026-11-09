$CONSOLE:ONLY
FOR v = -1 TO 12 STEP 3
    SELECT CASE v
        CASE IS < 0: PRINT v; "neg"
        CASE 0 TO 4: PRINT v; "low"
        CASE 5, 8: PRINT v; "five-or-eight"
        CASE ELSE: PRINT v; "else"
    END SELECT
NEXT
s$ = "beta"
SELECT CASE s$
    CASE "alpha": PRINT "A"
    CASE "b" TO "c": PRINT "B-range"
END SELECT
SELECT EVERYCASE 6
    CASE IS > 1: PRINT "gt1"
    CASE IS > 5: PRINT "gt5"
    CASE IS > 9: PRINT "gt9"
END SELECT
SYSTEM
