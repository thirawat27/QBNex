FUNCTION StdLib_NormalizeImportKey$ (module$)
    DIM importKey AS STRING
    DIM importChar AS STRING
    DIM resultText AS STRING

    importKey = LCASE$(LTRIM$(RTRIM$(module$)))
    IF LEN(importKey) = 0 THEN EXIT FUNCTION

    DO WHILE LEFT$(importKey, 7) = "stdlib."
        importKey = MID$(importKey, 8)
    LOOP
    IF LEFT$(importKey, 7) = "stdlib\" THEN importKey = MID$(importKey, 8)
    IF LEFT$(importKey, 7) = "stdlib/" THEN importKey = MID$(importKey, 8)

    IF RIGHT$(importKey, 4) = ".bas" THEN importKey = LEFT$(importKey, LEN(importKey) - 4)

    FOR i = 1 TO LEN(importKey)
        importChar = MID$(importKey, i, 1)
        IF importChar = "\" OR importChar = "/" THEN importChar = "."
        resultText = resultText + importChar
    NEXT

    DO WHILE INSTR(resultText, "..")
        i = INSTR(resultText, "..")
        resultText = LEFT$(resultText, i - 1) + RIGHT$(resultText, LEN(resultText) - i)
    LOOP

    IF LEFT$(resultText, 1) = "." THEN resultText = MID$(resultText, 2)
    IF RIGHT$(resultText, 1) = "." THEN resultText = LEFT$(resultText, LEN(resultText) - 1)

    StdLib_NormalizeImportKey$ = resultText
END FUNCTION

FUNCTION StdLib_CanonicalImportKey$ (module$)
    DIM normalizedKey AS STRING

    normalizedKey = StdLib_NormalizeImportKey$(module$)

    SELECT CASE normalizedKey
    CASE "qbnex", "stdlib", "stdlib.all", "qbnex_stdlib", "all"
        StdLib_CanonicalImportKey$ = "qbnex"
    CASE ELSE
        StdLib_CanonicalImportKey$ = normalizedKey
    END SELECT
END FUNCTION

FUNCTION StdLib_ImportPath$ (module$)
    DIM normalizedKey AS STRING
    DIM relativeModulePath AS STRING
    DIM importChar AS STRING

    normalizedKey = StdLib_CanonicalImportKey$(module$)
    IF LEN(normalizedKey) = 0 THEN EXIT FUNCTION

    IF normalizedKey = "qbnex" THEN
        StdLib_ImportPath$ = QBNexRoot$ + "compiler" + pathsep$ + "stdlib" + pathsep$ + "stdlib.bas"
        EXIT FUNCTION
    END IF

    FOR i = 1 TO LEN(normalizedKey)
        importChar = MID$(normalizedKey, i, 1)
        IF importChar = "." THEN importChar = pathsep$
        relativeModulePath = relativeModulePath + importChar
    NEXT

    StdLib_ImportPath$ = QBNexRoot$ + "compiler" + pathsep$ + "stdlib" + pathsep$ + relativeModulePath + ".bas"
END FUNCTION

FUNCTION StdLib_QueueImport$ (module$)
    DIM normalizedKey AS STRING
    DIM importPath AS STRING

    normalizedKey = StdLib_CanonicalImportKey$(module$)
    IF LEN(normalizedKey) = 0 THEN
        Give_Error "Expected $IMPORT:'module.name'"
        EXIT FUNCTION
    END IF

    IF normalizedKey = "qbnex" THEN
        IF importedModules$ <> "@" THEN EXIT FUNCTION
    ELSE
        IF INSTR(importedModules$, "@qbnex@") THEN EXIT FUNCTION
    END IF

    IF INSTR(importedModules$, "@" + normalizedKey + "@") THEN EXIT FUNCTION

    importedModules$ = importedModules$ + normalizedKey + "@"
    importPath = StdLib_ImportPath$(normalizedKey)
    StdLib_QueueImport$ = importPath
END FUNCTION

'-------------------------------------------------------------------------------
' Hoisted imports: '$IMPORT lines of the main source are resolved before the
' compile passes. Each module is split into its declaration head (lines before
' its first SUB/FUNCTION) and its procedure body. All heads are included before
' the program's first line and all bodies after its last line, so module-level
' DIM/REDIM SHARED stay in the main module and the program's own code after an
' import is not "after a SUB" (the top-level runtime lowering, D-06).
'-------------------------------------------------------------------------------

FUNCTION StdLib_MetaArgFromLine$ (rawLine$, meta$)
    DIM t AS STRING
    DIM p AS LONG
    DIM q1 AS LONG
    DIM q2 AS LONG

    t = LTRIM$(RTRIM$(rawLine$))
    DO WHILE LEFT$(t, 1) = "'"
        t = LTRIM$(MID$(t, 2))
    LOOP
    IF UCASE$(LEFT$(t, 4)) = "REM " THEN t = LTRIM$(MID$(t, 5))
    IF UCASE$(LEFT$(t, LEN(meta$))) <> meta$ THEN EXIT FUNCTION
    p = INSTR(t, ":")
    IF p = 0 THEN EXIT FUNCTION
    q1 = INSTR(p, t, "'")
    IF q1 = 0 THEN EXIT FUNCTION
    q2 = INSTR(q1 + 1, t, "'")
    IF q2 = 0 THEN EXIT FUNCTION
    StdLib_MetaArgFromLine$ = MID$(t, q1 + 1, q2 - q1 - 1)
END FUNCTION

FUNCTION StdLib_ReadText$ (path$)
    DIM fh AS LONG
    DIM text AS STRING

    fh = FREEFILE
    OPEN path$ FOR BINARY AS #fh
    IF LOF(fh) THEN text = SPACE$(LOF(fh)): GET #fh, , text
    CLOSE #fh
    StdLib_ReadText$ = text
END FUNCTION

SUB StdLib_SplitIncludeRange (spec$, fromLine AS LONG, toLine AS LONG)
    'an include written as 'file|from|to' reads only that line range of the file (hoisted imports);
    'the reader keeps the real file name and line numbers for diagnostics
    DIM p AS LONG
    DIM q AS LONG

    fromLine = 0
    toLine = 0
    p = INSTR(spec$, "|")
    IF p = 0 THEN EXIT SUB
    q = INSTR(p + 1, spec$, "|")
    IF q = 0 THEN EXIT SUB
    fromLine = VAL(MID$(spec$, p + 1, q - p - 1))
    toLine = VAL(MID$(spec$, q + 1))
    spec$ = LEFT$(spec$, p - 1)
END SUB

SUB StdLib_EmitRange (path$, fromLine AS LONG, toLine AS LONG, inBody AS INTEGER, headText$, bodyText$)
    DIM directive AS STRING

    IF fromLine > toLine THEN EXIT SUB
    directive = "'$INCLUDE:'" + path$ + "|" + LTRIM$(STR$(fromLine)) + "|" + LTRIM$(STR$(toLine)) + "'" + CHR$(13) + CHR$(10)
    IF inBody THEN bodyText$ = bodyText$ + directive ELSE headText$ = headText$ + directive
END SUB

SUB StdLib_HoistFile (path$, headText$, bodyText$, depth AS LONG)
    'split one file into declaration ranges (before its first SUB/FUNCTION) and procedure ranges;
    'its relative $INCLUDE files are split the same way, its $IMPORT dependencies are hoisted first
    DIM text AS STRING
    DIM lineText AS STRING
    DIM upperText AS STRING
    DIM depKey AS STRING
    DIM incName AS STRING
    DIM incPath AS STRING
    DIM inBody AS INTEGER
    DIM inDeclare AS INTEGER
    DIM lineNo AS LONG
    DIM segStart AS LONG
    DIM p AS LONG
    DIM nl AS LONG

    text = StdLib_ReadText$(path$)
    segStart = 1
    p = 1
    DO WHILE p <= LEN(text)
        nl = INSTR(p, text, CHR$(10))
        IF nl = 0 THEN nl = LEN(text) + 1
        lineText = MID$(text, p, nl - p)
        IF RIGHT$(lineText, 1) = CHR$(13) THEN lineText = LEFT$(lineText, LEN(lineText) - 1)
        p = nl + 1
        lineNo = lineNo + 1

        depKey = StdLib_MetaArgFromLine$(lineText, "$IMPORT")
        IF LEN(depKey) THEN StdLib_HoistModule depKey, headText$, bodyText$ 'the line itself stays, as a no-op
        IF SyntaxExt_LineHasTry%(lineText) THEN syntaxExtUsesTry = -1

        incPath = ""
        incName = StdLib_MetaArgFromLine$(lineText, "$INCLUDE")
        IF LEN(incName) AND depth < 20 THEN
            incPath = getfilepath$(path$) + incName
            IF _FILEEXISTS(incPath) = 0 THEN incPath = ""
        END IF
        IF LEN(incPath) THEN
            StdLib_EmitRange path$, segStart, lineNo - 1, inBody, headText$, bodyText$
            StdLib_HoistFile incPath, headText$, bodyText$, depth + 1
            segStart = lineNo + 1
        ELSEIF inBody = 0 THEN
            upperText = UCASE$(LTRIM$(lineText))
            'SUB/FUNCTION prototypes inside DECLARE [DYNAMIC|CUSTOMTYPE|STATIC] LIBRARY
            '... END DECLARE are declarations: they stay in the head (R12b 541f49d).
            IF inDeclare THEN
                IF LEFT$(upperText, 11) = "END DECLARE" THEN inDeclare = 0
            ELSEIF LEFT$(upperText, 8) = "DECLARE " AND INSTR(upperText, " LIBRARY") > 0 THEN
                inDeclare = -1
            ELSEIF LEFT$(upperText, 4) = "SUB " OR LEFT$(upperText, 9) = "FUNCTION " THEN
                StdLib_EmitRange path$, segStart, lineNo - 1, 0, headText$, bodyText$
                segStart = lineNo
                inBody = -1
            END IF
        END IF
    LOOP
    StdLib_EmitRange path$, segStart, lineNo, inBody, headText$, bodyText$
END SUB

SUB StdLib_HoistModule (module$, headText$, bodyText$)
    DIM moduleKey AS STRING
    DIM modulePath AS STRING

    moduleKey = StdLib_CanonicalImportKey$(module$)
    IF LEN(moduleKey) = 0 THEN EXIT SUB
    'same de-duplication rules as StdLib_QueueImport$
    IF moduleKey = "qbnex" THEN
        IF stdlibHoisted$ <> "@" THEN EXIT SUB
    ELSE
        IF INSTR(stdlibHoisted$, "@qbnex@") THEN EXIT SUB
    END IF
    IF INSTR(stdlibHoisted$, "@" + moduleKey + "@") THEN EXIT SUB
    modulePath = StdLib_ImportPath$(moduleKey)
    IF _FILEEXISTS(modulePath) = 0 THEN EXIT SUB 'left to the in-place $IMPORT, which reports it
    stdlibHoisted$ = stdlibHoisted$ + moduleKey + "@"
    'the in-place $IMPORT lines (also inside modules) become no-ops
    importedModules$ = stdlibHoisted$
    StdLib_HoistFile modulePath, headText$, bodyText$, 0
END SUB

SUB StdLib_PrepareHoistedImports
    DIM text AS STRING
    DIM lineText AS STRING
    DIM moduleName AS STRING
    DIM headText AS STRING
    DIM bodyText AS STRING
    DIM vwatchDir AS STRING
    DIM fh AS LONG
    DIM p AS LONG
    DIM nl AS LONG
    DIM usesTry AS INTEGER
    DIM codeText AS STRING
    DIM commentText AS STRING

    stdlibHoisted$ = "@"
    syntaxExtUsesTry = 0
    stdlibHeadFile$ = ""
    stdlibBodyFile$ = ""
    IF _FILEEXISTS(sourcefile$) = 0 THEN EXIT SUB

    text = StdLib_ReadText$(sourcefile$)
    p = 1
    DO WHILE p <= LEN(text)
        nl = INSTR(p, text, CHR$(10))
        IF nl = 0 THEN nl = LEN(text) + 1
        lineText = MID$(text, p, nl - p)
        p = nl + 1
        moduleName = StdLib_MetaArgFromLine$(lineText, "$IMPORT")
        IF LEN(moduleName) THEN StdLib_HoistModule moduleName, headText, bodyText
        IF RIGHT$(lineText, 1) = CHR$(13) THEN lineText = LEFT$(lineText, LEN(lineText) - 1)
        IF SyntaxExt_LineHasTry%(lineText) THEN usesTry = -1
        'TRY inside an $INCLUDE file of the main source needs the handler too
        moduleName = StdLib_MetaArgFromLine$(lineText, "$INCLUDE")
        IF LEN(moduleName) THEN
            IF StdLib_FileUsesTry%(getfilepath$(sourcefile$) + moduleName, 0) THEN usesTry = -1
            IF StdLib_FileUsesTry%(moduleName, 0) THEN usesTry = -1
        END IF
    LOOP
    IF syntaxExtUsesTry THEN usesTry = -1 'a hoisted module uses TRY
    IF stdlibHoisted$ = "@" AND usesTry = 0 AND LEN(defFnBodyText$) = 0 THEN EXIT SUB

    'FUNCTIONs made from single-line DEF FN (deffn.bas) go after the program's last line
    bodyText = bodyText + defFnBodyText$

    'the in-place $IMPORT lines become no-ops
    IF stdlibHoisted$ <> "@" THEN importedModules$ = stdlibHoisted$

    'TRY ... END TRY (syntax_ext.bas) needs one module-level error handler
    IF usesTry THEN
        headText = "DIM SHARED QBNEX_TRYERR AS LONG" + CHR$(13) + CHR$(10) + "GOTO QBNEX_TRY_SKIP" + CHR$(13) + CHR$(10) + "QBNEX_TRY_HANDLER:" + CHR$(13) + CHR$(10) + "QBNEX_TRYERR = ERR" + CHR$(13) + CHR$(10) + "RESUME NEXT" + CHR$(13) + CHR$(10) + "QBNEX_TRY_SKIP:" + CHR$(13) + CHR$(10) + headText
    END IF

    'these files take the place of the vwatch includes at the first/last line
    vwatchDir = QBNexRoot$ + "compiler" + pathsep$ + "support" + pathsep$ + "vwatch" + pathsep$
    IF vWatchOn THEN
        headText = "'$INCLUDE:'" + vwatchDir + "vwatch.bi'" + CHR$(13) + CHR$(10) + headText
        bodyText = bodyText + "'$INCLUDE:'" + vwatchDir + "vwatch.bm'" + CHR$(13) + CHR$(10)
    ELSE
        bodyText = bodyText + "'$INCLUDE:'" + vwatchDir + "vwatch_stub.bm'" + CHR$(13) + CHR$(10)
    END IF

    'no head file when there is nothing to put first (a program with only DEF FN):
    'including an empty file reads past its end
    IF LEN(headText) THEN
        stdlibHeadFile$ = tmpdir$ + "imports_head.bas"
        fh = FREEFILE
        OPEN stdlibHeadFile$ FOR OUTPUT AS #fh
        PRINT #fh, headText;
        CLOSE #fh
    END IF
    stdlibBodyFile$ = tmpdir$ + "imports_body.bas"
    fh = FREEFILE
    OPEN stdlibBodyFile$ FOR OUTPUT AS #fh
    PRINT #fh, bodyText;
    CLOSE #fh
END SUB
FUNCTION StdLib_FileUsesTry% (path$, depth AS LONG)
    'a TRY statement in this file or in one of its relative $INCLUDE files
    DIM text AS STRING
    DIM lineText AS STRING
    DIM incName AS STRING
    DIM p AS LONG
    DIM nl AS LONG

    IF depth > 20 THEN EXIT FUNCTION
    IF _FILEEXISTS(path$) = 0 THEN EXIT FUNCTION
    text = StdLib_ReadText$(path$)
    p = 1
    DO WHILE p <= LEN(text)
        nl = INSTR(p, text, CHR$(10))
        IF nl = 0 THEN nl = LEN(text) + 1
        lineText = MID$(text, p, nl - p)
        p = nl + 1
        IF RIGHT$(lineText, 1) = CHR$(13) THEN lineText = LEFT$(lineText, LEN(lineText) - 1)
        IF SyntaxExt_LineHasTry%(lineText) THEN StdLib_FileUsesTry% = -1: EXIT FUNCTION
        incName = StdLib_MetaArgFromLine$(lineText, "$INCLUDE")
        IF LEN(incName) THEN
            IF StdLib_FileUsesTry%(getfilepath$(path$) + incName, depth + 1) THEN StdLib_FileUsesTry% = -1: EXIT FUNCTION
        END IF
    LOOP
END FUNCTION