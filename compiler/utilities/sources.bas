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

FUNCTION ValidateUTF8Buffer% (text AS STRING, invalidPos AS LONG, invalidLine AS LONG, invalidContext AS STRING)
    DIM bytePos AS LONG
    DIM i AS LONG
    DIM lineStart AS LONG
    DIM lineEnd AS LONG
    DIM b1 AS INTEGER
    DIM b2 AS INTEGER
    DIM b3 AS INTEGER
    DIM b4 AS INTEGER

    invalidPos = 0
    invalidLine = 1
    invalidContext = ""

    bytePos = 1
    DO WHILE bytePos <= LEN(text)
        b1 = UTF8ByteAt%(text, bytePos)

        IF b1 < 128 THEN
            bytePos = bytePos + 1
        ELSEIF b1 >= 194 AND b1 <= 223 THEN
            b2 = UTF8ByteAt%(text, bytePos + 1)
            IF IsUTF8ContinuationByte%(b2) = 0 THEN GOTO invalid_utf8
            bytePos = bytePos + 2
        ELSEIF b1 = 224 THEN
            b2 = UTF8ByteAt%(text, bytePos + 1)
            b3 = UTF8ByteAt%(text, bytePos + 2)
            IF b2 < 160 OR b2 > 191 THEN GOTO invalid_utf8
            IF IsUTF8ContinuationByte%(b3) = 0 THEN GOTO invalid_utf8
            bytePos = bytePos + 3
        ELSEIF (b1 >= 225 AND b1 <= 236) OR (b1 >= 238 AND b1 <= 239) THEN
            b2 = UTF8ByteAt%(text, bytePos + 1)
            b3 = UTF8ByteAt%(text, bytePos + 2)
            IF IsUTF8ContinuationByte%(b2) = 0 THEN GOTO invalid_utf8
            IF IsUTF8ContinuationByte%(b3) = 0 THEN GOTO invalid_utf8
            bytePos = bytePos + 3
        ELSEIF b1 = 237 THEN
            b2 = UTF8ByteAt%(text, bytePos + 1)
            b3 = UTF8ByteAt%(text, bytePos + 2)
            IF b2 < 128 OR b2 > 159 THEN GOTO invalid_utf8
            IF IsUTF8ContinuationByte%(b3) = 0 THEN GOTO invalid_utf8
            bytePos = bytePos + 3
        ELSEIF b1 = 240 THEN
            b2 = UTF8ByteAt%(text, bytePos + 1)
            b3 = UTF8ByteAt%(text, bytePos + 2)
            b4 = UTF8ByteAt%(text, bytePos + 3)
            IF b2 < 144 OR b2 > 191 THEN GOTO invalid_utf8
            IF IsUTF8ContinuationByte%(b3) = 0 THEN GOTO invalid_utf8
            IF IsUTF8ContinuationByte%(b4) = 0 THEN GOTO invalid_utf8
            bytePos = bytePos + 4
        ELSEIF b1 >= 241 AND b1 <= 243 THEN
            b2 = UTF8ByteAt%(text, bytePos + 1)
            b3 = UTF8ByteAt%(text, bytePos + 2)
            b4 = UTF8ByteAt%(text, bytePos + 3)
            IF IsUTF8ContinuationByte%(b2) = 0 THEN GOTO invalid_utf8
            IF IsUTF8ContinuationByte%(b3) = 0 THEN GOTO invalid_utf8
            IF IsUTF8ContinuationByte%(b4) = 0 THEN GOTO invalid_utf8
            bytePos = bytePos + 4
        ELSEIF b1 = 244 THEN
            b2 = UTF8ByteAt%(text, bytePos + 1)
            b3 = UTF8ByteAt%(text, bytePos + 2)
            b4 = UTF8ByteAt%(text, bytePos + 3)
            IF b2 < 128 OR b2 > 143 THEN GOTO invalid_utf8
            IF IsUTF8ContinuationByte%(b3) = 0 THEN GOTO invalid_utf8
            IF IsUTF8ContinuationByte%(b4) = 0 THEN GOTO invalid_utf8
            bytePos = bytePos + 4
        ELSE
            GOTO invalid_utf8
        END IF
    LOOP

    ValidateUTF8Buffer% = -1
    EXIT FUNCTION

    invalid_utf8:
    invalidPos = bytePos
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

SUB lineinput3load (f$)
    DIM invalidPos AS LONG
    DIM invalidLine AS LONG
    DIM invalidContext AS STRING

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
            lineinput3buffer$ = MID$(lineinput3buffer$, 4)
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
        IF ValidateUTF8Buffer%(lineinput3buffer$, invalidPos, invalidLine, invalidContext) = 0 THEN
            SetCurrentFile f$
            ReportDetailedErrorWithSeverity ERR_INVALID_UTF8, ERR_FATAL, "Invalid UTF-8 byte sequence detected in source file", invalidLine, invalidContext, "", "Re-save this file as UTF-8 without mixing ANSI/UTF-16 bytes."
            compfailed = 1
            lineinput3buffer$ = ""
        END IF
    END IF

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
