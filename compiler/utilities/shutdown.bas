SUB ExitCompilerProcess
    IF HasErrors% THEN
        PrintAllErrors
        CleanupErrorHandler
        FOR closeall = 1 TO 255: CLOSE closeall: NEXT
        SYSTEM 1
    END IF

    IF compfailed <> 0 THEN
        CleanupErrorHandler
        FOR closeall = 1 TO 255: CLOSE closeall: NEXT
        SYSTEM 1
    END IF

    CleanupErrorHandler
    FOR closeall = 1 TO 255: CLOSE closeall: NEXT
    SYSTEM 0
END SUB
