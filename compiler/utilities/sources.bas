FUNCTION UTF8ByteAt% (text AS STRING, bytePos AS LONG)
    IF bytePos < 1 OR bytePos > LEN(text) THEN
        UTF8ByteAt% = -1
    ELSE
        UTF8ByteAt% = ASC(text, bytePos)
    END IF
END FUNCTION

FUNCTION IsUTF8ContinuationByte% (value AS INTEGER)
    IF value >= 128 AND value <= 191 THEN
        IsUTF8ContinuationByte% = -1
    ELSE
        IsUTF8ContinuationByte% = 0
    END IF
END FUNCTION

FUNCTION UTF8SequenceLength% (text AS STRING, bytePos AS LONG)
    'length of the well-formed UTF-8 sequence that starts at bytePos, 0 if the byte does not start one
    DIM b1 AS INTEGER
    DIM b2 AS INTEGER
    DIM lo AS INTEGER
    DIM hi AS INTEGER
    DIM n AS INTEGER
    DIM k AS INTEGER

    UTF8SequenceLength% = 0
    b1 = UTF8ByteAt%(text, bytePos)
    IF b1 < 0 THEN EXIT FUNCTION
    IF b1 < 128 THEN UTF8SequenceLength% = 1: EXIT FUNCTION
    lo = 128: hi = 191
    SELECT CASE b1
        CASE 194 TO 223: n = 2
        CASE 224: n = 3: lo = 160
        CASE 225 TO 236, 238, 239: n = 3
        CASE 237: n = 3: hi = 159
        CASE 240: n = 4: lo = 144
        CASE 241 TO 243: n = 4
        CASE 244: n = 4: hi = 143
        CASE ELSE: EXIT FUNCTION
    END SELECT
    b2 = UTF8ByteAt%(text, bytePos + 1)
    IF b2 < lo OR b2 > hi THEN EXIT FUNCTION
    FOR k = 2 TO n - 1
        IF IsUTF8ContinuationByte%(UTF8ByteAt%(text, bytePos + k)) = 0 THEN EXIT FUNCTION
    NEXT
    UTF8SequenceLength% = n
END FUNCTION

FUNCTION ValidateUTF8Buffer% (text AS STRING, invalidPos AS LONG, invalidLine AS LONG, invalidContext AS STRING, declaredUTF8 AS INTEGER)
    'Classifies the source bytes. Every result keeps the bytes exactly as they are (QBasic/QB64 strings are bytes).
    ' -1 = ASCII or well-formed UTF-8
    '  1 = 8-bit code page text (CP437, as QBasic and QB64 read it): bytes that are not UTF-8 and no sign of UTF-8
    '  0 = broken: NUL bytes (UTF-16 without BOM), or invalid bytes in a file that is otherwise UTF-8
    '      (UTF-8 BOM, or several multi-byte UTF-8 sequences that CP437 text would not contain)
    DIM bytePos AS LONG
    DIM i AS LONG
    DIM lineStart AS LONG
    DIM lineEnd AS LONG
    DIM n AS INTEGER
    DIM nulCount AS LONG
    DIM longSeqCount AS LONG
    DIM badCount AS LONG

    invalidPos = 0
    invalidLine = 1
    invalidContext = ""

    bytePos = 1
    DO WHILE bytePos <= LEN(text)
        n = UTF8SequenceLength%(text, bytePos)
        IF n = 0 THEN
            badCount = badCount + 1
            IF invalidPos = 0 THEN invalidPos = bytePos
            bytePos = bytePos + 1
        ELSE
            IF n = 1 THEN
                IF ASC(text, bytePos) = 0 THEN
                    nulCount = nulCount + 1
                    IF invalidPos = 0 THEN invalidPos = bytePos
                END IF
            ELSEIF n >= 3 THEN
                longSeqCount = longSeqCount + 1
            END IF
            bytePos = bytePos + n
        END IF
    LOOP

    IF nulCount = 0 AND badCount = 0 THEN ValidateUTF8Buffer% = -1: EXIT FUNCTION
    'CP437 box/graphics characters form a valid 3-byte UTF-8 sequence now and then (E2 95 90 =
    '"Γò É" is also U+2550), so only call it UTF-8 when such sequences outnumber the bad bytes
    IF nulCount = 0 AND declaredUTF8 = 0 AND (longSeqCount < 2 OR longSeqCount <= badCount) THEN ValidateUTF8Buffer% = 1: EXIT FUNCTION

    invalidLine = 1
    FOR i = 1 TO invalidPos - 1
        IF MID$(text, i, 1) = CHR$(10) THEN invalidLine = invalidLine + 1
    NEXT

    lineStart = invalidPos
    DO WHILE lineStart > 1
        IF MID$(text, lineStart - 1, 1) = CHR$(10) OR MID$(text, lineStart - 1, 1) = CHR$(13) THEN EXIT DO
        lineStart = lineStart - 1
    LOOP

    lineEnd = invalidPos
    DO WHILE lineEnd <= LEN(text)
        IF MID$(text, lineEnd, 1) = CHR$(10) OR MID$(text, lineEnd, 1) = CHR$(13) THEN EXIT DO
        lineEnd = lineEnd + 1
    LOOP

    invalidContext = MID$(text, lineStart, lineEnd - lineStart)
    ValidateUTF8Buffer% = 0
END FUNCTION

FUNCTION SourceLooksBinary% (text AS STRING)
    'NUL bytes, but far fewer than UTF-16 text has (one per character): binary data
    DIM p AS LONG, nuls AS LONG
    p = INSTR(text, CHR$(0))
    DO WHILE p > 0
        nuls = nuls + 1
        p = INSTR(p + 1, text, CHR$(0))
    LOOP
    IF nuls > 0 AND nuls * 4 < LEN(text) THEN SourceLooksBinary% = -1
END FUNCTION

SUB lineinput3load (f$)
    DIM invalidPos AS LONG
    DIM invalidLine AS LONG
    DIM invalidContext AS STRING
    DIM hadBOM AS INTEGER

    OPEN f$ FOR BINARY AS #1
    l = LOF(1)
    lineinput3buffer$ = SPACE$(l)
    GET #1, , lineinput3buffer$
    IF LEN(lineinput3buffer$) THEN IF RIGHT$(lineinput3buffer$, 1) = CHR$(26) THEN lineinput3buffer$ = LEFT$(lineinput3buffer$, LEN(lineinput3buffer$) - 1)
    CLOSE #1

    ' Check for and handle UTF-8 BOM (EF BB BF = 239 187 191)
    IF LEN(lineinput3buffer$) >= 3 THEN
        IF ASC(lineinput3buffer$, 1) = 239 AND ASC(lineinput3buffer$, 2) = 187 AND ASC(lineinput3buffer$, 3) = 191 THEN
            ' Skip UTF-8 BOM
            hadBOM = -1
            lineinput3buffer$ = MID$(lineinput3buffer$, 4)
        END IF
    END IF

    ' QuickBASIC 4.x "fast load" binary format (tokenized, starts FC 00): not source text
    IF LEN(lineinput3buffer$) >= 2 THEN
        IF ASC(lineinput3buffer$, 1) = 252 AND ASC(lineinput3buffer$, 2) = 0 THEN
            SetCurrentFile f$
            ReportDetailedErrorWithSeverity ERR_ENCODING_ISSUE, ERR_FATAL, "File is in QuickBASIC binary (fast load) format, not text", 1, "", "", "Open it in QuickBASIC and save it as Text (File > Save As > Text - Readable by Other Programs)."
            compfailed = 1
            lineinput3buffer$ = ""
        END IF
    END IF

    ' Check for UTF-16 BOM (not supported - warn user)
    IF LEN(lineinput3buffer$) >= 2 THEN
        IF ASC(lineinput3buffer$, 1) = 255 AND ASC(lineinput3buffer$, 2) = 254 THEN
            ' UTF-16 LE - reject early so we do not compile an empty/stale program
            SetCurrentFile f$
            ReportDetailedErrorWithSeverity ERR_ENCODING_ISSUE, ERR_FATAL, "UTF-16 LE encoding detected. Please convert file to UTF-8", 1, "", "", "Re-save this file as UTF-8 before compiling."
            compfailed = 1
            lineinput3buffer$ = ""
        ELSEIF ASC(lineinput3buffer$, 1) = 254 AND ASC(lineinput3buffer$, 2) = 255 THEN
            ' UTF-16 BE - reject early so we do not compile an empty/stale program
            SetCurrentFile f$
            ReportDetailedErrorWithSeverity ERR_ENCODING_ISSUE, ERR_FATAL, "UTF-16 BE encoding detected. Please convert file to UTF-8", 1, "", "", "Re-save this file as UTF-8 before compiling."
            compfailed = 1
            lineinput3buffer$ = ""
        END IF
    END IF

    IF LEN(lineinput3buffer$) THEN
        IF ValidateUTF8Buffer%(lineinput3buffer$, invalidPos, invalidLine, invalidContext, hadBOM) = 0 THEN
            SetCurrentFile f$
            IF SourceLooksBinary%(lineinput3buffer$) THEN
                ReportDetailedErrorWithSeverity ERR_INVALID_UTF8, ERR_FATAL, "File contains binary data, not BASIC source text", invalidLine, "", "", "Check that this is the program's .BAS text file (not a compressed, tokenized or data file)."
            ELSE
                ReportDetailedErrorWithSeverity ERR_INVALID_UTF8, ERR_FATAL, "Invalid UTF-8 byte sequence detected in source file (or UTF-16 without BOM)", invalidLine, invalidContext, "", "Re-save this file as UTF-8 without mixing ANSI/UTF-16 bytes."
            END IF
            compfailed = 1
            lineinput3buffer$ = ""
        END IF
    END IF

    DefFn_RewriteBuffer 'single-line DEF FN -> FUNCTION (deffn.bas)
    lineinput3index = 1
END SUB

FUNCTION lineinput3$
    'returns CHR$(13) if no more lines are available
    l = LEN(lineinput3buffer$)
    IF lineinput3index > l THEN lineinput3$ = CHR$(13): EXIT FUNCTION
    c13 = INSTR(lineinput3index, lineinput3buffer$, CHR$(13))
    c10 = INSTR(lineinput3index, lineinput3buffer$, CHR$(10))
    IF c10 = 0 AND c13 = 0 THEN
        lineinput3$ = MID$(lineinput3buffer$, lineinput3index, l - lineinput3index + 1)
        lineinput3index = l + 1
        EXIT FUNCTION
    END IF
    IF c10 = 0 THEN c10 = 2147483647
    IF c13 = 0 THEN c13 = 2147483647
    IF c10 < c13 THEN
        '10 before 13
        lineinput3$ = MID$(lineinput3buffer$, lineinput3index, c10 - lineinput3index)
        lineinput3index = c10 + 1
        IF lineinput3index <= l THEN
            IF ASC(MID$(lineinput3buffer$, lineinput3index, 1)) = 13 THEN lineinput3index = lineinput3index + 1
        END IF
    ELSE
        '13 before 10
        lineinput3$ = MID$(lineinput3buffer$, lineinput3index, c13 - lineinput3index)
        lineinput3index = c13 + 1
        IF lineinput3index <= l THEN
            IF ASC(MID$(lineinput3buffer$, lineinput3index, 1)) = 10 THEN lineinput3index = lineinput3index + 1
        END IF
    END IF
END FUNCTION
