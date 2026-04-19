SUB FinalizeBuildStatus (outputBaseName AS STRING)
    IF path.exe$ = "../../" OR path.exe$ = "..\..\" THEN path.exe$ = ""

    IF _FILEEXISTS(path.exe$ + outputBaseName + extension$) THEN
        compfailed = 0
        lastBinaryGenerated$ = path.exe$ + outputBaseName + extension$
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
    IF (compfailed <> 0 OR warningsissued <> 0) AND ConsoleMode = 0 THEN END 1
    IF compfailed <> 0 THEN SYSTEM 1

    IF HasErrors% THEN
        PrintAllErrors
    END IF

    CleanupErrorHandler
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
