SUB ShowQbErrorDebugDisplay
    IF ConsoleMode THEN
        PRINT
    ELSE
        _AUTODISPLAY
        SCREEN _NEWIMAGE(80, 25, 0), , 0, 0
        COLOR 7, 0
    END IF

    _CONTROLCHR OFF
    PRINT "A QB error has occurred (and you have compiled in debugging support)."
    PRINT "Some key information (qbnex.bas):"
    PRINT "Error"; ERR
    PRINT "Description: "; _ERRORMESSAGE$
    PRINT "Line"; _ERRORLINE
    IF _INCLERRORLINE THEN
        PRINT "Included line"; _INCLERRORLINE
        PRINT "Included file "; _INCLERRORFILE$
    END IF
    PRINT
    PRINT "Loaded source file details:"
    PRINT "qberrorhappened ="; qberrorhappened; "qberrorhappenedvalue ="; qberrorhappenedvalue; "linenumber ="; linenumber
    PRINT "ca$ = {"; ca$; "}"
    PRINT "linefragment = {"; linefragment; "}"
    END
END SUB

SUB LogQbErrorDetails
    IF Debug THEN PRINT #9, "QB ERROR!"
    IF Debug THEN PRINT #9, "ERR="; ERR
    IF Debug THEN PRINT #9, "ERL="; ERL
END SUB

FUNCTION UnusedVariableBaseKey$ (rawName$)
    DIM normalizedName AS STRING

    normalizedName = UCASE$(RTRIM$(rawName$))
    IF RIGHT$(normalizedName, 2) = "()" THEN normalizedName = LEFT$(normalizedName, LEN(normalizedName) - 2)

    UnusedVariableBaseKey$ = normalizedName
END FUNCTION

SUB ReconcileSharedIncludeUsageForWarnings
    DIM i AS LONG
    DIM j AS LONG
    DIM declKey AS STRING
    DIM useKey AS STRING
    DIM declType AS STRING
    DIM useType AS STRING
    DIM includeScoped AS _BYTE

    FOR i = 1 TO totalVariablesCreated
        IF usedVariableList(i).used = 0 THEN
            IF usedVariableList(i).scope = 0 THEN
                includeScoped = 0
                IF usedVariableList(i).includeLevel > 0 THEN includeScoped = -1
                IF LEN(RTRIM$(usedVariableList(i).includedFile)) > 0 THEN includeScoped = -1

                IF includeScoped THEN
                    declKey = UnusedVariableBaseKey$(usedVariableList(i).NAME)
                    IF LEN(declKey) > 0 THEN
                        FOR j = 1 TO totalVariablesCreated
                            IF i <> j THEN
                                IF usedVariableList(j).used <> 0 THEN
                                    IF usedVariableList(j).scope <> 0 THEN
                                        useKey = UnusedVariableBaseKey$(usedVariableList(j).NAME)
                                        IF useKey = declKey THEN
                                            IF usedVariableList(j).isarray = usedVariableList(i).isarray THEN
                                                declType = UCASE$(RTRIM$(usedVariableList(i).varType))
                                                useType = UCASE$(RTRIM$(usedVariableList(j).varType))
                                                IF declType = useType OR declType = "" OR useType = "" THEN
                                                    usedVariableList(i).used = -1
                                                    EXIT FOR
                                                END IF
                                            END IF
                                        END IF
                                    END IF
                                END IF
                            END IF
                        NEXT
                    END IF
                END IF
            END IF
        END IF
    NEXT
END SUB

SUB ReportUnusedVariableWarnings
    IF IgnoreWarnings THEN EXIT SUB

    ReconcileSharedIncludeUsageForWarnings

    totalUnusedVariables = 0
    FOR i = 1 TO totalVariablesCreated
        IF usedVariableList(i).used = 0 THEN
            totalUnusedVariables = totalUnusedVariables + 1
        END IF
    NEXT

    IF totalUnusedVariables = 0 THEN EXIT SUB

    maxVarNameLen = 0
    FOR i = 1 TO totalVariablesCreated
        IF usedVariableList(i).used = 0 THEN
            IF LEN(usedVariableList(i).NAME) > maxVarNameLen THEN maxVarNameLen = LEN(usedVariableList(i).NAME)
        END IF
    NEXT

    header$ = "unused variable"
    FOR i = 1 TO totalVariablesCreated
        IF usedVariableList(i).used = 0 THEN
            addWarning usedVariableList(i).linenumber, usedVariableList(i).includeLevel, usedVariableList(i).includedLine, usedVariableList(i).includedFile, header$, usedVariableList(i).NAME + SPACE$((maxVarNameLen + 1) - LEN(usedVariableList(i).NAME)) + "  " + usedVariableList(i).varType
        END IF
    NEXT
END SUB

SUB HandleFrontendErrorAndExit (errMessage$)
    DIM fullContext AS STRING
    DIM secondaryContext AS STRING
    DIM locationNote AS STRING
    DIM reportLineNumber AS LONG
    DIM reportFile AS STRING
    DIM fragmentContext AS STRING
    DIM processedContext AS STRING
    DIM frontendErrorKey AS STRING

    IF Error_Happened THEN
        errMessage$ = Error_Message
        Error_Happened = 0
    END IF

    layout$ = ""
    layoutok = 0

    IF forceIncludingFile THEN
        IF INSTR(errMessage$, "END SUB/FUNCTION before") THEN errMessage$ = "SUB without END SUB"
    ELSE
        IF inclevel > 0 THEN errMessage$ = errMessage$ + incerror$
    END IF

    fullContext = RTRIM$(diagnosticSourceLine)
    secondaryContext = ""
    locationNote = ""
    reportLineNumber = linenumber
    reportFile = sourcefile$
    fragmentContext = CleanDiagnosticContext$(linefragment)
    processedContext = CleanDiagnosticContext$(wholeline$)

    IF RTRIM$(fullContext) = "" THEN fullContext = processedContext
    IF RTRIM$(fullContext) = "" THEN fullContext = fragmentContext

    IF RTRIM$(processedContext) <> "" AND RTRIM$(processedContext) <> RTRIM$(fullContext) THEN secondaryContext = processedContext
    IF RTRIM$(fragmentContext) <> "" AND RTRIM$(fragmentContext) <> RTRIM$(fullContext) AND RTRIM$(fragmentContext) <> RTRIM$(secondaryContext) THEN
        IF RTRIM$(secondaryContext) = "" THEN secondaryContext = fragmentContext
    END IF

    IF inclevel > 0 THEN
        IF RTRIM$(incname$(inclevel)) <> "" THEN reportFile = incname$(inclevel)
        IF inclinenumber(inclevel) > 0 THEN reportLineNumber = inclinenumber(inclevel)
        locationNote = "Triggered while compiling an included module."
    END IF

    frontendErrorKey = UCASE$(NormalizeDiagnosticMessage$(errMessage$)) + "|" + UCASE$(RTRIM$(reportFile)) + "|" + LTRIM$(STR$(reportLineNumber))
    IF FrontendErrorHandled THEN
        IF RTRIM$(LastFrontendErrorKey) = RTRIM$(frontendErrorKey) THEN
            WarnIfStaleOutputBinary
            CleanupErrorHandler
            SYSTEM 1
        END IF
    END IF
    FrontendErrorHandled = -1
    LastFrontendErrorKey = frontendErrorKey

    SetCurrentFile reportFile
    SetErrorPhase "Legacy Frontend"
    PushErrorContext "syntax validation"
    IF RTRIM$(reportFile) <> "" THEN PushErrorContext "file: " + RTRIM$(reportFile)
    ReportDetailedError ERR_INVALID_SYNTAX, errMessage$, reportLineNumber, fullContext, secondaryContext, locationNote
    IF RTRIM$(reportFile) <> "" THEN PopErrorContext
    PopErrorContext
    ClearErrorPhase
    PrintAllErrors
    WarnIfStaleOutputBinary
    CleanupErrorHandler
    SYSTEM 1
END SUB
