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
        StdLib_ImportPath$ = getfilepath$(COMMAND$(0)) + "source" + pathsep$ + "stdlib" + pathsep$ + "stdlib.bas"
        EXIT FUNCTION
    END IF

    FOR i = 1 TO LEN(normalizedKey)
        importChar = MID$(normalizedKey, i, 1)
        IF importChar = "." THEN importChar = pathsep$
        relativeModulePath = relativeModulePath + importChar
    NEXT

    StdLib_ImportPath$ = getfilepath$(COMMAND$(0)) + "source" + pathsep$ + "stdlib" + pathsep$ + relativeModulePath + ".bas"
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
