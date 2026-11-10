$CONSOLE:ONLY
TRY
    ERROR 53
    PRINT "not here"
CATCH e
    PRINT "caught"; e
FINALLY
    PRINT "finally"
END TRY
TRY
    PRINT "ok body"
CATCH
    PRINT "no"
END TRY
Risky
PRINT "done"
SYSTEM

SUB Risky
    TRY
        ERROR 5
    CATCH c
        PRINT "sub caught"; c
    END TRY
END SUB
