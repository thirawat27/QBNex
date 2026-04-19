SUB InitOptimizationModule
END SUB

SUB CleanupOptimizationModule
END SUB

SUB RecordHashLookup
END SUB

SUB RecordHashCollision
END SUB

SUB RecordStringOperation
END SUB

SUB RecordFileIO
END SUB

SUB EndPerformanceMetrics
END SUB

SUB PrintPerformanceReport
END SUB

SUB PrintCacheStats
END SUB

SUB InitDeferredReferences
    SinglePassMode = 0
END SUB

SUB InitMainHashTable
END SUB

SUB InitCompilationState
END SUB

SUB AddDeferredReference (refName AS STRING, lineNum AS LONG, refType AS INTEGER, scopeId AS LONG, refFlags AS LONG)
END SUB

FUNCTION IsDeferred% (refName AS STRING, refType AS INTEGER)
    IsDeferred% = 0
END FUNCTION

SUB ResolveDeferredReferences
END SUB

SUB ReportUnresolvedReferences
END SUB

FUNCTION GetDeferredResolvedIndex& (refName AS STRING, refType AS INTEGER)
    GetDeferredResolvedIndex& = 0
END FUNCTION

FUNCTION CanAvoidRecompile% (triggerType AS STRING)
    CanAvoidRecompile% = 0
END FUNCTION

FUNCTION GetUnresolvedCount%
    GetUnresolvedCount% = 0
END FUNCTION

SUB PrintDeferredStats
END SUB

SUB CleanupDeferredReferences
    SinglePassMode = 0
END SUB

SUB InitCompilerPhases
END SUB

SUB PrintPhaseReport
END SUB

SUB PrintParallelMetrics
END SUB

SUB CleanupParallelProcessing
END SUB

SUB CleanupIntegrationModule
END SUB

SUB CleanupParser
END SUB

SUB CleanupSymbolTable
END SUB

SUB CleanupCodeGenerator
END SUB

SUB InitErrorHandler
    DIM i AS INTEGER

    CompatVerboseDiagnostics = -1
    CompatHasErrors = 0
    CompatErrorCount = 0
    CompatMaxErrors = 100
    CompatCurrentFile = ""
    CompatCurrentPhase = ""
    CompatErrorContextDepth = 0

    FOR i = 1 TO 16
        CompatErrorContext(i) = ""
    NEXT
END SUB

SUB SetVerboseMode (enabled AS _BYTE)
    CompatVerboseDiagnostics = enabled
END SUB

SUB SetMaxErrors (maxCount AS LONG)
    CompatMaxErrors = maxCount
END SUB

SUB SetCurrentFile (fileName AS STRING)
    CompatCurrentFile = RTRIM$(fileName)
END SUB

SUB SetErrorPhase (phaseName AS STRING)
    CompatCurrentPhase = RTRIM$(phaseName)
END SUB

SUB ClearErrorPhase
    CompatCurrentPhase = ""
END SUB

SUB PushErrorContext (contextText AS STRING)
    contextText = LTRIM$(RTRIM$(contextText))
    IF contextText = "" THEN EXIT SUB

    IF CompatErrorContextDepth < 16 THEN
        CompatErrorContextDepth = CompatErrorContextDepth + 1
        CompatErrorContext(CompatErrorContextDepth) = contextText
    ELSE
        CompatErrorContext(16) = contextText
    END IF
END SUB

SUB PopErrorContext
    IF CompatErrorContextDepth <= 0 THEN EXIT SUB
    CompatErrorContext(CompatErrorContextDepth) = ""
    CompatErrorContextDepth = CompatErrorContextDepth - 1
END SUB

FUNCTION HasErrors%
    HasErrors% = CompatHasErrors
END FUNCTION

SUB PrintAllErrors
END SUB

SUB CleanupErrorHandler
    CompatCurrentFile = ""
    CompatCurrentPhase = ""
    CompatErrorContextDepth = 0
END SUB

SUB ReportError (errCode AS INTEGER, message AS STRING, lineNumber AS LONG, sourceContext AS STRING)
    ReportDetailedErrorWithSeverity errCode, ERR_ERROR, message, lineNumber, sourceContext, "", ""
END SUB

SUB ReportDetailedError (errCode AS INTEGER, message AS STRING, lineNumber AS LONG, mainContext AS STRING, secondaryContext AS STRING, locationNote AS STRING)
    ReportDetailedErrorWithSeverity errCode, ERR_ERROR, message, lineNumber, mainContext, secondaryContext, locationNote
END SUB

SUB ReportDetailedErrorWithSeverity (errCode AS INTEGER, severity AS INTEGER, message AS STRING, lineNumber AS LONG, mainContext AS STRING, secondaryContext AS STRING, locationNote AS STRING)
    CompatHasErrors = -1
    CompatErrorCount = CompatErrorCount + 1
    IF CompatErrorCount > CompatMaxErrors THEN EXIT SUB

    PRINT
    PRINT "[!] " + CompatSeverityLabel$(severity) + ": " + RTRIM$(message)

    IF RTRIM$(CompatCurrentFile) <> "" THEN
        IF lineNumber > 0 THEN
            PRINT "    at " + RTRIM$(CompatCurrentFile) + ":" + LTRIM$(STR$(lineNumber))
        ELSE
            PRINT "    at " + RTRIM$(CompatCurrentFile)
        END IF
    ELSEIF lineNumber > 0 THEN
        PRINT "    line " + LTRIM$(STR$(lineNumber))
    END IF

    IF RTRIM$(mainContext) <> "" THEN PRINT "    " + RTRIM$(mainContext)
    IF RTRIM$(secondaryContext) <> "" THEN PRINT "    note: " + RTRIM$(secondaryContext)
    IF RTRIM$(locationNote) <> "" THEN PRINT "    note: " + RTRIM$(locationNote)

    IF CompatVerboseDiagnostics THEN
        PRINT "[!] cause: " + CompatCauseText$(errCode, message)
        PRINT "[+] example: " + CompatExampleText$(errCode)
        PRINT "[::] flow: " + CompatBuildFlow$
    END IF
END SUB

FUNCTION CompatSeverityLabel$ (severity AS INTEGER)
    SELECT CASE severity
        CASE ERR_FATAL
            CompatSeverityLabel$ = "fatal"
        CASE ELSE
            CompatSeverityLabel$ = "error"
    END SELECT
END FUNCTION

FUNCTION CompatCauseText$ (errCode AS INTEGER, message AS STRING)
    SELECT CASE errCode
        CASE ERR_INVALID_SYNTAX
            CompatCauseText$ = "The parser found a statement shape that is not valid at this location."
        CASE ERR_ENCODING_ISSUE
            CompatCauseText$ = "The source file encoding is not supported for direct compilation."
        CASE ERR_INVALID_UTF8
            CompatCauseText$ = "The source file contains bytes that are not valid UTF-8."
        CASE ELSE
            CompatCauseText$ = "Compilation stopped because the current source state is inconsistent."
    END SELECT
END FUNCTION

FUNCTION CompatExampleText$ (errCode AS INTEGER)
    SELECT CASE errCode
        CASE ERR_INVALID_SYNTAX
            CompatExampleText$ = "Check statement order, missing keywords, or unmatched delimiters near the reported line."
        CASE ERR_ENCODING_ISSUE, ERR_INVALID_UTF8
            CompatExampleText$ = "Re-save the source file as UTF-8, then run the compiler again."
        CASE ELSE
            CompatExampleText$ = "Fix the reported source line and retry the compilation."
    END SELECT
END FUNCTION

FUNCTION CompatBuildFlow$ ()
    DIM i AS INTEGER
    DIM result AS STRING

    result = ""
    IF RTRIM$(CompatCurrentPhase) <> "" THEN result = RTRIM$(CompatCurrentPhase)

    FOR i = 1 TO CompatErrorContextDepth
        IF RTRIM$(CompatErrorContext(i)) <> "" THEN
            IF result <> "" THEN result = result + " -> "
            result = result + RTRIM$(CompatErrorContext(i))
        END IF
    NEXT

    IF result = "" THEN result = "compiler frontend"
    CompatBuildFlow$ = result
END FUNCTION
