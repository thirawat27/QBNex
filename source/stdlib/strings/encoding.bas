' =============================================================================
' QBNex String Library — Encoding helpers — encoding.bas
' =============================================================================
'
' Base64 encode/decode, URL encode/decode, and UTF-8 byte scanning.
'
' Usage:
'
'   '$INCLUDE:'stdlib/strings/encoding.bas'
'
'   PRINT Base64Encode$("Hello, World!")   ' SGVsbG8sIFdvcmxkIQ==
'   PRINT Base64Decode$("SGVsbG8sIFdvcmxkIQ==")   ' Hello, World!
'
'   PRINT UrlEncode$("hello world & more")  ' hello+world+%26+more
'   PRINT UrlDecode$("hello+world+%26+more") ' hello world & more
'
' =============================================================================

' ---------------------------------------------------------------------------
' Base64
' ---------------------------------------------------------------------------
DIM SHARED _B64_CHARS AS STRING
_B64_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"

FUNCTION Base64Encode$ (data$)
    DIM result AS STRING, i AS LONG, n AS LONG
    DIM b0 AS LONG, b1 AS LONG, b2 AS LONG
    DIM c0 AS LONG, c1 AS LONG, c2 AS LONG, c3 AS LONG
    n      = LEN(data$)
    result = ""
    i      = 1
    DO WHILE i <= n
        b0 = ASC(MID$(data$, i,     1))
        IF i + 1 <= n THEN b1 = ASC(MID$(data$, i + 1, 1)) ELSE b1 = 0
        IF i + 2 <= n THEN b2 = ASC(MID$(data$, i + 2, 1)) ELSE b2 = 0

        c0 = b0 \ 4
        c1 = ((b0 AND 3) * 16) OR (b1 \ 16)
        c2 = ((b1 AND 15) * 4) OR (b2 \ 64)
        c3 = b2 AND 63

        result = result + MID$(_B64_CHARS, c0 + 1, 1) + _
                          MID$(_B64_CHARS, c1 + 1, 1)
        IF i + 1 <= n THEN result = result + MID$(_B64_CHARS, c2 + 1, 1) ELSE result = result + "="
        IF i + 2 <= n THEN result = result + MID$(_B64_CHARS, c3 + 1, 1) ELSE result = result + "="
        i = i + 3
    LOOP
    Base64Encode$ = result
END FUNCTION

FUNCTION _B64_Index& (ch$)
    DIM i AS LONG
    i = INSTR(_B64_CHARS, ch$)
    IF i = 0 THEN _B64_Index& = -1 ELSE _B64_Index& = i - 1
END FUNCTION

FUNCTION Base64Decode$ (encoded$)
    DIM result AS STRING, i AS LONG, n AS LONG
    DIM c0 AS LONG, c1 AS LONG, c2 AS LONG, c3 AS LONG
    DIM b0 AS LONG, b1 AS LONG, b2 AS LONG
    n      = LEN(encoded$)
    result = ""
    i      = 1
    DO WHILE i <= n - 3
        c0 = _B64_Index&(MID$(encoded$, i,     1))
        c1 = _B64_Index&(MID$(encoded$, i + 1, 1))
        c2 = _B64_Index&(MID$(encoded$, i + 2, 1))
        c3 = _B64_Index&(MID$(encoded$, i + 3, 1))
        IF c0 < 0 OR c1 < 0 THEN i = i + 4: GOTO _b64_next

        b0 = (c0 * 4) OR (c1 \ 16)
        result = result + CHR$(b0)
        IF c2 >= 0 THEN
            b1 = ((c1 AND 15) * 16) OR (c2 \ 4)
            result = result + CHR$(b1)
        END IF
        IF c3 >= 0 THEN
            b2 = ((c2 AND 3) * 64) OR c3
            result = result + CHR$(b2)
        END IF
        _b64_next:
        i = i + 4
    LOOP
    Base64Decode$ = result
END FUNCTION

' ---------------------------------------------------------------------------
' URL Encoding (application/x-www-form-urlencoded)
' ---------------------------------------------------------------------------
FUNCTION UrlEncode$ (s$)
    DIM result AS STRING, i AS LONG, c AS INTEGER, ch AS STRING
    result = ""
    FOR i = 1 TO LEN(s$)
        ch = MID$(s$, i, 1)
        c  = ASC(ch)
        SELECT CASE c
            CASE 65 TO 90, 97 TO 122, 48 TO 57, 45, 95, 46, 126
                result = result + ch          ' unreserved: A-Z a-z 0-9 - _ . ~
            CASE 32
                result = result + "+"         ' space -> +
            CASE ELSE
                result = result + "%" + UCASE$(HEX$(c))
        END SELECT
    NEXT i
    UrlEncode$ = result
END FUNCTION

FUNCTION UrlDecode$ (s$)
    DIM result AS STRING, i AS LONG, ch AS STRING, hex AS STRING
    result = ""
    i = 1
    DO WHILE i <= LEN(s$)
        ch = MID$(s$, i, 1)
        SELECT CASE ch
            CASE "+"
                result = result + " "
                i = i + 1
            CASE "%"
                IF i + 2 <= LEN(s$) THEN
                    hex = MID$(s$, i + 1, 2)
                    result = result + CHR$(VAL("&H" + hex))
                    i = i + 3
                ELSE
                    result = result + ch
                    i = i + 1
                END IF
            CASE ELSE
                result = result + ch
                i = i + 1
        END SELECT
    LOOP
    UrlDecode$ = result
END FUNCTION

' ---------------------------------------------------------------------------
' HTML Entity Encoding (minimal — covers the 5 XML entities)
' ---------------------------------------------------------------------------
FUNCTION HtmlEncode$ (s$)
    DIM result AS STRING, i AS LONG, ch AS STRING
    result = ""
    FOR i = 1 TO LEN(s$)
        ch = MID$(s$, i, 1)
        SELECT CASE ch
            CASE "&":  result = result + "&amp;"
            CASE "<":  result = result + "&lt;"
            CASE ">":  result = result + "&gt;"
            CASE CHR$(34): result = result + "&quot;"
            CASE "'":  result = result + "&#39;"
            CASE ELSE: result = result + ch
        END SELECT
    NEXT i
    HtmlEncode$ = result
END FUNCTION

' ---------------------------------------------------------------------------
' Simple hex encoding / decoding
' ---------------------------------------------------------------------------
FUNCTION HexEncode$ (data$)
    DIM result AS STRING, i AS LONG, h AS STRING
    result = ""
    FOR i = 1 TO LEN(data$)
        h = HEX$(ASC(MID$(data$, i, 1)))
        IF LEN(h) = 1 THEN h = "0" + h
        result = result + h
    NEXT i
    HexEncode$ = result
END FUNCTION

FUNCTION HexDecode$ (hex$)
    DIM result AS STRING, i AS LONG
    result = ""
    FOR i = 1 TO LEN(hex$) - 1 STEP 2
        result = result + CHR$(VAL("&H" + MID$(hex$, i, 2)))
    NEXT i
    HexDecode$ = result
END FUNCTION
