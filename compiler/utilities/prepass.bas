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

    ResolveColorSupportInclude$ = QBNexRoot$ + "compiler" + pathsep$ + "support" + pathsep$ + "color" + pathsep$ + colorFileName$ + ".bi"
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
    topLevelRuntimeSeenProc = 0
    topLevelHoistScan = "": topLevelHoistWanted = 0: topLevelHoistInjected = 0: topLevelHoistSubSeen = 0
    topLevelHoistSrcFile = "": topLevelHoistSrcLine = 0: topLevelHoistFromMain = 0
    ClassSyntax_ClearRegistry
    ClassSyntax_ClearScopes
    SyntaxExt_Reset
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

    continueline = 0
    endifs = 0
    lineelseused = 0
    continuelinefrom = 0
    linenumber = 0
    reallinenumber = 0
    declaringlibrary = 0
    percentage = -1

    IF UseGL THEN gl_include_content 'SUB _GL declared: register _gl* SUB/FUNCTIONs and _GL_* constants
END SUB

FUNCTION NextPrepassLine$
    DIM processedLine$

    IF LEN(classSyntaxQueue$) THEN
        processedLine$ = ClassSyntax_DequeueLine$
        NextPrepassLine$ = TopLevelRuntime_ProcessLine$(processedLine$)
        EXIT FUNCTION
    END IF

    processedLine$ = lineinput3$
    NextPrepassLine$ = processedLine$
    IF processedLine$ = CHR$(13) THEN
        IF topLevelRuntimeFinalized = 0 THEN
            TopLevelRuntime_Finalize
            IF LEN(classSyntaxDeferredQueue$) THEN NextPrepassLine$ = ClassSyntax_DequeueDeferredLine$
        ELSEIF LEN(classSyntaxDeferredQueue$) THEN
            NextPrepassLine$ = ClassSyntax_DequeueDeferredLine$
        END IF
    ELSE
        processedLine$ = ClassSyntax_ProcessLine$(processedLine$)
        NextPrepassLine$ = TopLevelRuntime_ProcessLine$(processedLine$)
    END IF
END FUNCTION

FUNCTION NextMainPassLine$ (currentLine$)
    DIM processedLine$

    processedLine$ = currentLine$
    NextMainPassLine$ = processedLine$
    IF inclevel <> 0 THEN EXIT FUNCTION

    IF LEN(classSyntaxQueue$) THEN
        processedLine$ = ClassSyntax_DequeueLine$
        NextMainPassLine$ = TopLevelRuntime_ProcessLine$(processedLine$)
        EXIT FUNCTION
    END IF

    processedLine$ = lineinput3$
    NextMainPassLine$ = processedLine$
    IF processedLine$ = CHR$(13) THEN
        IF topLevelRuntimeFinalized = 0 THEN
            TopLevelRuntime_Finalize
            IF LEN(classSyntaxDeferredQueue$) THEN NextMainPassLine$ = ClassSyntax_DequeueDeferredLine$
        ELSEIF LEN(classSyntaxDeferredQueue$) THEN
            NextMainPassLine$ = ClassSyntax_DequeueDeferredLine$
        END IF
    ELSE
        processedLine$ = ClassSyntax_ProcessLine$(processedLine$)
        NextMainPassLine$ = TopLevelRuntime_ProcessLine$(processedLine$)
    END IF
END FUNCTION
