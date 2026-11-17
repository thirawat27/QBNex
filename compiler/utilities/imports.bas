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
    StdLib_CanonicalImportKey$ = StdLib_NormalizeImportKey$(module$)
END FUNCTION

FUNCTION StdLib_ImportPath$ (module$)
    DIM normalizedKey AS STRING
    DIM relativeModulePath AS STRING
    DIM importChar AS STRING

    normalizedKey = StdLib_CanonicalImportKey$(module$)
    IF LEN(normalizedKey) = 0 THEN EXIT FUNCTION

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

    IF INSTR(importedModules$, "@" + normalizedKey + "@") THEN EXIT FUNCTION

    importedModules$ = importedModules$ + normalizedKey + "@"
    importPath = StdLib_ImportPath$(normalizedKey)
    IF _FILEEXISTS(importPath) = 0 THEN
        Give_Error "No standard library module named '" + normalizedKey + "' (modules: core, collections, strings, json, url, path, csv, math, sys, result, regex, hash, zip, http, utf8, datetime, qbnex)"
        EXIT FUNCTION
    END IF
    StdLib_QueueImport$ = importPath
END FUNCTION

'-------------------------------------------------------------------------------
' Hoisted imports: '$IMPORT lines of the main source are resolved before the
' compile passes. Each module is split into its declaration head (lines before
' its first SUB/FUNCTION) and its procedure body. All heads are included before
' the program's first line and all bodies after its last line, so module-level
' DIM/REDIM SHARED stay in the main module and the program's own code after an
' import is not "after a SUB" (the top-level runtime lowering).
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
    'the meta command itself, not one that begins with its name ($INCLUDEONCE)
    p = INSTR(" :" + CHR$(9), MID$(t, LEN(meta$) + 1, 1))
    IF p = 0 OR LEN(t) = LEN(meta$) THEN EXIT FUNCTION
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

FUNCTION StdLib_FindInclude$ (folder$, name$, fromFolder AS LONG)
    'The file an $INCLUDE of name$ in a file in folder$ opens, found as the include managers
    'find it (ResolveIncludeFile$). fromFolder is -1 when it is relative to folder$. "" when
    'there is none.
    DIM mode AS LONG, foundIn AS LONG

    IF IsCompilerFile%(folder$) THEN mode = 1
    StdLib_FindInclude$ = ResolveIncludeFile$(name$, folder$, mode, foundIn)
    fromFolder = (foundIn = 1)
END FUNCTION

'The folder an $INCLUDE is looked for in, k-th in the search order (mode 0: a user's file,
'1: a file of the compiler, 2: a file the compiler includes at the first or last line).
'A user's file is looked for next to the including file (in folder$), next to the main
'source file, then in the folder qb was started in: never in the QBNex folder the compiler
'works in, where it found QBNex's own files (README.md). The compiler's files are next to
'the including file or in the QBNex folder. "" when there is no k-th folder.
FUNCTION IncludeSearchFolder$ (k AS LONG, folder$, mode AS LONG)
    SELECT CASE mode * 10 + k
    CASE 1, 11: IncludeSearchFolder$ = folder$
    CASE 2: IncludeSearchFolder$ = getfilepath$(sourcefile$)
    CASE 3: IncludeSearchFolder$ = JoinPath$(_STARTDIR$, "")
    CASE 12, 21: IncludeSearchFolder$ = QBNexRoot$
    END SELECT
END FUNCTION

'The file an $INCLUDE of name$ opens; folder$ is the including file's folder. An absolute
'path is taken as written, a relative one is looked for in the IncludeSearchFolder$ order:
'each name as written first, then (Linux, macOS) with another letter case
'(ResolvePathCase$), so an exact match anywhere wins. foundIn is the folder's number in that
'order. Returns: the path to open, or "" when there is none.
FUNCTION ResolveIncludeFile$ (name$, folder$, mode AS LONG, foundIn AS LONG)
    DIM nm AS STRING, d AS STRING, f AS STRING, k AS LONG, pass AS LONG

    foundIn = 0
    nm = name$
    DO WHILE LEFT$(nm, 2) = "./" OR LEFT$(nm, 2) = ".\"
        nm = MID$(nm, 3)
    LOOP
    FOR pass = 1 TO 2
        FOR k = 1 TO 3
            f = ""
            IF IsAbsolutePath%(nm) THEN
                IF k = 1 THEN f = nm
            ELSE
                d = IncludeSearchFolder$(k, folder$, mode)
                IF LEN(d) THEN f = Path_Normalize$(d + nm)
            END IF
            IF LEN(f) AND pass = 2 THEN f = ResolvePathCase$(f)
            IF LEN(f) THEN
                IF _FILEEXISTS(f) THEN ResolveIncludeFile$ = f: foundIn = k: EXIT FUNCTION
            END IF
        NEXT
    NEXT
END FUNCTION

'"File name not found", with the folders it was looked for in.
FUNCTION IncludeNotFoundText$ (name$, folder$, mode AS LONG)
    DIM r AS STRING, d AS STRING, seen AS STRING, k AS LONG

    r = "File " + name$ + " not found"
    IF IsAbsolutePath%(name$) THEN IncludeNotFoundText$ = r: EXIT FUNCTION
    seen = CHR$(0)
    FOR k = 1 TO 3
        d = IncludeSearchFolder$(k, folder$, mode)
        IF LEN(d) THEN
            IF INSTR(seen, CHR$(0) + d + CHR$(0)) = 0 THEN
                IF seen = CHR$(0) THEN r = r + " (looked in " ELSE r = r + ", "
                r = r + d
                seen = seen + d + CHR$(0)
            END IF
        END IF
    NEXT
    IF seen <> CHR$(0) THEN r = r + ")"
    IncludeNotFoundText$ = r
END FUNCTION

'A file of the compiler: under the QBNex folder's compiler folder (the stdlib modules), or
'a generated import file; its own includes may be in the QBNex folder.
FUNCTION IsCompilerFile% (f$)
    DIM r AS STRING, p AS STRING

    IF StdLib_IsGeneratedFile%(f$) THEN IsCompilerFile% = -1: EXIT FUNCTION
    r = Path_Normalize$(QBNexRoot$ + "compiler" + Path_Sep$)
    p = Path_Normalize$(f$)
    IF os$ = "WIN" THEN p = UCASE$(p): r = UCASE$(r)
    IF LEFT$(p, LEN(r)) = r THEN IsCompilerFile% = -1
END FUNCTION

'A path in the QBNex folder written relative to it, for the generated $INCLUDE lines: the
'folder's own name may have a ' in it (O'Neil), which would end '$INCLUDE:'...' early.
FUNCTION StdLib_RootRelative$ (path$)
    DIM r AS STRING, p AS STRING

    StdLib_RootRelative$ = path$
    r = QBNexRoot$
    p = path$
    IF os$ = "WIN" THEN r = UCASE$(r): p = UCASE$(p)
    IF LEFT$(p, LEN(r)) = r THEN StdLib_RootRelative$ = MID$(path$, LEN(r) + 1)
END FUNCTION

'Files with $INCLUDEONCE (QB64-PE) are included the first time only; the key is a file's
'full path (letter case ignored on Windows), so another relative name of the same file
'is the same file.
FUNCTION IncludeOnce_Key$ (f$)
    DIM d AS STRING, k AS STRING, n AS LONG

    n = Path_LastSep&(f$)
    d = LEFT$(f$, n)
    k = ResolveAbsoluteDirectory$(d)
    IF LEN(k) = 0 THEN k = d
    k = Path_Normalize$(k + MID$(f$, n + 1))
    IF os$ = "WIN" THEN k = UCASE$(k)
    IncludeOnce_Key$ = CHR$(0) + k + CHR$(0)
END FUNCTION

SUB IncludeOnce_Mark (f$)
    IF IncludeOnce_Seen%(f$) THEN EXIT SUB
    includeOnceFiles$ = includeOnceFiles$ + IncludeOnce_Key$(f$)
END SUB

FUNCTION IncludeOnce_Seen% (f$)
    IF LEN(includeOnceFiles$) = 0 THEN EXIT FUNCTION
    IF INSTR(includeOnceFiles$, IncludeOnce_Key$(f$)) THEN IncludeOnce_Seen% = -1
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
    'relative to the QBNex folder (the include managers look there for the compiler's files)
    directive = "'$INCLUDE:'" + StdLib_RootRelative$(path$) + "|" + LTRIM$(STR$(fromLine)) + "|" + LTRIM$(STR$(toLine)) + "'" + CHR$(13) + CHR$(10)
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
    DIM fromFolder AS LONG

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
            'only an include relative to this file is split here; any other one stays in place
            incPath = StdLib_FindInclude$(getfilepath$(path$), incName, fromFolder)
            IF fromFolder = 0 THEN incPath = ""
        END IF
        IF LEN(incPath) THEN
            StdLib_EmitRange path$, segStart, lineNo - 1, inBody, headText$, bodyText$
            StdLib_HoistFile incPath, headText$, bodyText$, depth + 1
            segStart = lineNo + 1
        ELSEIF inBody = 0 THEN
            upperText = UCASE$(LTRIM$(lineText))
            'SUB/FUNCTION prototypes inside DECLARE [DYNAMIC|CUSTOMTYPE|STATIC] LIBRARY
            '... END DECLARE are declarations: they stay in the head.
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
    'same de-duplication as StdLib_QueueImport$: each module once ('qbnex' is the list of
    'its modules, each imported by its own key, so it combines with them in any order)
    IF INSTR(stdlibHoisted$, "@" + moduleKey + "@") THEN EXIT SUB
    modulePath = StdLib_ImportPath$(moduleKey)
    IF _FILEEXISTS(modulePath) = 0 THEN EXIT SUB 'left to the in-place $IMPORT, which reports it
    stdlibHoisted$ = stdlibHoisted$ + moduleKey + "@"
    stdlibImportOrigins$ = stdlibImportOrigins$ + CHR$(0) + moduleKey + CHR$(1) + stdlibOriginNow$
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
    DIM fromFolder AS LONG
    DIM lineNo AS LONG

    stdlibHoisted$ = "@"
    stdlibImportOrigins$ = ""
    'each pass (and a recompile) starts at the first line here: $INCLUDEONCE files are read again
    includeOnceFiles$ = ""
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
        lineNo = lineNo + 1
        stdlibOriginNow$ = "line " + LTRIM$(STR$(lineNo)) + " of " + sourcefile$
        moduleName = StdLib_MetaArgFromLine$(lineText, "$IMPORT")
        IF LEN(moduleName) THEN StdLib_HoistModule moduleName, headText, bodyText
        IF RIGHT$(lineText, 1) = CHR$(13) THEN lineText = LEFT$(lineText, LEN(lineText) - 1)
        IF SyntaxExt_LineHasTry%(lineText) THEN usesTry = -1
        'TRY inside an $INCLUDE file of the main source needs the handler too
        moduleName = StdLib_MetaArgFromLine$(lineText, "$INCLUDE")
        IF LEN(moduleName) THEN
            moduleName = StdLib_FindInclude$(getfilepath$(sourcefile$), moduleName, fromFolder)
            IF StdLib_FileUsesTry%(moduleName, 0) THEN usesTry = -1
            'and its $IMPORT lines are hoisted like the main source's
            StdLib_HoistIncludeImports moduleName, headText, bodyText, 0
        END IF
    LOOP
    IF syntaxExtUsesTry THEN usesTry = -1 'a hoisted module uses TRY
    syntaxExtUsesTry = usesTry 'TRY somewhere in the program: SyntaxExt_ProcessLine$ tracks the loops
    IF stdlibHoisted$ = "@" AND usesTry = 0 AND LEN(defFnBodyText$) = 0 THEN EXIT SUB

    'FUNCTIONs made from single-line DEF FN (deffn.bas) go after the program's last line
    bodyText = bodyText + defFnBodyText$

    'the in-place $IMPORT lines become no-ops
    IF stdlibHoisted$ <> "@" THEN importedModules$ = stdlibHoisted$

    'TRY ... END TRY (syntax_ext.bas) needs one module-level error handler
    IF usesTry THEN
        headText = "DIM SHARED QBNEX_TRYERR AS LONG" + CHR$(13) + CHR$(10) + "GOTO QBNEX_TRY_SKIP" + CHR$(13) + CHR$(10) + "QBNEX_TRY_HANDLER:" + CHR$(13) + CHR$(10) + "QBNEX_TRYERR = ERR" + CHR$(13) + CHR$(10) + "RESUME NEXT" + CHR$(13) + CHR$(10) + "QBNEX_TRY_SKIP:" + CHR$(13) + CHR$(10) + headText
    END IF

    'these files take the place of the vwatch includes at the first/last line (relative to
    'the QBNex folder, see StdLib_RootRelative$)
    vwatchDir = "compiler" + pathsep$ + "support" + pathsep$ + "vwatch" + pathsep$
    IF vWatchOn THEN
        headText = "'$INCLUDE:'" + vwatchDir + "vwatch.bi'" + CHR$(13) + CHR$(10) + headText
        bodyText = bodyText + "'$INCLUDE:'" + vwatchDir + "vwatch.bm'" + CHR$(13) + CHR$(10)
    ELSE
        bodyText = bodyText + "'$INCLUDE:'" + vwatchDir + "vwatch_stub.bm'" + CHR$(13) + CHR$(10)
    END IF

    'no head file when there is nothing to put first (a program with only DEF FN):
    'including an empty file reads past its end
    'full paths: diagnostics name them, and the include managers open them as written
    IF LEN(headText) THEN
        stdlibHeadFile$ = QBNexRoot$ + MID$(tmpdir$, 3) + "imports_head.bas"
        fh = FREEFILE
        OPEN stdlibHeadFile$ FOR OUTPUT AS #fh
        PRINT #fh, headText;
        CLOSE #fh
    END IF
    stdlibBodyFile$ = QBNexRoot$ + MID$(tmpdir$, 3) + "imports_body.bas"
    fh = FREEFILE
    OPEN stdlibBodyFile$ FOR OUTPUT AS #fh
    PRINT #fh, bodyText;
    CLOSE #fh
END SUB
SUB StdLib_HoistIncludeImports (path$, headText$, bodyText$, depth AS LONG)
    'hoists the $IMPORT lines of an $INCLUDE file of the main source and of the files
    'it includes in turn. In place, a module would come after the SUBs of the code before it,
    'where its CONST and DIM SHARED cannot be (an include of stdlib/qbnex.bas imports them all).
    DIM text AS STRING
    DIM lineText AS STRING
    DIM argText AS STRING
    DIM p AS LONG
    DIM nl AS LONG
    DIM fromFolder AS LONG
    DIM lineNo AS LONG

    IF depth > 20 OR LEN(path$) = 0 THEN EXIT SUB
    IF _FILEEXISTS(path$) = 0 THEN EXIT SUB
    text = StdLib_ReadText$(path$)
    IF INSTR(UCASE$(text), "$IMPORT") = 0 AND INSTR(UCASE$(text), "$INCLUDE") = 0 THEN EXIT SUB
    p = 1
    DO WHILE p <= LEN(text)
        nl = INSTR(p, text, CHR$(10))
        IF nl = 0 THEN nl = LEN(text) + 1
        lineText = MID$(text, p, nl - p)
        p = nl + 1
        lineNo = lineNo + 1
        stdlibOriginNow$ = "line " + LTRIM$(STR$(lineNo)) + " of " + path$
        argText = StdLib_MetaArgFromLine$(lineText, "$IMPORT")
        IF LEN(argText) THEN StdLib_HoistModule argText, headText$, bodyText$
        argText = StdLib_MetaArgFromLine$(lineText, "$INCLUDE")
        IF LEN(argText) THEN StdLib_HoistIncludeImports StdLib_FindInclude$(getfilepath$(path$), argText, fromFolder), headText$, bodyText$, depth + 1
    LOOP
END SUB

FUNCTION StdLib_FileUsesTry% (path$, depth AS LONG)
    'a TRY statement in this file or in one of its relative $INCLUDE files
    DIM text AS STRING
    DIM lineText AS STRING
    DIM incName AS STRING
    DIM p AS LONG
    DIM nl AS LONG
    DIM fromFolder AS LONG

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
            IF StdLib_FileUsesTry%(StdLib_FindInclude$(getfilepath$(path$), incName, fromFolder), depth + 1) THEN StdLib_FileUsesTry% = -1: EXIT FUNCTION
        END IF
    LOOP
END FUNCTION
'The generated import files: target\qb-temp\imports_head.bas and imports_body.bas.
FUNCTION StdLib_IsGeneratedFile% (f$)
    DIM a AS STRING, b AS STRING, p AS STRING

    'asked for each line read from an $INCLUDE file: most paths differ in length
    IF LEN(f$) <> LEN(stdlibHeadFile$) AND LEN(f$) <> LEN(stdlibBodyFile$) THEN EXIT FUNCTION
    p = f$: a = stdlibHeadFile$: b = stdlibBodyFile$
    IF os$ = "WIN" THEN p = UCASE$(p): a = UCASE$(a): b = UCASE$(b)
    IF (LEN(a) > 0 AND p = a) OR (LEN(b) > 0 AND p = b) THEN StdLib_IsGeneratedFile% = -1
END FUNCTION

'Where the stdlib module in path$ was imported ("line 3 of D:\x\main.bas"), and its name
'in modKey$; "" when path$ is no hoisted module.
FUNCTION StdLib_ImportOriginOf$ (path$, modKey$)
    DIM rel AS STRING, r AS STRING, p AS LONG, q AS LONG

    modKey$ = ""
    r = Path_Normalize$(QBNexRoot$ + "compiler" + pathsep$ + "stdlib" + pathsep$)
    rel = Path_Normalize$(path$)
    IF os$ = "WIN" THEN
        IF UCASE$(LEFT$(rel, LEN(r))) <> UCASE$(r) THEN EXIT FUNCTION
    ELSE
        IF LEFT$(rel, LEN(r)) <> r THEN EXIT FUNCTION
    END IF
    modKey$ = StdLib_CanonicalImportKey$(MID$(rel, LEN(r) + 1))
    p = INSTR(stdlibImportOrigins$, CHR$(0) + modKey$ + CHR$(1))
    IF p = 0 THEN modKey$ = "": EXIT FUNCTION
    p = p + LEN(modKey$) + 2
    q = INSTR(p, stdlibImportOrigins$, CHR$(0))
    IF q = 0 THEN q = LEN(stdlibImportOrigins$) + 1
    StdLib_ImportOriginOf$ = MID$(stdlibImportOrigins$, p, q - p)
END FUNCTION

'How an include chain names incname$(x): a generated import file by the $IMPORT in the
'program that brought the next file in, else as QBNex's generated file, with its full path.
FUNCTION IncludeChainName$ (x AS LONG)
    DIM o AS STRING, modKey AS STRING

    IncludeChainName$ = incname$(x)
    IF StdLib_IsGeneratedFile%(incname$(x)) = 0 THEN EXIT FUNCTION
    IF x < inclevel THEN o = StdLib_ImportOriginOf$(incname$(x + 1), modKey)
    IF LEN(o) THEN
        'no quotes: the first quoted name in a message is the one it is about
        IncludeChainName$ = "the $IMPORT of module " + modKey + " in " + o
    ELSE
        IncludeChainName$ = "QBNex's generated file " + incname$(x)
    END IF
END FUNCTION