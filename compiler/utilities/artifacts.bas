FUNCTION EscapeUnixShellPath$ (pathValue$)
    EscapeUnixShellPath$ = StrReplace$(pathValue$, "'", "'\''")
END FUNCTION

FUNCTION QuotedFilename$ (f$)

    IF os$ = "WIN" THEN
        QuotedFilename$ = CHR$(34) + f$ + CHR$(34)
        EXIT FUNCTION
    END IF

    IF os$ = "LNX" THEN
        QuotedFilename$ = "'" + EscapeUnixShellPath$(f$) + "'"
        EXIT FUNCTION
    END IF

    QuotedFilename$ = CHR$(34) + f$ + CHR$(34)
END FUNCTION

FUNCTION ResolveAbsoluteDirectory$ (pathCandidate$)
    DIM currentdir AS STRING
    DIM resolvedPath AS STRING

    IF LEN(pathCandidate$) = 0 THEN EXIT FUNCTION
    IF _DIREXISTS(pathCandidate$) = 0 THEN EXIT FUNCTION

    currentdir = _CWD$
    CHDIR pathCandidate$
    resolvedPath = _CWD$
    CHDIR currentdir

    IF LEN(resolvedPath) THEN
        IF RIGHT$(resolvedPath, 1) <> pathsep$ THEN resolvedPath = resolvedPath + pathsep$
    END IF

    ResolveAbsoluteDirectory$ = resolvedPath
END FUNCTION

FUNCTION IsCompilerRelativeOutputPath% (pathCandidate$)
    pathCandidate$ = RTRIM$(pathCandidate$)

    SELECT CASE pathCandidate$
    CASE "..\..\", "../../", ".\", "./"
        IsCompilerRelativeOutputPath% = -1
    CASE ELSE
        IsCompilerRelativeOutputPath% = 0
    END SELECT
END FUNCTION

'a compiler-relative output folder (the legacy "..\..\" and ".\") is the compiler's folder
FUNCTION ResolveCompilerRelativeOutputDirectory$ (pathCandidate$)
    DIM resolvedPath AS STRING

    IF LEN(pathCandidate$) = 0 THEN EXIT FUNCTION
    IF IsCompilerRelativeOutputPath%(pathCandidate$) = 0 THEN EXIT FUNCTION
    resolvedPath = _CWD$

    IF LEN(resolvedPath) THEN
        IF RIGHT$(resolvedPath, 1) <> pathsep$ THEN resolvedPath = resolvedPath + pathsep$
    END IF

    ResolveCompilerRelativeOutputDirectory$ = resolvedPath
END FUNCTION

FUNCTION ResolveOutputBinaryPath$ (outputPath$)
    DIM pathOut AS STRING
    DIM outputName AS STRING

    outputPath$ = RTRIM$(outputPath$)
    IF outputPath$ = "" THEN EXIT FUNCTION

    pathOut = getfilepath$(outputPath$)
    IF pathOut = "" THEN
        ResolveOutputBinaryPath$ = outputPath$
        EXIT FUNCTION
    END IF

    outputName = MID$(outputPath$, LEN(pathOut) + 1)
    IF IsCompilerRelativeOutputPath%(pathOut) THEN
        pathOut = ResolveCompilerRelativeOutputDirectory$(pathOut)
    ELSE
        pathOut = ResolveAbsoluteDirectory$(pathOut)
    END IF

    IF pathOut = "" THEN
        ResolveOutputBinaryPath$ = outputPath$
    ELSE
        ResolveOutputBinaryPath$ = pathOut + outputName
    END IF
END FUNCTION

SUB PreparePendingOutputBinary (sourceBaseName AS STRING)
    DIM outputName AS STRING
    DIM pathOut AS STRING

    pendingOutputBinary$ = path.exe$ + sourceBaseName + extension$
    pathOut = getfilepath$(pendingOutputBinary$)
    IF LEN(pathOut) THEN
        IF IsCompilerRelativeOutputPath%(pathOut) THEN
            pathOut = ResolveCompilerRelativeOutputDirectory$(pathOut)
        ELSE
            pathOut = ResolveAbsoluteDirectory$(pathOut)
        END IF
        IF LEN(pathOut) THEN pendingOutputBinary$ = pathOut + sourceBaseName + extension$
    END IF

    IF LEN(outputfile_cmd$) = 0 THEN EXIT SUB

    pathOut = getfilepath$(outputfile_cmd$)
    outputName = MID$(outputfile_cmd$, LEN(pathOut) + 1)
    IF outputName = "" THEN outputName = outputfile_cmd$
    outputName = RemoveFileExtension$(outputName)

    IF LEN(pathOut) THEN pathOut = ResolveAbsoluteDirectory$(pathOut)

    pendingOutputBinary$ = pathOut + outputName + extension$
END SUB

SUB WarnIfStaleOutputBinary
    DIM outputPath AS STRING
    DIM sourcePath AS STRING

    outputPath = RTRIM$(pendingOutputBinary$)
    sourcePath = RTRIM$(sourcefile$)

    outputPath = ResolveOutputBinaryPath$(outputPath)
    IF outputPath = "" THEN EXIT SUB
    IF sourcePath = "" THEN EXIT SUB
    IF _FILEEXISTS(outputPath) = 0 THEN EXIT SUB
    IF _FILEEXISTS(sourcePath) = 0 THEN EXIT SUB

    PRINT
    PRINT "Warning: Existing output was not updated because compilation failed."
    PRINT "Existing executable may be stale:"
    PRINT "  Source: "; sourcePath
    PRINT "  Output: "; outputPath
END SUB
