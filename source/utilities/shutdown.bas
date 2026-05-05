SUB FinalizeBuildStatus (outputBaseName AS STRING)
    DIM finalizedOutputPath AS STRING

    finalizedOutputPath = RTRIM$(pendingOutputBinary$)
    IF finalizedOutputPath = "" THEN finalizedOutputPath = path.exe$ + outputBaseName + extension$
    finalizedOutputPath = ResolveOutputBinaryPath$(finalizedOutputPath)

    IF _FILEEXISTS(finalizedOutputPath) THEN
        compfailed = 0
        lastBinaryGenerated$ = finalizedOutputPath
    ELSE
        compfailed = 1
    END IF

    IF compfailed THEN
        PRINT "ERROR: Build failed."
        PRINT "Check " + compilelog$ + " for details."
    ELSE
        IF NOT QuietMode THEN PRINT "Build complete: "; lastBinaryGenerated$
    END IF
END SUB

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

SUB FinalizeCompilerRun (outputBaseName AS STRING)
    IF No_C_Compile_Mode THEN
        compfailed = 0
    ELSE
        FinalizeBuildStatus outputBaseName
    END IF
    ExitCompilerProcess
END SUB
