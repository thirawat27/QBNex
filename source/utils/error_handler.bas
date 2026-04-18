'===============================================================================
' QBNex Error Handler Module
'===============================================================================
' Structured compiler diagnostics that explain:
' - what happened
' - where it happened
' - why the compiler stopped
' - how to fix it
'===============================================================================

'-------------------------------------------------------------------------------
' ERROR SEVERITY LEVELS
'-------------------------------------------------------------------------------

CONST ERR_INFO = 1
CONST ERR_WARNING = 2
CONST ERR_ERROR = 3
CONST ERR_FATAL = 4
CONST MAX_ERROR_CONTEXTS = 16

'-------------------------------------------------------------------------------
' ERROR CODES
'-------------------------------------------------------------------------------

' General errors (1000-1099)
CONST ERR_NO_SOURCE_FILE = 1001
CONST ERR_FILE_NOT_FOUND = 1002
CONST ERR_PERMISSION_DENIED = 1003
CONST ERR_OUT_OF_MEMORY = 1004
CONST ERR_INVALID_SYNTAX = 1005
CONST ERR_ENCODING_ISSUE = 1006
CONST ERR_INVALID_UTF8 = 1007

' Parser errors (1100-1199)
CONST ERR_UNEXPECTED_TOKEN = 1101
CONST ERR_UNCLOSED_STRING = 1102
CONST ERR_UNCLOSED_COMMENT = 1103
CONST ERR_INVALID_IDENTIFIER = 1104
CONST ERR_EXPECTED_EOL = 1105
CONST ERR_EXPECTED_THEN = 1106
CONST ERR_EXPECTED_TO = 1107
CONST ERR_EXPECTED_NEXT = 1108
CONST ERR_EXPECTED_LOOP = 1109
CONST ERR_EXPECTED_WEND = 1110

' Semantic errors (1200-1299)
CONST ERR_UNDEFINED_SYMBOL = 1201
CONST ERR_REDEFINED_SYMBOL = 1202
CONST ERR_TYPE_MISMATCH = 1203
CONST ERR_WRONG_ARGUMENT_COUNT = 1204
CONST ERR_INVALID_SCOPE = 1205
CONST ERR_UNDECLARED_VARIABLE = 1206
CONST ERR_OPTION_EXPLICIT_VIOLATION = 1207
CONST ERR_INVALID_ARRAY_BOUNDS = 1208
CONST ERR_SUBSCRIPT_OUT_OF_RANGE = 1209

' Code generation errors (1300-1399)
CONST ERR_CODEGEN_FAILED = 1301
CONST ERR_UNSUPPORTED_FEATURE = 1302
CONST ERR_LINK_ERROR = 1303

'-------------------------------------------------------------------------------
' ERROR INFORMATION TYPE
'-------------------------------------------------------------------------------

TYPE ErrorInfo
    errorCode AS INTEGER
    severity AS INTEGER
    message AS STRING * 512
    fileName AS STRING * 256
    lineNumber AS LONG
    columnNumber AS INTEGER
    context AS STRING * 256
    secondaryContext AS STRING * 256
    locationNote AS STRING * 256
    suggestion AS STRING * 512
    cause AS STRING * 256
    fixExample AS STRING * 256
    phaseName AS STRING * 64
    contextTrace AS STRING * 512
    fingerprint AS STRING * 512
    duplicateCount AS LONG
    recovered AS _BYTE
END TYPE

'-------------------------------------------------------------------------------
' ERROR STATISTICS
'-------------------------------------------------------------------------------

TYPE ErrorStats
    infoCount AS LONG
    warningCount AS LONG
    errorCount AS LONG
    fatalCount AS LONG
    totalCount AS LONG
    suppressedDuplicateCount AS LONG
    maxErrors AS LONG
    hasErrors AS _BYTE
END TYPE

'-------------------------------------------------------------------------------
' ERROR STATE
'-------------------------------------------------------------------------------

DIM SHARED Errors(1 TO 1000) AS ErrorInfo
DIM SHARED ErrorCount AS LONG
DIM SHARED Stats AS ErrorStats
DIM SHARED CurrentFile AS STRING
DIM SHARED ErrorOutputFile AS INTEGER
DIM SHARED VerboseMode AS _BYTE
DIM SHARED WarningsAsErrors AS _BYTE
DIM SHARED CurrentErrorPhase AS STRING * 64
DIM SHARED ErrorContextDepth AS INTEGER
DIM SHARED ErrorContextStack(1 TO MAX_ERROR_CONTEXTS) AS STRING * 128

'-------------------------------------------------------------------------------
' INITIALIZATION
'-------------------------------------------------------------------------------

SUB InitErrorHandler
    DIM i AS INTEGER

    ErrorCount = 0
    CurrentFile = ""
    ErrorOutputFile = 0
    VerboseMode = -1
    WarningsAsErrors = 0
    CurrentErrorPhase = ""
    ErrorContextDepth = 0

    FOR i = 1 TO MAX_ERROR_CONTEXTS
        ErrorContextStack(i) = ""
    NEXT

    Stats.infoCount = 0
    Stats.warningCount = 0
    Stats.errorCount = 0
    Stats.fatalCount = 0
    Stats.totalCount = 0
    Stats.suppressedDuplicateCount = 0
    Stats.maxErrors = 100
    Stats.hasErrors = 0
END SUB

SUB SetErrorOutputFile (fileNum AS INTEGER)
    ErrorOutputFile = fileNum
END SUB

SUB SetVerboseMode (enabled AS _BYTE)
    VerboseMode = enabled
END SUB

SUB SetWarningsAsErrors (enabled AS _BYTE)
    WarningsAsErrors = enabled
END SUB

SUB SetMaxErrors (maxCount AS LONG)
    Stats.maxErrors = maxCount
END SUB

SUB SetCurrentFile (fileName AS STRING)
    CurrentFile = RTRIM$(fileName)
END SUB

SUB SetErrorPhase (phaseName AS STRING)
    CurrentErrorPhase = RTRIM$(phaseName)
END SUB

SUB ClearErrorPhase
    CurrentErrorPhase = ""
END SUB

FUNCTION GetErrorPhase$ ()
    GetErrorPhase$ = RTRIM$(CurrentErrorPhase)
END FUNCTION

SUB PushErrorContext (contextText AS STRING)
    contextText = NormalizeDiagnosticMessage$(contextText)
    IF contextText = "" THEN EXIT SUB

    IF ErrorContextDepth < MAX_ERROR_CONTEXTS THEN
        ErrorContextDepth = ErrorContextDepth + 1
        ErrorContextStack(ErrorContextDepth) = contextText
    ELSE
        ErrorContextStack(MAX_ERROR_CONTEXTS) = contextText
    END IF
END SUB

SUB PopErrorContext
    IF ErrorContextDepth <= 0 THEN EXIT SUB
    ErrorContextStack(ErrorContextDepth) = ""
    ErrorContextDepth = ErrorContextDepth - 1
END SUB

SUB ClearErrorContextStack
    DIM i AS INTEGER

    FOR i = 1 TO MAX_ERROR_CONTEXTS
        ErrorContextStack(i) = ""
    NEXT

    ErrorContextDepth = 0
END SUB

'-------------------------------------------------------------------------------
' DIAGNOSTIC HELPERS
'-------------------------------------------------------------------------------

FUNCTION GetDefaultSeverity% (errCode AS INTEGER)
    SELECT CASE errCode
    CASE 1000 TO 1099: GetDefaultSeverity% = ERR_ERROR
    CASE 1100 TO 1199: GetDefaultSeverity% = ERR_ERROR
    CASE 1200 TO 1299: GetDefaultSeverity% = ERR_ERROR
    CASE 1300 TO 1399: GetDefaultSeverity% = ERR_FATAL
    CASE ELSE: GetDefaultSeverity% = ERR_ERROR
    END SELECT
END FUNCTION

FUNCTION NormalizeDiagnosticMessage$ (text AS STRING)
    DIM i AS LONG
    DIM result AS STRING
    DIM ch AS STRING
    DIM lastWasSpace AS _BYTE

    result = ""
    lastWasSpace = 0

    FOR i = 1 TO LEN(text)
        ch = MID$(text, i, 1)
        IF ASC(ch) < 32 THEN ch = " "

        IF ch = " " THEN
            IF lastWasSpace = 0 THEN result = result + " "
            lastWasSpace = -1
        ELSE
            result = result + ch
            lastWasSpace = 0
        END IF
    NEXT

    NormalizeDiagnosticMessage$ = LTRIM$(RTRIM$(result))
END FUNCTION

FUNCTION NormalizeSourceContext$ (text AS STRING)
    DIM i AS LONG
    DIM result AS STRING
    DIM ch AS STRING
    DIM code AS INTEGER

    result = ""

    FOR i = 1 TO LEN(text)
        ch = MID$(text, i, 1)
        code = ASC(ch)

        SELECT CASE code
        CASE 9
            result = result + "    "
        CASE 10, 13
            IF LEN(result) > 0 AND RIGHT$(result, 1) <> " " THEN result = result + " "
        CASE 0 TO 31
            result = result + " "
        CASE ELSE
            result = result + ch
        END SELECT
    NEXT

    NormalizeSourceContext$ = RTRIM$(result)
END FUNCTION

FUNCTION MergeDiagnosticNote$ (existingText AS STRING, newText AS STRING)
    DIM normalizedExisting AS STRING
    DIM normalizedNew AS STRING

    normalizedExisting = NormalizeDiagnosticMessage$(existingText)
    normalizedNew = NormalizeDiagnosticMessage$(newText)

    IF normalizedNew = "" THEN
        MergeDiagnosticNote$ = normalizedExisting
    ELSEIF normalizedExisting = "" THEN
        MergeDiagnosticNote$ = normalizedNew
    ELSEIF UCASE$(normalizedExisting) = UCASE$(normalizedNew) THEN
        MergeDiagnosticNote$ = normalizedExisting
    ELSEIF INSTR(UCASE$(normalizedExisting), UCASE$(normalizedNew)) > 0 THEN
        MergeDiagnosticNote$ = normalizedExisting
    ELSE
        MergeDiagnosticNote$ = normalizedExisting + " | " + normalizedNew
    END IF
END FUNCTION

FUNCTION GetErrorContextTrace$ ()
    DIM i AS INTEGER
    DIM trace AS STRING
    DIM itemText AS STRING

    trace = ""

    FOR i = 1 TO ErrorContextDepth
        itemText = NormalizeDiagnosticMessage$(ErrorContextStack(i))
        IF itemText <> "" THEN
            IF trace = "" THEN
                trace = itemText
            ELSE
                trace = trace + " -> " + itemText
            END IF
        END IF
    NEXT

    GetErrorContextTrace$ = trace
END FUNCTION

FUNCTION BuildErrorFingerprint$ (errCode AS INTEGER, severity AS INTEGER, fileName AS STRING, lineNum AS LONG, message AS STRING, context AS STRING, phaseName AS STRING)
    BuildErrorFingerprint$ = UCASE$(RTRIM$(fileName)) + "|" + LTRIM$(STR$(lineNum)) + "|" + LTRIM$(STR$(severity)) + "|" + LTRIM$(STR$(errCode)) + "|" + UCASE$(NormalizeDiagnosticMessage$(message)) + "|" + UCASE$(NormalizeSourceContext$(context)) + "|" + UCASE$(NormalizeDiagnosticMessage$(phaseName))
END FUNCTION

FUNCTION FindExistingErrorIndex% (fingerprint AS STRING)
    DIM i AS LONG

    fingerprint = LEFT$(RTRIM$(fingerprint), 512)
    IF fingerprint = "" THEN
        FindExistingErrorIndex% = 0
        EXIT FUNCTION
    END IF

    FOR i = 1 TO ErrorCount
        IF RTRIM$(Errors(i).fingerprint) = fingerprint THEN
            FindExistingErrorIndex% = i
            EXIT FUNCTION
        END IF
    NEXT

    FindExistingErrorIndex% = 0
END FUNCTION

FUNCTION GetBlockingIssueCount& ()
    GetBlockingIssueCount& = Stats.errorCount + Stats.fatalCount
    IF WarningsAsErrors THEN GetBlockingIssueCount& = GetBlockingIssueCount& + Stats.warningCount
END FUNCTION

FUNCTION RepeatSpaces$ (count AS INTEGER)
    IF count <= 0 THEN
        RepeatSpaces$ = ""
    ELSE
        RepeatSpaces$ = STRING$(count, " ")
    END IF
END FUNCTION

FUNCTION ExtractDiagnosticToken$ (message AS STRING)
    DIM pos1 AS INTEGER
    DIM pos2 AS INTEGER
    DIM token AS STRING

    token = ""

    pos1 = INSTR(message, "'")
    IF pos1 > 0 THEN
        pos2 = INSTR(pos1 + 1, message, "'")
        IF pos2 > pos1 THEN token = MID$(message, pos1 + 1, pos2 - pos1 - 1)
    END IF

    IF token = "" THEN
        pos1 = INSTR(message, "(")
        IF pos1 > 0 THEN
            pos2 = INSTR(pos1 + 1, message, ")")
            IF pos2 > pos1 AND pos2 - pos1 < 64 THEN token = MID$(message, pos1 + 1, pos2 - pos1 - 1)
        END IF
    END IF

    ExtractDiagnosticToken$ = NormalizeDiagnosticMessage$(token)
END FUNCTION

FUNCTION IsIdentifierStartChar% (ch AS STRING)
    IF LEN(ch) = 0 THEN
        IsIdentifierStartChar% = 0
        EXIT FUNCTION
    END IF

    IF (ASC(ch) >= 65 AND ASC(ch) <= 90) OR (ASC(ch) >= 97 AND ASC(ch) <= 122) OR ch = "_" THEN
        IsIdentifierStartChar% = -1
    ELSE
        IsIdentifierStartChar% = 0
    END IF
END FUNCTION

FUNCTION IsIdentifierBodyChar% (ch AS STRING)
    IF LEN(ch) = 0 THEN
        IsIdentifierBodyChar% = 0
        EXIT FUNCTION
    END IF

    IF IsIdentifierStartChar%(ch) THEN
        IsIdentifierBodyChar% = -1
    ELSEIF ASC(ch) >= 48 AND ASC(ch) <= 57 THEN
        IsIdentifierBodyChar% = -1
    ELSEIF ch = "$" OR ch = "%" OR ch = "&" OR ch = "!" OR ch = "#" OR ch = "~" THEN
        IsIdentifierBodyChar% = -1
    ELSE
        IsIdentifierBodyChar% = 0
    END IF
END FUNCTION

FUNCTION ExtractLeadingIdentifier$ (context AS STRING)
    DIM normalized AS STRING
    DIM i AS LONG
    DIM startPos AS LONG

    normalized = NormalizeSourceContext$(context)
    startPos = 1

    DO WHILE startPos <= LEN(normalized) AND MID$(normalized, startPos, 1) = " "
        startPos = startPos + 1
    LOOP

    IF startPos > LEN(normalized) THEN
        ExtractLeadingIdentifier$ = ""
        EXIT FUNCTION
    END IF

    IF IsIdentifierStartChar%(MID$(normalized, startPos, 1)) = 0 THEN
        ExtractLeadingIdentifier$ = ""
        EXIT FUNCTION
    END IF

    i = startPos
    DO WHILE i <= LEN(normalized)
        IF IsIdentifierBodyChar%(MID$(normalized, i, 1)) = 0 THEN EXIT DO
        i = i + 1
    LOOP

    ExtractLeadingIdentifier$ = MID$(normalized, startPos, i - startPos)
END FUNCTION

FUNCTION NextNonSpaceCharAfterLeadingIdentifier$ (context AS STRING)
    DIM normalized AS STRING
    DIM token AS STRING
    DIM i AS LONG

    normalized = NormalizeSourceContext$(context)
    token = ExtractLeadingIdentifier$(normalized)
    IF token = "" THEN
        NextNonSpaceCharAfterLeadingIdentifier$ = ""
        EXIT FUNCTION
    END IF

    i = INSTR(normalized, token)
    IF i = 0 THEN
        NextNonSpaceCharAfterLeadingIdentifier$ = ""
        EXIT FUNCTION
    END IF

    i = i + LEN(token)
    DO WHILE i <= LEN(normalized) AND MID$(normalized, i, 1) = " "
        i = i + 1
    LOOP

    IF i > LEN(normalized) THEN
        NextNonSpaceCharAfterLeadingIdentifier$ = ""
    ELSE
        NextNonSpaceCharAfterLeadingIdentifier$ = MID$(normalized, i, 1)
    END IF
END FUNCTION

FUNCTION CountChar% (text AS STRING, target AS STRING)
    DIM i AS LONG
    DIM total AS INTEGER

    total = 0
    FOR i = 1 TO LEN(text)
        IF MID$(text, i, 1) = target THEN total = total + 1
    NEXT

    CountChar% = total
END FUNCTION

FUNCTION IsAdjacentSwapMatch% (source AS STRING, target AS STRING)
    DIM i AS LONG
    DIM mismatchPos AS LONG

    source = UCASE$(source)
    target = UCASE$(target)

    IF LEN(source) <> LEN(target) OR LEN(source) < 2 THEN
        IsAdjacentSwapMatch% = 0
        EXIT FUNCTION
    END IF

    mismatchPos = 0
    FOR i = 1 TO LEN(source)
        IF MID$(source, i, 1) <> MID$(target, i, 1) THEN
            mismatchPos = i
            EXIT FOR
        END IF
    NEXT

    IF mismatchPos = 0 OR mismatchPos >= LEN(source) THEN
        IsAdjacentSwapMatch% = 0
        EXIT FUNCTION
    END IF

    IF MID$(source, mismatchPos, 1) <> MID$(target, mismatchPos + 1, 1) THEN
        IsAdjacentSwapMatch% = 0
        EXIT FUNCTION
    END IF

    IF MID$(source, mismatchPos + 1, 1) <> MID$(target, mismatchPos, 1) THEN
        IsAdjacentSwapMatch% = 0
        EXIT FUNCTION
    END IF

    IF LEFT$(source, mismatchPos - 1) + MID$(source, mismatchPos + 2) = LEFT$(target, mismatchPos - 1) + MID$(target, mismatchPos + 2) THEN
        IsAdjacentSwapMatch% = -1
    ELSE
        IsAdjacentSwapMatch% = 0
    END IF
END FUNCTION

FUNCTION IsSingleEditMatch% (source AS STRING, target AS STRING)
    DIM i AS LONG
    DIM j AS LONG
    DIM edits AS INTEGER
    DIM lenSource AS INTEGER
    DIM lenTarget AS INTEGER

    source = UCASE$(source)
    target = UCASE$(target)
    lenSource = LEN(source)
    lenTarget = LEN(target)

    IF ABS(lenSource - lenTarget) > 1 THEN
        IsSingleEditMatch% = 0
        EXIT FUNCTION
    END IF

    i = 1
    j = 1
    edits = 0

    DO WHILE i <= lenSource AND j <= lenTarget
        IF MID$(source, i, 1) = MID$(target, j, 1) THEN
            i = i + 1
            j = j + 1
        ELSE
            edits = edits + 1
            IF edits > 1 THEN
                IsSingleEditMatch% = 0
                EXIT FUNCTION
            END IF

            IF lenSource > lenTarget THEN
                i = i + 1
            ELSEIF lenTarget > lenSource THEN
                j = j + 1
            ELSE
                i = i + 1
                j = j + 1
            END IF
        END IF
    LOOP

    IF i <= lenSource OR j <= lenTarget THEN edits = edits + 1

    IF edits = 1 THEN
        IsSingleEditMatch% = -1
    ELSE
        IsSingleEditMatch% = 0
    END IF
END FUNCTION

FUNCTION SuggestStatementKeyword$ (token AS STRING)
    DIM candidates AS STRING
    DIM startPos AS LONG
    DIM endPos AS LONG
    DIM candidate AS STRING

    token = UCASE$(NormalizeDiagnosticMessage$(token))
    IF LEN(token) < 3 THEN
        SuggestStatementKeyword$ = ""
        EXIT FUNCTION
    END IF

    candidates = "PRINT|INPUT|IF|THEN|ELSE|ELSEIF|FOR|NEXT|DO|LOOP|WHILE|WEND|SELECT|CASE|DIM|REDIM|CONST|DECLARE|SUB|FUNCTION|END|TYPE|CLASS|METHOD|CALL|LET|OPEN|CLOSE|READ|DATA|RESTORE|RETURN|GOTO|GOSUB|ON|OPTION|EXIT|SYSTEM|SLEEP|RANDOMIZE|COLOR|LOCATE|LINE|PSET|CIRCLE|PAINT|PLAY|SOUND|BEEP|CLS|WRITE|LPRINT"
    startPos = 1

    DO WHILE startPos <= LEN(candidates)
        endPos = INSTR(startPos, candidates, "|")
        IF endPos = 0 THEN endPos = LEN(candidates) + 1
        candidate = MID$(candidates, startPos, endPos - startPos)

        IF token <> candidate THEN
            IF IsSingleEditMatch%(token, candidate) OR IsAdjacentSwapMatch%(token, candidate) THEN
                SuggestStatementKeyword$ = candidate
                EXIT FUNCTION
            END IF
        END IF

        startPos = endPos + 1
    LOOP

    SuggestStatementKeyword$ = ""
END FUNCTION

FUNCTION GetStatementTypoToken$ (context AS STRING, suggestion AS STRING)
    DIM token AS STRING
    DIM nextChar AS STRING

    suggestion = ""
    token = UCASE$(ExtractLeadingIdentifier$(context))
    IF token = "" THEN
        GetStatementTypoToken$ = ""
        EXIT FUNCTION
    END IF

    nextChar = NextNonSpaceCharAfterLeadingIdentifier$(context)
    IF nextChar = "=" OR nextChar = ":" OR LEFT$(token, 2) = "__" THEN
        GetStatementTypoToken$ = ""
        EXIT FUNCTION
    END IF

    suggestion = SuggestStatementKeyword$(token)
    IF suggestion = "" THEN
        GetStatementTypoToken$ = ""
    ELSE
        GetStatementTypoToken$ = token
    END IF
END FUNCTION

FUNCTION InferErrorCode% (requestedCode AS INTEGER, message AS STRING, context AS STRING)
    DIM upperMessage AS STRING
    DIM upperContext AS STRING
    DIM typoToken AS STRING
    DIM suggestion AS STRING

    InferErrorCode% = requestedCode
    upperMessage = UCASE$(NormalizeDiagnosticMessage$(message))
    upperContext = UCASE$(NormalizeSourceContext$(context))

    IF requestedCode <> ERR_INVALID_SYNTAX AND requestedCode <> ERR_UNEXPECTED_TOKEN THEN EXIT FUNCTION

    IF INSTR(upperMessage, "TYPE MISMATCH") > 0 THEN
        InferErrorCode% = ERR_TYPE_MISMATCH
    ELSEIF INSTR(upperMessage, "NAME ALREADY IN USE") > 0 OR INSTR(upperMessage, "ALREADY DEFINED") > 0 THEN
        InferErrorCode% = ERR_REDEFINED_SYMBOL
    ELSEIF INSTR(upperMessage, "NOT DEFINED") > 0 OR INSTR(upperMessage, "UNRESOLVED SYMBOL") > 0 OR INSTR(upperMessage, "UNDEFINED SYMBOL") > 0 THEN
        InferErrorCode% = ERR_UNDEFINED_SYMBOL
    ELSEIF INSTR(upperMessage, "EXPECTED THEN") > 0 OR INSTR(upperMessage, "THEN/GOTO") > 0 THEN
        InferErrorCode% = ERR_EXPECTED_THEN
    ELSEIF INSTR(upperMessage, "EXPECTED TO") > 0 OR INSTR(upperMessage, "FOR NAME = START TO END") > 0 THEN
        InferErrorCode% = ERR_EXPECTED_TO
    ELSEIF INSTR(upperMessage, "EXPECTED NEXT") > 0 THEN
        InferErrorCode% = ERR_EXPECTED_NEXT
    ELSEIF INSTR(upperMessage, "EXPECTED LOOP") > 0 OR INSTR(upperMessage, "LOOP ERROR!") > 0 THEN
        InferErrorCode% = ERR_EXPECTED_LOOP
    ELSEIF INSTR(upperMessage, "EXPECTED WEND") > 0 THEN
        InferErrorCode% = ERR_EXPECTED_WEND
    ELSEIF INSTR(upperMessage, "EXPECTED " + CHR$(34)) > 0 OR INSTR(upperMessage, "UNCLOSED STRING") > 0 THEN
        InferErrorCode% = ERR_UNCLOSED_STRING
    ELSEIF INSTR(upperMessage, "INVALID NAME") > 0 OR INSTR(upperMessage, "INVALID IDENTIFIER") > 0 OR INSTR(upperMessage, "IDENTIFIER LONGER") > 0 THEN
        InferErrorCode% = ERR_INVALID_IDENTIFIER
    ELSEIF INSTR(upperMessage, "UNEXPECTED CHARACTER") > 0 OR INSTR(upperMessage, "INVALID SYMBOL") > 0 OR INSTR(upperMessage, "UNEXPECTED TOKEN") > 0 THEN
        InferErrorCode% = ERR_UNEXPECTED_TOKEN
    ELSEIF requestedCode = ERR_INVALID_SYNTAX THEN
        IF CountChar%(context, CHR$(34)) MOD 2 <> 0 THEN InferErrorCode% = ERR_UNCLOSED_STRING
        typoToken = GetStatementTypoToken$(context, suggestion)
        IF typoToken <> "" THEN InferErrorCode% = ERR_UNEXPECTED_TOKEN
        IF INSTR(upperContext, "IF ") > 0 AND INSTR(upperContext, "THEN") = 0 THEN InferErrorCode% = ERR_EXPECTED_THEN
        IF INSTR(upperContext, "FOR ") > 0 AND INSTR(upperContext, " TO ") = 0 AND INSTR(upperMessage, "EXPECTED") > 0 THEN InferErrorCode% = ERR_EXPECTED_TO
    END IF
END FUNCTION

FUNCTION GetDiagnosticHeadline$ (errCode AS INTEGER, message AS STRING, context AS STRING)
    DIM token AS STRING
    DIM typoToken AS STRING
    DIM suggestion AS STRING

    token = ExtractDiagnosticToken$(message)
    typoToken = GetStatementTypoToken$(context, suggestion)

    SELECT CASE errCode
    CASE ERR_EXPECTED_THEN
        GetDiagnosticHeadline$ = "IF statement is missing THEN or GOTO"
    CASE ERR_EXPECTED_TO
        GetDiagnosticHeadline$ = "FOR loop is missing TO"
    CASE ERR_EXPECTED_NEXT
        GetDiagnosticHeadline$ = "FOR block was not closed with NEXT"
    CASE ERR_EXPECTED_LOOP
        GetDiagnosticHeadline$ = "DO block was not closed with LOOP"
    CASE ERR_EXPECTED_WEND
        GetDiagnosticHeadline$ = "WHILE block was not closed with WEND"
    CASE ERR_UNCLOSED_STRING
        GetDiagnosticHeadline$ = "String literal was opened but not closed"
    CASE ERR_UNDEFINED_SYMBOL
        IF token <> "" THEN
            GetDiagnosticHeadline$ = "'" + token + "' is not defined in the current scope"
        ELSE
            GetDiagnosticHeadline$ = "A symbol is used before it is defined"
        END IF
    CASE ERR_REDEFINED_SYMBOL
        IF token <> "" THEN
            GetDiagnosticHeadline$ = "'" + token + "' is already defined"
        ELSE
            GetDiagnosticHeadline$ = "A name is declared more than once in the same scope"
        END IF
    CASE ERR_TYPE_MISMATCH
        GetDiagnosticHeadline$ = "Expression mixes incompatible types"
    CASE ERR_INVALID_IDENTIFIER
        GetDiagnosticHeadline$ = "Identifier contains invalid characters or shape"
    CASE ERR_UNEXPECTED_TOKEN
        IF typoToken <> "" THEN
            GetDiagnosticHeadline$ = "Unknown statement '" + typoToken + "'. Did you mean '" + suggestion + "'?"
        ELSE
            GetDiagnosticHeadline$ = "Unexpected token or character"
        END IF
    CASE ELSE
        IF NormalizeDiagnosticMessage$(message) <> "" THEN
            GetDiagnosticHeadline$ = NormalizeDiagnosticMessage$(message)
        ELSE
            GetDiagnosticHeadline$ = GetErrorMessage$(errCode)
        END IF
    END SELECT
END FUNCTION

FUNCTION GetPointerHint$ (errCode AS INTEGER, message AS STRING, context AS STRING)
    DIM typoToken AS STRING
    DIM suggestion AS STRING

    typoToken = GetStatementTypoToken$(context, suggestion)

    SELECT CASE errCode
    CASE ERR_EXPECTED_THEN
        GetPointerHint$ = "add THEN here"
    CASE ERR_EXPECTED_TO
        GetPointerHint$ = "add TO here"
    CASE ERR_EXPECTED_NEXT
        GetPointerHint$ = "add NEXT after the loop body"
    CASE ERR_EXPECTED_LOOP
        GetPointerHint$ = "close the DO block here"
    CASE ERR_EXPECTED_WEND
        GetPointerHint$ = "close the WHILE block here"
    CASE ERR_UNCLOSED_STRING
        GetPointerHint$ = "close the string here"
    CASE ERR_UNDEFINED_SYMBOL
        GetPointerHint$ = "unknown symbol"
    CASE ERR_REDEFINED_SYMBOL
        GetPointerHint$ = "duplicate name"
    CASE ERR_TYPE_MISMATCH
        GetPointerHint$ = "check operand types here"
    CASE ERR_UNEXPECTED_TOKEN
        IF typoToken <> "" THEN
            GetPointerHint$ = "did you mean " + suggestion + "?"
        ELSE
            GetPointerHint$ = "unexpected token"
        END IF
    CASE ELSE
        GetPointerHint$ = ""
    END SELECT
END FUNCTION

FUNCTION GetLocationLabel$ (fileName AS STRING, lineNum AS LONG, colNum AS INTEGER)
    DIM location AS STRING

    location = RTRIM$(fileName)

    IF lineNum > 0 THEN
        location = location + ":" + LTRIM$(STR$(lineNum))
        IF colNum > 0 THEN location = location + ":" + LTRIM$(STR$(colNum))
    END IF

    GetLocationLabel$ = location
END FUNCTION

'-------------------------------------------------------------------------------
' ERROR REPORTING
'-------------------------------------------------------------------------------

SUB ReportError (errCode AS INTEGER, message AS STRING, lineNum AS LONG, context AS STRING)
    ReportErrorWithSeverity errCode, GetDefaultSeverity%(errCode), message, lineNum, context
END SUB

SUB ReportDetailedError (errCode AS INTEGER, message AS STRING, lineNum AS LONG, context AS STRING, secondaryContext AS STRING, locationNote AS STRING)
    ReportDetailedErrorWithSeverity errCode, GetDefaultSeverity%(errCode), message, lineNum, context, secondaryContext, locationNote
END SUB

SUB ReportErrorWithSeverity (errCode AS INTEGER, severity AS INTEGER, message AS STRING, lineNum AS LONG, context AS STRING)
    ReportDetailedErrorWithSeverity errCode, severity, message, lineNum, context, "", ""
END SUB

SUB ReportDetailedErrorWithSeverity (errCode AS INTEGER, severity AS INTEGER, message AS STRING, lineNum AS LONG, context AS STRING, secondaryContext AS STRING, locationNote AS STRING)
    DIM errIdx AS LONG
    DIM duplicateIdx AS LONG
    DIM colNum AS INTEGER
    DIM causeStr AS STRING
    DIM fixStr AS STRING
    DIM normalizedMessage AS STRING
    DIM normalizedContext AS STRING
    DIM normalizedSecondary AS STRING
    DIM normalizedLocation AS STRING
    DIM phaseName AS STRING
    DIM contextTrace AS STRING
    DIM fingerprint AS STRING

    normalizedMessage = NormalizeDiagnosticMessage$(message)
    normalizedContext = NormalizeSourceContext$(context)
    normalizedSecondary = NormalizeSourceContext$(secondaryContext)
    normalizedLocation = NormalizeDiagnosticMessage$(locationNote)
    phaseName = GetErrorPhase$()
    contextTrace = GetErrorContextTrace$()

    errCode = InferErrorCode%(errCode, normalizedMessage, normalizedContext)
    fingerprint = BuildErrorFingerprint$(errCode, severity, CurrentFile, lineNum, normalizedMessage, normalizedContext, phaseName)

    duplicateIdx = FindExistingErrorIndex%(fingerprint)
    IF duplicateIdx > 0 THEN
        Errors(duplicateIdx).duplicateCount = Errors(duplicateIdx).duplicateCount + 1
        Stats.suppressedDuplicateCount = Stats.suppressedDuplicateCount + 1
        EXIT SUB
    END IF

    IF GetBlockingIssueCount& >= Stats.maxErrors THEN
        fingerprint = BuildErrorFingerprint$(9999, ERR_FATAL, CurrentFile, 0, "Too many errors - compilation aborted", "", phaseName)
        duplicateIdx = FindExistingErrorIndex%(fingerprint)

        IF duplicateIdx = 0 AND ErrorCount < 1000 THEN
            ErrorCount = ErrorCount + 1
            Errors(ErrorCount).errorCode = 9999
            Errors(ErrorCount).severity = ERR_FATAL
            Errors(ErrorCount).message = "Too many errors - compilation aborted"
            Errors(ErrorCount).fileName = CurrentFile
            Errors(ErrorCount).lineNumber = 0
            Errors(ErrorCount).columnNumber = 0
            Errors(ErrorCount).context = ""
            Errors(ErrorCount).secondaryContext = ""
            Errors(ErrorCount).locationNote = "Stop after reaching the configured diagnostic budget."
            Errors(ErrorCount).suggestion = "Fix the reported errors and recompile"
            Errors(ErrorCount).cause = "Maximum error limit reached"
            Errors(ErrorCount).fixExample = ""
            Errors(ErrorCount).phaseName = phaseName
            Errors(ErrorCount).contextTrace = contextTrace
            Errors(ErrorCount).fingerprint = fingerprint
            Errors(ErrorCount).duplicateCount = 1
            Errors(ErrorCount).recovered = 0

            Stats.totalCount = Stats.totalCount + 1
            Stats.fatalCount = Stats.fatalCount + 1
            Stats.hasErrors = -1
            PrintError ErrorCount
        ELSEIF duplicateIdx > 0 THEN
            Errors(duplicateIdx).duplicateCount = Errors(duplicateIdx).duplicateCount + 1
            Stats.suppressedDuplicateCount = Stats.suppressedDuplicateCount + 1
        END IF

        EXIT SUB
    END IF

    colNum = 0
    IF normalizedContext <> "" THEN colNum = FindErrorColumn%(errCode, normalizedMessage, normalizedContext)

    causeStr = GetErrorCause$(errCode, normalizedMessage, normalizedContext)
    fixStr = GetFixExample$(errCode, normalizedMessage, normalizedContext)

    IF ErrorCount < 1000 THEN
        ErrorCount = ErrorCount + 1
        errIdx = ErrorCount

        Errors(errIdx).errorCode = errCode
        Errors(errIdx).severity = severity
        Errors(errIdx).message = normalizedMessage
        Errors(errIdx).fileName = CurrentFile
        Errors(errIdx).lineNumber = lineNum
        Errors(errIdx).columnNumber = colNum
        Errors(errIdx).context = normalizedContext
        Errors(errIdx).secondaryContext = normalizedSecondary
        Errors(errIdx).locationNote = normalizedLocation
        Errors(errIdx).suggestion = GetDetailedSuggestion$(errCode, normalizedMessage, normalizedContext)
        Errors(errIdx).cause = causeStr
        Errors(errIdx).fixExample = fixStr
        Errors(errIdx).phaseName = phaseName
        Errors(errIdx).contextTrace = contextTrace
        Errors(errIdx).fingerprint = fingerprint
        Errors(errIdx).duplicateCount = 1
        Errors(errIdx).recovered = 0
    END IF

    Stats.totalCount = Stats.totalCount + 1
    SELECT CASE severity
    CASE ERR_INFO
        Stats.infoCount = Stats.infoCount + 1
    CASE ERR_WARNING
        Stats.warningCount = Stats.warningCount + 1
        IF WarningsAsErrors THEN Stats.hasErrors = -1
    CASE ERR_ERROR
        Stats.errorCount = Stats.errorCount + 1
        Stats.hasErrors = -1
    CASE ERR_FATAL
        Stats.fatalCount = Stats.fatalCount + 1
        Stats.hasErrors = -1
    END SELECT

    PrintError errIdx
END SUB

SUB ReportWarning (warnCode AS INTEGER, message AS STRING, lineNum AS LONG, context AS STRING)
    ReportErrorWithSeverity warnCode, ERR_WARNING, message, lineNum, context
END SUB

SUB ReportInfo (infoCode AS INTEGER, message AS STRING, lineNum AS LONG)
    ReportErrorWithSeverity infoCode, ERR_INFO, message, lineNum, ""
END SUB

'-------------------------------------------------------------------------------
' ERROR MESSAGE FORMATTING
'-------------------------------------------------------------------------------

FUNCTION GetErrorMessage$ (errCode AS INTEGER)
    SELECT CASE errCode
    CASE ERR_NO_SOURCE_FILE: GetErrorMessage$ = "No source file specified"
    CASE ERR_FILE_NOT_FOUND: GetErrorMessage$ = "Source file not found"
    CASE ERR_PERMISSION_DENIED: GetErrorMessage$ = "Permission denied accessing file"
    CASE ERR_OUT_OF_MEMORY: GetErrorMessage$ = "Out of memory"
    CASE ERR_INVALID_SYNTAX: GetErrorMessage$ = "Invalid syntax"
    CASE ERR_UNEXPECTED_TOKEN: GetErrorMessage$ = "Unexpected token"
    CASE ERR_UNCLOSED_STRING: GetErrorMessage$ = "Unclosed string literal"
    CASE ERR_UNCLOSED_COMMENT: GetErrorMessage$ = "Unclosed comment"
    CASE ERR_INVALID_IDENTIFIER: GetErrorMessage$ = "Invalid identifier"
    CASE ERR_EXPECTED_EOL: GetErrorMessage$ = "Expected end of line"
    CASE ERR_EXPECTED_THEN: GetErrorMessage$ = "Expected THEN"
    CASE ERR_EXPECTED_TO: GetErrorMessage$ = "Expected TO"
    CASE ERR_EXPECTED_NEXT: GetErrorMessage$ = "Expected NEXT"
    CASE ERR_EXPECTED_LOOP: GetErrorMessage$ = "Expected LOOP"
    CASE ERR_EXPECTED_WEND: GetErrorMessage$ = "Expected WEND"
    CASE ERR_UNDEFINED_SYMBOL: GetErrorMessage$ = "Undefined symbol"
    CASE ERR_REDEFINED_SYMBOL: GetErrorMessage$ = "Symbol already defined"
    CASE ERR_TYPE_MISMATCH: GetErrorMessage$ = "Type mismatch"
    CASE ERR_WRONG_ARGUMENT_COUNT: GetErrorMessage$ = "Wrong number of arguments"
    CASE ERR_INVALID_SCOPE: GetErrorMessage$ = "Invalid scope"
    CASE ERR_UNDECLARED_VARIABLE: GetErrorMessage$ = "Undeclared variable (Option Explicit)"
    CASE ERR_OPTION_EXPLICIT_VIOLATION: GetErrorMessage$ = "Option Explicit requires variable declaration"
    CASE ERR_INVALID_ARRAY_BOUNDS: GetErrorMessage$ = "Invalid array bounds"
    CASE ERR_SUBSCRIPT_OUT_OF_RANGE: GetErrorMessage$ = "Subscript out of range"
    CASE ERR_CODEGEN_FAILED: GetErrorMessage$ = "Code generation failed"
    CASE ERR_UNSUPPORTED_FEATURE: GetErrorMessage$ = "Unsupported feature"
    CASE ERR_LINK_ERROR: GetErrorMessage$ = "Link error"
    CASE ELSE: GetErrorMessage$ = "Unknown error"
    END SELECT
END FUNCTION

FUNCTION GetSuggestion$ (errCode AS INTEGER)
    SELECT CASE errCode
    CASE ERR_NO_SOURCE_FILE: GetSuggestion$ = "Specify a .bas file to compile: qb program.bas"
    CASE ERR_FILE_NOT_FOUND: GetSuggestion$ = "Check the file path exists and try again"
    CASE ERR_PERMISSION_DENIED: GetSuggestion$ = "Check file permissions or run with elevated privileges"
    CASE ERR_OUT_OF_MEMORY: GetSuggestion$ = "Close other applications or use smaller source files"
    CASE ERR_INVALID_SYNTAX: GetSuggestion$ = "Review the syntax and compare with QBasic documentation"
    CASE ERR_ENCODING_ISSUE: GetSuggestion$ = "Save the source file as UTF-8 encoded text"
    CASE ERR_INVALID_UTF8: GetSuggestion$ = "Remove invalid UTF-8 characters or re-encode the file"
    CASE ERR_UNEXPECTED_TOKEN: GetSuggestion$ = "Remove or replace the unexpected symbol"
    CASE ERR_UNCLOSED_STRING: GetSuggestion$ = "Add closing double quote to the string literal"
    CASE ERR_UNCLOSED_COMMENT: GetSuggestion$ = "Close the comment with appropriate delimiter"
    CASE ERR_INVALID_IDENTIFIER: GetSuggestion$ = "Use valid variable names (letters, numbers, underscores)"
    CASE ERR_EXPECTED_THEN: GetSuggestion$ = "Add THEN after the IF condition"
    CASE ERR_EXPECTED_TO: GetSuggestion$ = "Add TO in the FOR loop statement"
    CASE ERR_EXPECTED_NEXT: GetSuggestion$ = "Add NEXT to close the FOR loop"
    CASE ERR_EXPECTED_LOOP: GetSuggestion$ = "Add LOOP to close the DO loop"
    CASE ERR_EXPECTED_WEND: GetSuggestion$ = "Add WEND to close the WHILE loop"
    CASE ERR_UNDEFINED_SYMBOL: GetSuggestion$ = "Define the symbol or check for typos"
    CASE ERR_REDEFINED_SYMBOL: GetSuggestion$ = "Use a different name or remove duplicate definition"
    CASE ERR_TYPE_MISMATCH: GetSuggestion$ = "Convert the value to the correct type using type conversion functions"
    CASE ERR_WRONG_ARGUMENT_COUNT: GetSuggestion$ = "Check the function signature and provide correct arguments"
    CASE ERR_INVALID_SCOPE: GetSuggestion$ = "Move the statement to an appropriate scope"
    CASE ERR_UNDECLARED_VARIABLE: GetSuggestion$ = "Declare the variable with DIM or turn off Option Explicit"
    CASE ERR_OPTION_EXPLICIT_VIOLATION: GetSuggestion$ = "Add DIM statement before using the variable"
    CASE ERR_INVALID_ARRAY_BOUNDS: GetSuggestion$ = "Check array dimension bounds are positive integers"
    CASE ERR_SUBSCRIPT_OUT_OF_RANGE: GetSuggestion$ = "Ensure array index is within declared bounds"
    CASE ERR_CODEGEN_FAILED: GetSuggestion$ = "Check for syntax errors or unsupported language features"
    CASE ERR_UNSUPPORTED_FEATURE: GetSuggestion$ = "Use an alternative syntax or update QBNex"
    CASE ERR_LINK_ERROR: GetSuggestion$ = "Check C++ compiler installation and library paths"
    CASE ELSE: GetSuggestion$ = "Review the error context and consult the documentation"
    END SELECT
END FUNCTION

'-------------------------------------------------------------------------------
' DETAILED ERROR INFORMATION
'-------------------------------------------------------------------------------

FUNCTION GetDetailedSuggestion$ (errCode AS INTEGER, message AS STRING, context AS STRING)
    DIM basicSuggestion AS STRING
    DIM upperMessage AS STRING
    DIM typoToken AS STRING
    DIM suggestion AS STRING

    basicSuggestion = GetSuggestion$(errCode)
    upperMessage = UCASE$(NormalizeDiagnosticMessage$(message))
    typoToken = GetStatementTypoToken$(context, suggestion)

    SELECT CASE errCode
    CASE ERR_INVALID_SYNTAX
        IF INSTR(upperMessage, "$INCLUDE") > 0 THEN
            GetDetailedSuggestion$ = "Use $INCLUDE:'relative-or-absolute-file' and keep the path inside single quotes."
        ELSEIF INSTR(upperMessage, "$IMPORT") > 0 THEN
            GetDetailedSuggestion$ = "Use $IMPORT:'module.name' and keep the module name inside single quotes."
        ELSEIF INSTR(upperMessage, "EXPECTED )") > 0 OR INSTR(upperMessage, "MISSING )") > 0 THEN
            GetDetailedSuggestion$ = "Close the open parenthesis before continuing the expression."
        ELSEIF INSTR(upperMessage, "EXPECTED (") > 0 THEN
            GetDetailedSuggestion$ = "Add the opening parenthesis that starts the parameter list or grouped expression."
        ELSEIF INSTR(upperMessage, "EXPECTED =") > 0 THEN
            GetDetailedSuggestion$ = "Assignments and CONST declarations require = between the name and value."
        ELSEIF INSTR(upperMessage, "EXPECTED ,") > 0 THEN
            GetDetailedSuggestion$ = "Separate adjacent items with commas and remove any trailing comma."
        ELSEIF INSTR(context, "$") > 0 THEN
            GetDetailedSuggestion$ = "Metacommands must start with $. Check for typos in " + context
        ELSEIF INSTR(context, "(") > 0 AND INSTR(context, ")") = 0 THEN
            GetDetailedSuggestion$ = "Missing closing parenthesis. Add ) at the end of the expression."
        ELSEIF INSTR(UCASE$(context), "IF") > 0 AND INSTR(UCASE$(context), "THEN") = 0 THEN
            GetDetailedSuggestion$ = "IF statements require THEN. Example: IF x = 1 THEN"
        ELSEIF INSTR(UCASE$(context), "FOR") > 0 AND INSTR(UCASE$(context), "TO") = 0 THEN
            GetDetailedSuggestion$ = "FOR loops require TO. Example: FOR i = 1 TO 10"
        ELSE
            GetDetailedSuggestion$ = basicSuggestion
        END IF

    CASE ERR_UNEXPECTED_TOKEN
        IF typoToken <> "" THEN
            GetDetailedSuggestion$ = "'" + typoToken + "' is not a built-in statement. Replace it with '" + suggestion + "' if you intended the standard QBasic command."
        ELSE
            GetDetailedSuggestion$ = basicSuggestion
        END IF

    CASE ERR_EXPECTED_THEN
        GetDetailedSuggestion$ = "Add THEN after the IF condition. If you intended a jump, use IF condition GOTO label."

    CASE ERR_EXPECTED_TO
        GetDetailedSuggestion$ = "Write FOR <variable> = start TO finish. Add STEP only after the TO range."

    CASE ERR_EXPECTED_NEXT
        GetDetailedSuggestion$ = "Close the FOR block with NEXT or NEXT <variable>."

    CASE ERR_EXPECTED_LOOP
        GetDetailedSuggestion$ = "Close the DO block with LOOP, LOOP WHILE ..., or LOOP UNTIL ...."

    CASE ERR_EXPECTED_WEND
        GetDetailedSuggestion$ = "Close the WHILE block with WEND."

    CASE ERR_UNDECLARED_VARIABLE
        IF context <> "" THEN
            GetDetailedSuggestion$ = "Variable '" + RTRIM$(context) + "' was used but not declared. Add: DIM " + RTRIM$(context) + " AS [type]"
        ELSE
            GetDetailedSuggestion$ = basicSuggestion
        END IF

    CASE ERR_TYPE_MISMATCH
        IF INSTR(context, "$") > 0 THEN
            GetDetailedSuggestion$ = "String variable used where numeric expected. Use VAL() to convert or remove $ from the variable."
        ELSEIF INSTR(context, "!") > 0 OR INSTR(context, "#") > 0 THEN
            GetDetailedSuggestion$ = "Numeric type mismatch. Use CSNG(), CDBL(), CINT(), or CLNG() to convert explicitly."
        ELSE
            GetDetailedSuggestion$ = basicSuggestion
        END IF

    CASE ERR_REDEFINED_SYMBOL
        GetDetailedSuggestion$ = "Rename the duplicate symbol or remove the earlier declaration in the same scope."

    CASE ERR_UNDEFINED_SYMBOL
        GetDetailedSuggestion$ = "Declare the symbol before using it, or fix the spelling to match the existing declaration."

    CASE ERR_ENCODING_ISSUE
        GetDetailedSuggestion$ = "The source file contains characters that cannot be properly decoded. Save the file as UTF-8 without BOM using Notepad++ or VS Code."

    CASE ERR_INVALID_UTF8
        GetDetailedSuggestion$ = "Invalid UTF-8 byte sequence detected. This often happens when mixing ANSI and UTF-8 encoding. Re-save the file as UTF-8."

    CASE ELSE
        GetDetailedSuggestion$ = basicSuggestion
    END SELECT
END FUNCTION

FUNCTION GetErrorCause$ (errCode AS INTEGER, message AS STRING, context AS STRING)
    DIM upperMessage AS STRING
    DIM typoToken AS STRING
    DIM suggestion AS STRING

    upperMessage = UCASE$(NormalizeDiagnosticMessage$(message))
    typoToken = GetStatementTypoToken$(context, suggestion)

    SELECT CASE errCode
    CASE ERR_INVALID_SYNTAX
        IF INSTR(upperMessage, "EXPECTED )") > 0 OR INSTR(upperMessage, "MISSING )") > 0 THEN
            GetErrorCause$ = "A parenthesized expression was started but never closed."
        ELSEIF INSTR(upperMessage, "$INCLUDE") > 0 OR INSTR(upperMessage, "$IMPORT") > 0 THEN
            GetErrorCause$ = "A compiler metacommand was parsed, but its required quoted path or module name was missing."
        ELSE
            GetErrorCause$ = "The compiler could not parse the statement due to incorrect syntax or unexpected symbols."
        END IF
    CASE ERR_EXPECTED_THEN
        GetErrorCause$ = "The compiler parsed an IF condition but never found THEN or GOTO to finish the statement."
    CASE ERR_EXPECTED_TO
        GetErrorCause$ = "The compiler found a FOR loop header but could not find the TO range separator."
    CASE ERR_EXPECTED_NEXT
        GetErrorCause$ = "A FOR block was opened earlier and the matching NEXT statement is missing."
    CASE ERR_EXPECTED_LOOP
        GetErrorCause$ = "A DO block was opened earlier and the matching LOOP statement is missing."
    CASE ERR_EXPECTED_WEND
        GetErrorCause$ = "A WHILE block was opened earlier and the matching WEND statement is missing."
    CASE ERR_UNCLOSED_STRING
        GetErrorCause$ = "A string literal was opened with " + CHR$(34) + " but never closed."
    CASE ERR_UNDEFINED_SYMBOL
        GetErrorCause$ = "A variable, function, or label was referenced that has not been defined."
    CASE ERR_REDEFINED_SYMBOL
        GetErrorCause$ = "A variable, function, or type was defined more than once in the same scope."
    CASE ERR_TYPE_MISMATCH
        GetErrorCause$ = "An operation was attempted between incompatible data types."
    CASE ERR_UNDECLARED_VARIABLE
        GetErrorCause$ = "OPTION EXPLICIT is enabled and a variable was used without being declared first."
    CASE ERR_ENCODING_ISSUE
        GetErrorCause$ = "Source file encoding does not match expected UTF-8 format."
    CASE ERR_INVALID_UTF8
        GetErrorCause$ = "A multi-byte UTF-8 character sequence is incomplete or malformed."
    CASE ERR_UNEXPECTED_TOKEN
        IF typoToken <> "" THEN
            GetErrorCause$ = "The line starts with '" + typoToken + "', which does not match a known built-in statement. The closest built-in command is '" + suggestion + "'."
        ELSE
            GetErrorCause$ = "A symbol appeared where the compiler did not expect it."
        END IF
    CASE ERR_OUT_OF_MEMORY
        GetErrorCause$ = "System has insufficient memory to continue compilation."
    CASE ERR_FILE_NOT_FOUND
        GetErrorCause$ = "The specified source file or include file could not be found."
    CASE ERR_PERMISSION_DENIED
        GetErrorCause$ = "The operating system denied access to the file."
    CASE ERR_WRONG_ARGUMENT_COUNT
        GetErrorCause$ = "A function or subroutine was called with the wrong number of arguments."
    CASE ERR_INVALID_ARRAY_BOUNDS
        GetErrorCause$ = "Array dimensions must be positive integers."
    CASE ERR_SUBSCRIPT_OUT_OF_RANGE
        GetErrorCause$ = "An array index exceeds the declared bounds."
    CASE ERR_CODEGEN_FAILED
        GetErrorCause$ = "An internal error occurred during C++ code generation."
    CASE ERR_UNSUPPORTED_FEATURE
        GetErrorCause$ = "This language feature is not yet implemented in QBNex."
    CASE ERR_LINK_ERROR
        GetErrorCause$ = "The C++ linker failed to create the executable."
    CASE ELSE
        GetErrorCause$ = ""
    END SELECT
END FUNCTION

FUNCTION GetFixExample$ (errCode AS INTEGER, message AS STRING, context AS STRING)
    DIM upperMessage AS STRING
    DIM typoToken AS STRING
    DIM suggestion AS STRING

    upperMessage = UCASE$(NormalizeDiagnosticMessage$(message))
    typoToken = GetStatementTypoToken$(context, suggestion)

    SELECT CASE errCode
    CASE ERR_INVALID_SYNTAX
        IF INSTR(upperMessage, "$INCLUDE") > 0 THEN
            GetFixExample$ = "$INCLUDE:'./stdlib/math.bas'"
        ELSEIF INSTR(upperMessage, "$IMPORT") > 0 THEN
            GetFixExample$ = "$IMPORT:'module.name'"
        ELSEIF INSTR(upperMessage, "EXPECTED )") > 0 OR INSTR(upperMessage, "MISSING )") > 0 THEN
            GetFixExample$ = "PRINT (value + 1)"
        ELSEIF INSTR(UCASE$(context), "IF") > 0 THEN
            GetFixExample$ = "IF x = 1 THEN PRINT " + CHR$(34) + "Yes" + CHR$(34) + " ELSE PRINT " + CHR$(34) + "No" + CHR$(34)
        ELSEIF INSTR(UCASE$(context), "FOR") > 0 THEN
            GetFixExample$ = "FOR i = 1 TO 10 STEP 2: PRINT i: NEXT"
        ELSEIF INSTR(UCASE$(context), "SUB") > 0 THEN
            GetFixExample$ = "SUB MySub(param AS INTEGER): ' code : END SUB"
        ELSEIF INSTR(UCASE$(context), "FUNCTION") > 0 THEN
            GetFixExample$ = "FUNCTION MyFunc(x) AS INTEGER: MyFunc = x * 2: END FUNCTION"
        ELSE
            GetFixExample$ = ""
        END IF

    CASE ERR_UNEXPECTED_TOKEN
        IF typoToken <> "" AND suggestion = "PRINT" THEN
            GetFixExample$ = "PRINT " + CHR$(34) + "Hello" + CHR$(34)
        ELSEIF typoToken <> "" THEN
            GetFixExample$ = suggestion + " ..."
        ELSE
            GetFixExample$ = ""
        END IF

    CASE ERR_EXPECTED_THEN
        GetFixExample$ = "IF score > 10 THEN PRINT " + CHR$(34) + "win" + CHR$(34)

    CASE ERR_EXPECTED_TO
        GetFixExample$ = "FOR i = 1 TO 10: PRINT i: NEXT"

    CASE ERR_EXPECTED_NEXT
        GetFixExample$ = "FOR i = 1 TO 10: PRINT i: NEXT i"

    CASE ERR_EXPECTED_LOOP
        GetFixExample$ = "DO WHILE running: PRINT running: LOOP"

    CASE ERR_EXPECTED_WEND
        GetFixExample$ = "WHILE ready: PRINT ready: WEND"

    CASE ERR_UNDECLARED_VARIABLE
        GetFixExample$ = "DIM myVariable AS STRING"

    CASE ERR_TYPE_MISMATCH
        GetFixExample$ = "numValue = VAL(stringValue$)"

    CASE ERR_UNCLOSED_STRING
        GetFixExample$ = "text$ = " + CHR$(34) + "Complete sentence" + CHR$(34)

    CASE ERR_WRONG_ARGUMENT_COUNT
        GetFixExample$ = "Check the function definition for the required parameters."

    CASE ERR_INVALID_ARRAY_BOUNDS
        GetFixExample$ = "DIM arr(1 TO 100)"

    CASE ERR_ENCODING_ISSUE
        GetFixExample$ = "Encoding > Convert to UTF-8 without BOM"

    CASE ERR_REDEFINED_SYMBOL
        GetFixExample$ = "Use unique names: myVar1, myVar2"

    CASE ELSE
        GetFixExample$ = ""
    END SELECT
END FUNCTION

FUNCTION FindErrorColumn% (errCode AS INTEGER, message AS STRING, context AS STRING)
    DIM columnPos AS INTEGER
    DIM token AS STRING
    DIM upperContext AS STRING
    DIM i AS INTEGER
    DIM typoToken AS STRING
    DIM suggestion AS STRING

    upperContext = UCASE$(context)
    token = ExtractDiagnosticToken$(message)
    typoToken = GetStatementTypoToken$(context, suggestion)

    SELECT CASE errCode
    CASE ERR_UNCLOSED_STRING
        columnPos = INSTR(context, CHR$(34))
        IF columnPos > 0 THEN
            FindErrorColumn% = columnPos
            EXIT FUNCTION
        END IF

    CASE ERR_EXPECTED_THEN, ERR_EXPECTED_TO, ERR_EXPECTED_NEXT, ERR_EXPECTED_LOOP, ERR_EXPECTED_WEND
        FindErrorColumn% = LEN(RTRIM$(context)) + 1
        EXIT FUNCTION

    CASE ERR_REDEFINED_SYMBOL, ERR_UNDEFINED_SYMBOL
        IF token <> "" THEN
            columnPos = INSTR(upperContext, UCASE$(token))
            IF columnPos > 0 THEN
                FindErrorColumn% = columnPos
                EXIT FUNCTION
            END IF
        END IF

    CASE ERR_UNEXPECTED_TOKEN
        IF typoToken <> "" THEN
            FindErrorColumn% = INSTR(upperContext, typoToken)
            IF FindErrorColumn% = 0 THEN FindErrorColumn% = 1
            EXIT FUNCTION
        END IF
        FOR i = 1 TO LEN(context)
            SELECT CASE MID$(context, i, 1)
            CASE "@", "#", "$", "%", "&", "*", "^", "!"
                FindErrorColumn% = i
                EXIT FUNCTION
            END SELECT
        NEXT
    END SELECT

    FindErrorColumn% = 0
END FUNCTION

FUNCTION GetSeverityString$ (severity AS INTEGER)
    SELECT CASE severity
    CASE ERR_INFO: GetSeverityString$ = "INFO"
    CASE ERR_WARNING: GetSeverityString$ = "WARNING"
    CASE ERR_ERROR: GetSeverityString$ = "ERROR"
    CASE ERR_FATAL: GetSeverityString$ = "FATAL"
    CASE ELSE: GetSeverityString$ = "UNKNOWN"
    END SELECT
END FUNCTION

FUNCTION PadLeftZero$ (value AS LONG, WIDTH AS INTEGER)
    DIM s AS STRING

    s = LTRIM$(STR$(value))
    DO WHILE LEN(s) < WIDTH
        s = "0" + s
    LOOP

    PadLeftZero$ = s
END FUNCTION

FUNCTION GetRenderedSeverity% (severity AS INTEGER)
    GetRenderedSeverity% = severity
    IF severity = ERR_WARNING AND WarningsAsErrors THEN GetRenderedSeverity% = ERR_ERROR
END FUNCTION

FUNCTION GetDiagnosticKind$ (severity AS INTEGER)
    SELECT CASE GetRenderedSeverity%(severity)
    CASE ERR_WARNING
        GetDiagnosticKind$ = "warning"
    CASE ERR_INFO
        GetDiagnosticKind$ = "info"
    CASE ELSE
        GetDiagnosticKind$ = "error"
    END SELECT
END FUNCTION

FUNCTION GetDiagnosticCodeTag$ (severity AS INTEGER, errCode AS INTEGER)
    SELECT CASE severity
    CASE ERR_WARNING
        GetDiagnosticCodeTag$ = "W" + PadLeftZero$(errCode, 4)
    CASE ERR_INFO
        GetDiagnosticCodeTag$ = "I" + PadLeftZero$(errCode, 4)
    CASE ELSE
        GetDiagnosticCodeTag$ = "E" + PadLeftZero$(errCode, 4)
    END SELECT
END FUNCTION

FUNCTION GetSeverityColor% (severity AS INTEGER)
    SELECT CASE GetRenderedSeverity%(severity)
    CASE ERR_INFO
        GetSeverityColor% = 11
    CASE ERR_WARNING
        GetSeverityColor% = 14
    CASE ERR_FATAL
        GetSeverityColor% = 4
    CASE ELSE
        GetSeverityColor% = 12
    END SELECT
END FUNCTION

FUNCTION GetDiagnosticTitle$ (severity AS INTEGER)
    SELECT CASE GetRenderedSeverity%(severity)
    CASE ERR_WARNING
        GetDiagnosticTitle$ = "Warning"
    CASE ERR_INFO
        GetDiagnosticTitle$ = "Info"
    CASE ERR_FATAL
        GetDiagnosticTitle$ = "Fatal"
    CASE ELSE
        GetDiagnosticTitle$ = "Error"
    END SELECT
END FUNCTION

FUNCTION GetDiagnosticMarker$ (severity AS INTEGER)
    SELECT CASE GetRenderedSeverity%(severity)
    CASE ERR_WARNING
        GetDiagnosticMarker$ = "[~]"
    CASE ERR_INFO
        GetDiagnosticMarker$ = "[i]"
    CASE ERR_FATAL
        GetDiagnosticMarker$ = "[!!]"
    CASE ELSE
        GetDiagnosticMarker$ = "[x]"
    END SELECT
END FUNCTION

FUNCTION FormatDiagnosticLocation$ (fileName AS STRING, lineNumber AS LONG, columnNumber AS INTEGER)
    DIM location AS STRING

    location = RTRIM$(fileName)
    IF location = "" THEN
        FormatDiagnosticLocation$ = ""
        EXIT FUNCTION
    END IF

    IF lineNumber > 0 THEN
        location = location + "(" + LTRIM$(STR$(lineNumber))
        IF columnNumber > 0 THEN location = location + "," + LTRIM$(STR$(columnNumber))
        location = location + ")"
    END IF

    FormatDiagnosticLocation$ = location
END FUNCTION

FUNCTION FormatDiagnosticFlow$ (phaseName AS STRING, contextTrace AS STRING)
    DIM flowText AS STRING

    flowText = ""
    phaseName = RTRIM$(phaseName)
    contextTrace = RTRIM$(contextTrace)

    IF phaseName <> "" THEN flowText = phaseName
    IF contextTrace <> "" THEN
        IF flowText <> "" THEN
            flowText = flowText + " :: " + contextTrace
        ELSE
            flowText = contextTrace
        END IF
    END IF

    FormatDiagnosticFlow$ = flowText
END FUNCTION

'-------------------------------------------------------------------------------
' ERROR OUTPUT
'-------------------------------------------------------------------------------

SUB PrintError (errIdx AS LONG)
    DIM errInfo AS ErrorInfo
    DIM diagnosticTitle AS STRING
    DIM diagnosticMarker AS STRING
    DIM codeTag AS STRING
    DIM headline AS STRING
    DIM pointerHint AS STRING
    DIM lineStr AS STRING
    DIM gutter AS STRING
    DIM severityColor AS INTEGER
    DIM locationText AS STRING
    DIM flowText AS STRING

    IF errIdx < 1 OR errIdx > ErrorCount THEN EXIT SUB

    errInfo = Errors(errIdx)
    diagnosticTitle = GetDiagnosticTitle$(errInfo.severity)
    diagnosticMarker = GetDiagnosticMarker$(errInfo.severity)
    codeTag = GetDiagnosticCodeTag$(errInfo.severity, errInfo.errorCode)
    headline = GetDiagnosticHeadline$(errInfo.errorCode, RTRIM$(errInfo.message), RTRIM$(errInfo.context))
    pointerHint = GetPointerHint$(errInfo.errorCode, RTRIM$(errInfo.message), RTRIM$(errInfo.context))
    severityColor = GetSeverityColor%(errInfo.severity)
    locationText = FormatDiagnosticLocation$(RTRIM$(errInfo.fileName), errInfo.lineNumber, errInfo.columnNumber)
    flowText = FormatDiagnosticFlow$(RTRIM$(errInfo.phaseName), RTRIM$(errInfo.contextTrace))

    IF errIdx > 1 THEN PRINT

    COLOR severityColor
    PRINT diagnosticMarker;
    COLOR 7
    PRINT " QBNex :: " + diagnosticTitle + " [" + codeTag + "]  ";
    COLOR 15
    PRINT headline
    COLOR 7

    IF locationText <> "" THEN PRINT "  [@] " + locationText

    IF RTRIM$(errInfo.context) <> "" THEN
        IF errInfo.lineNumber > 0 THEN
            lineStr = LTRIM$(STR$(errInfo.lineNumber))
        ELSE
            lineStr = "?"
        END IF
        gutter = RepeatSpaces$(LEN(lineStr))

        PRINT "  [#] source"
        COLOR 8
        PRINT "    " + lineStr + " | ";
        COLOR 7
        PRINT RTRIM$(errInfo.context)

        IF errInfo.columnNumber > 0 THEN
            COLOR severityColor
            PRINT "    " + gutter + " | " + RepeatSpaces$(errInfo.columnNumber - 1) + "^";
            IF pointerHint <> "" THEN
                PRINT " " + pointerHint
            ELSE
                PRINT
            END IF
            COLOR 7
        END IF
    END IF

    IF RTRIM$(errInfo.suggestion) <> "" THEN
        COLOR 10
        PRINT "  [>] next     " + RTRIM$(errInfo.suggestion)
        COLOR 7
    END IF

    IF flowText <> "" THEN PRINT "  [::] flow    " + flowText
    IF WarningsAsErrors AND errInfo.severity = ERR_WARNING THEN PRINT "  [*] config   warning promoted to blocking diagnostic"
    IF errInfo.duplicateCount > 1 THEN PRINT "  [=] repeat   seen " + LTRIM$(STR$(errInfo.duplicateCount)) + " times; duplicates hidden"

    IF VerboseMode THEN
        IF RTRIM$(errInfo.locationNote) <> "" THEN PRINT "  [^] where    " + RTRIM$(errInfo.locationNote)
        IF RTRIM$(errInfo.message) <> "" AND UCASE$(RTRIM$(errInfo.message)) <> UCASE$(headline) THEN PRINT "  [.] detail   " + RTRIM$(errInfo.message)
        IF RTRIM$(errInfo.secondaryContext) <> "" THEN PRINT "  [>>] while   " + RTRIM$(errInfo.secondaryContext)
        IF RTRIM$(errInfo.cause) <> "" THEN PRINT "  [!] cause    " + RTRIM$(errInfo.cause)
        IF RTRIM$(errInfo.fixExample) <> "" THEN PRINT "  [+] example  " + RTRIM$(errInfo.fixExample)
    END IF

    IF ErrorOutputFile > 0 THEN
        PRINT #ErrorOutputFile, diagnosticMarker + " QBNex :: " + diagnosticTitle + " [" + codeTag + "]  " + headline
        IF locationText <> "" THEN PRINT #ErrorOutputFile, "  [@] " + locationText
        IF RTRIM$(errInfo.context) <> "" THEN
            PRINT #ErrorOutputFile, "  [#] source"
            PRINT #ErrorOutputFile, "    " + lineStr + " | " + RTRIM$(errInfo.context)
            IF errInfo.columnNumber > 0 THEN
                PRINT #ErrorOutputFile, "    " + gutter + " | " + RepeatSpaces$(errInfo.columnNumber - 1) + "^";
                IF pointerHint <> "" THEN
                    PRINT #ErrorOutputFile, " " + pointerHint
                ELSE
                    PRINT #ErrorOutputFile,
                END IF
            END IF
        END IF
        IF RTRIM$(errInfo.suggestion) <> "" THEN PRINT #ErrorOutputFile, "  [>] next     " + RTRIM$(errInfo.suggestion)
        IF flowText <> "" THEN PRINT #ErrorOutputFile, "  [::] flow    " + flowText
        IF WarningsAsErrors AND errInfo.severity = ERR_WARNING THEN PRINT #ErrorOutputFile, "  [*] config   warning promoted to blocking diagnostic"
        IF errInfo.duplicateCount > 1 THEN PRINT #ErrorOutputFile, "  [=] repeat   seen " + LTRIM$(STR$(errInfo.duplicateCount)) + " times; duplicates hidden"
        IF VerboseMode THEN
            IF RTRIM$(errInfo.locationNote) <> "" THEN PRINT #ErrorOutputFile, "  [^] where    " + RTRIM$(errInfo.locationNote)
            IF RTRIM$(errInfo.message) <> "" AND UCASE$(RTRIM$(errInfo.message)) <> UCASE$(headline) THEN PRINT #ErrorOutputFile, "  [.] detail   " + RTRIM$(errInfo.message)
            IF RTRIM$(errInfo.secondaryContext) <> "" THEN PRINT #ErrorOutputFile, "  [>>] while   " + RTRIM$(errInfo.secondaryContext)
            IF RTRIM$(errInfo.cause) <> "" THEN PRINT #ErrorOutputFile, "  [!] cause    " + RTRIM$(errInfo.cause)
            IF RTRIM$(errInfo.fixExample) <> "" THEN PRINT #ErrorOutputFile, "  [+] example  " + RTRIM$(errInfo.fixExample)
        END IF
        PRINT #ErrorOutputFile, ""
    END IF
END SUB

SUB PrintAllErrors
    DIM i AS LONG

    IF ErrorCount = 0 THEN
        PRINT "No errors reported."
        EXIT SUB
    END IF

    FOR i = 1 TO ErrorCount
        PrintError i
    NEXT

    PrintErrorSummary
END SUB

SUB PrintErrorSummary
    DIM blockingCount AS LONG
    DIM summaryMarker AS STRING

    IF Stats.totalCount = 0 THEN EXIT SUB

    PRINT
    IF Stats.hasErrors THEN
        blockingCount = GetBlockingIssueCount&
        IF Stats.fatalCount > 0 THEN
            summaryMarker = "[!!]"
        ELSE
            summaryMarker = "[x]"
        END IF
        COLOR 12
        PRINT summaryMarker; " QBNex :: Build Halted  "; blockingCount; " blocking diagnostic(s)";
        COLOR 7
        IF Stats.warningCount > 0 AND NOT WarningsAsErrors THEN
            PRINT " and "; Stats.warningCount; " warning(s)"
        ELSEIF Stats.warningCount > 0 AND WarningsAsErrors THEN
            PRINT " ("; Stats.warningCount; " warning(s) promoted)"
        ELSE
            PRINT
        END IF
        IF Stats.suppressedDuplicateCount > 0 THEN PRINT "[=] QBNex :: Hidden  "; Stats.suppressedDuplicateCount; " duplicate diagnostic(s)."
        IF VerboseMode THEN PRINT "[::] QBNex :: Flow   fix the first blocking diagnostic first; later ones may be follow-on effects."
    ELSE
        IF Stats.warningCount > 0 THEN
            COLOR 14
            PRINT "[~] QBNex :: Build Complete  "; Stats.warningCount; " warning(s)"
            COLOR 7
        ELSE
            PRINT "[ok] QBNex :: Build Complete"
        END IF
        IF Stats.suppressedDuplicateCount > 0 THEN PRINT "[=] QBNex :: Hidden  "; Stats.suppressedDuplicateCount; " duplicate diagnostic(s)."
    END IF
END SUB

'-------------------------------------------------------------------------------
' ERROR RECOVERY
'-------------------------------------------------------------------------------

FUNCTION CanRecover% (errCode AS INTEGER)
    SELECT CASE errCode
    CASE ERR_NO_SOURCE_FILE, ERR_FILE_NOT_FOUND, ERR_OUT_OF_MEMORY
        CanRecover% = 0
    CASE ERR_FATAL
        CanRecover% = 0
    CASE ELSE
        CanRecover% = -1
    END SELECT
END FUNCTION

SUB MarkRecovered (errIdx AS LONG)
    IF errIdx >= 1 AND errIdx <= ErrorCount THEN Errors(errIdx).recovered = -1
END SUB

FUNCTION ShouldAbort%
    ShouldAbort% = (Stats.fatalCount > 0) OR (GetBlockingIssueCount& >= Stats.maxErrors)
END FUNCTION

'-------------------------------------------------------------------------------
' QUERY FUNCTIONS
'-------------------------------------------------------------------------------

FUNCTION HasErrors%
    HasErrors% = Stats.hasErrors
END FUNCTION

FUNCTION GetErrorCount%
    GetErrorCount% = ErrorCount
END FUNCTION

FUNCTION GetWarningCount%
    GetWarningCount% = Stats.warningCount
END FUNCTION

FUNCTION GetFatalCount%
    GetFatalCount% = Stats.fatalCount
END FUNCTION

FUNCTION GetSuppressedDuplicateCount%
    GetSuppressedDuplicateCount% = Stats.suppressedDuplicateCount
END FUNCTION

'-------------------------------------------------------------------------------
' CLEANUP
'-------------------------------------------------------------------------------

SUB CleanupErrorHandler
    ErrorCount = 0
    CurrentFile = ""
    CurrentErrorPhase = ""
    ClearErrorContextStack

    IF ErrorOutputFile > 0 THEN
        CLOSE #ErrorOutputFile
        ErrorOutputFile = 0
    END IF
END SUB
