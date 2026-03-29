FOR I = 1 TO 3
    ON I GOSUB FirstCase, SecondCase, ThirdCase
NEXT I

X = 2
ON X GOTO SkipCase, TargetCase, FailCase

SkipCase:
PRINT "BAD-SKIP"
END

TargetCase:
PRINT "TARGET"
END

FailCase:
PRINT "BAD-FAIL"
END

FirstCase:
PRINT "G1"
RETURN

SecondCase:
PRINT "G2"
RETURN

ThirdCase:
PRINT "G3"
RETURN
