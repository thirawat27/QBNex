SUB EnsureInvalidLineCapacity (targetLineNumber AS LONG)
    DIM newSize AS LONG

    IF targetLineNumber < UBOUND(InValidLine) THEN EXIT SUB

    newSize = UBOUND(InValidLine) * 2
    IF newSize < targetLineNumber + 1000 THEN newSize = targetLineNumber + 1000
    REDIM _PRESERVE InValidLine(1 TO newSize) AS _BYTE
END SUB

FUNCTION ResolveColorSupportInclude$ (colorDepth AS LONG)
    DIM colorFileName AS STRING

    IF colorDepth = 0 THEN
        colorFileName$ = "color0"
    ELSE
        colorFileName$ = "color32"
    END IF

    IF qbnexprefix_set THEN colorFileName$ = colorFileName$ + "_noprefix"

    ResolveColorSupportInclude$ = getfilepath$(COMMAND$(0)) + "internal" + pathsep$ + "support" + pathsep$ + "color" + pathsep$ + colorFileName$ + ".bi"
END FUNCTION

SUB ResetPrepassManagers
    addmetainclude$ = ""
    importedModules$ = "@"
    ClassSyntax_Reset
    classSyntaxDeferredQueue$ = ""
    topLevelRuntimeLines = ""
    topLevelRuntimeCallInjected = 0
    topLevelRuntimeFinalized = 0
    topLevelRuntimeProcDepth = 0
    topLevelRuntimeTypeDepth = 0
    topLevelRuntimeDeclareDepth = 0
    ClassSyntax_ClearRegistry
    ClassSyntax_ClearScopes
END SUB

SUB ResetPostPrepassState
    DataOffset = 0
    inclevel = 0
    subfuncn = 0
    lastLineReturn = 0
    lastLine = 0
    firstLine = 1
    UserDefineCount = 7

    FOR i = 0 TO constlast
        constdefined(i) = 0
    NEXT

    FOR i = 1 TO 27
        defineaz(i) = "SINGLE"
        defineextaz(i) = "!"
    NEXT
END SUB

SUB StartMainPassSession
    OPEN tmpdir$ + "ret0.txt" FOR OUTPUT AS #15
    PRINT #15, "if (next_return_point){"
    PRINT #15, "next_return_point--;"
    PRINT #15, "switch(return_point[next_return_point]){"
    PRINT #15, "case 0:"
    PRINT #15, "return;"
    PRINT #15, "break;"

    continueline = 0
    endifs = 0
    lineelseused = 0
    continuelinefrom = 0
    linenumber = 0
    reallinenumber = 0
    declaringlibrary = 0
    percentage = -1

    PRINT #12, "S_0:;"
    IF UseGL THEN gl_include_content
END SUB

FUNCTION NextPrepassLine$
    IF LEN(classSyntaxQueue$) THEN
        NextPrepassLine$ = ClassSyntax_DequeueLine$
        NextPrepassLine$ = TopLevelRuntime_ProcessLine$(NextPrepassLine$)
        EXIT FUNCTION
    END IF

    NextPrepassLine$ = lineinput3$
    IF NextPrepassLine$ = CHR$(13) THEN
        IF topLevelRuntimeFinalized = 0 THEN
            TopLevelRuntime_Finalize
            IF LEN(classSyntaxDeferredQueue$) THEN NextPrepassLine$ = ClassSyntax_DequeueDeferredLine$
        ELSEIF LEN(classSyntaxDeferredQueue$) THEN
            NextPrepassLine$ = ClassSyntax_DequeueDeferredLine$
        END IF
    ELSE
        NextPrepassLine$ = ClassSyntax_ProcessLine$(NextPrepassLine$)
        NextPrepassLine$ = TopLevelRuntime_ProcessLine$(NextPrepassLine$)
    END IF
END FUNCTION

FUNCTION NextMainPassLine$ (currentLine$)
    NextMainPassLine$ = currentLine$
    IF inclevel <> 0 THEN EXIT FUNCTION

    IF LEN(classSyntaxQueue$) THEN
        NextMainPassLine$ = ClassSyntax_DequeueLine$
        NextMainPassLine$ = TopLevelRuntime_ProcessLine$(NextMainPassLine$)
        EXIT FUNCTION
    END IF

    NextMainPassLine$ = lineinput3$
    IF NextMainPassLine$ = CHR$(13) THEN
        IF topLevelRuntimeFinalized = 0 THEN
            TopLevelRuntime_Finalize
            IF LEN(classSyntaxDeferredQueue$) THEN NextMainPassLine$ = ClassSyntax_DequeueDeferredLine$
        ELSEIF LEN(classSyntaxDeferredQueue$) THEN
            NextMainPassLine$ = ClassSyntax_DequeueDeferredLine$
        END IF
    ELSE
        NextMainPassLine$ = ClassSyntax_ProcessLine$(NextMainPassLine$)
        NextMainPassLine$ = TopLevelRuntime_ProcessLine$(NextMainPassLine$)
    END IF
END FUNCTION
