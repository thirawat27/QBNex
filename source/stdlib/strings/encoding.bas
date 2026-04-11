' ============================================================================
' QBNex Standard Library - Strings: Encoding
' ============================================================================
' Base64, URL, HTML, Hex encoding/decoding functions
' ============================================================================

CONST BASE64_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"

' ============================================================================
' FUNCTION: Base64Encode
' Encode string to Base64 (RFC 4648)
' ============================================================================
FUNCTION Base64Encode$ (text AS STRING)
    DIM result AS STRING
    DIM i AS LONG
    DIM b1 AS LONG, b2 AS LONG, b3 AS LONG
    DIM e1 AS LONG, e2 AS LONG, e3 AS LONG, e4 AS LONG
    DIM LEN AS LONG
    
    result = ""
    LEN = LEN(text)
    i = 1
    
    DO WHILE i <= LEN
        b1 = ASC(MID$(text, i, 1))
        IF i + 1 <= LEN THEN b2 = ASC(MID$(text, i + 1, 1)) ELSE b2 = 0
        IF i + 2 <= LEN THEN b3 = ASC(MID$(text, i + 2, 1)) ELSE b3 = 0
        
        e1 = b1 \ 4
        e2 = ((b1 AND 3) * 16) + (b2 \ 16)
        e3 = ((b2 AND 15) * 4) + (b3 \ 64)
        e4 = b3 AND 63
        
        result = result + MID$(BASE64_CHARS, e1 + 1, 1)
        result = result + MID$(BASE64_CHARS, e2 + 1, 1)
        
        IF i + 1 <= LEN THEN
            result = result + MID$(BASE64_CHARS, e3 + 1, 1)
        ELSE
            result = result + "="
        END IF
        
        IF i + 2 <= LEN THEN
            result = result + MID$(BASE64_CHARS, e4 + 1, 1)
        ELSE
            result = result + "="
        END IF
        
        i = i + 3
    LOOP
    
    Base64Encode = result
END FUNCTION

' ============================================================================
' FUNCTION: UrlEncode
' URL encode string (application/x-www-form-urlencoded)
' ============================================================================
FUNCTION UrlEncode$ (text AS STRING)
    DIM result AS STRING
    DIM i AS LONG
    DIM c AS STRING
    DIM code AS LONG
    
    result = ""
    FOR i = 1 TO LEN(text)
        c = MID$(text, i, 1)
        code = ASC(c)
        
        IF (code >= 48 AND code <= 57) OR _
        (code >= 65 AND code <= 90) OR _
        (code >= 97 AND code <= 122) OR _
        c = "-" OR c = "_" OR c = "." OR c = "~" THEN
        result = result + c
    ELSEIF c = " " THEN
        result = result + "+"
    ELSE
        result = result + "%" + RIGHT$("0" + HEX$(code), 2)
    END IF
NEXT i
    
UrlEncode = result
END FUNCTION

' ============================================================================
' FUNCTION: UrlDecode
' URL decode string
' ============================================================================
FUNCTION UrlDecode$ (text AS STRING)
    DIM result AS STRING
    DIM i AS LONG
    DIM c AS STRING
    DIM hexCode AS STRING
    
    result = ""
    i = 1
    
    DO WHILE i <= LEN(text)
        c = MID$(text, i, 1)
        
        IF c = "+" THEN
            result = result + " "
            i = i + 1
        ELSEIF c = "%" AND i + 2 <= LEN(text) THEN
            hexCode = MID$(text, i + 1, 2)
            result = result + CHR$(VAL("&H" + hexCode))
            i = i + 3
        ELSE
            result = result + c
            i = i + 1
        END IF
    LOOP
    
    UrlDecode = result
END FUNCTION

' ============================================================================
' FUNCTION: HtmlEncode
' HTML encode (5 XML entities)
' ============================================================================
FUNCTION HtmlEncode$ (text AS STRING)
    DIM result AS STRING
    DIM i AS LONG
    DIM c AS STRING
    
    result = ""
    FOR i = 1 TO LEN(text)
        c = MID$(text, i, 1)
        
        SELECT CASE c
        CASE "&": result = result + "&amp;"
        CASE "<": result = result + "&lt;"
        CASE ">": result = result + "&gt;"
        CASE CHR$(34): result = result + "&quot;"
        CASE "'": result = result + "&apos;"
        CASE ELSE: result = result + c
        END SELECT
    NEXT i
    
    HtmlEncode = result
END FUNCTION

' ============================================================================
' FUNCTION: HexEncode
' Encode bytes to hex string
' ============================================================================
FUNCTION HexEncode$ (text AS STRING)
    DIM result AS STRING
    DIM i AS LONG
    
    result = ""
    FOR i = 1 TO LEN(text)
        result = result + RIGHT$("0" + HEX$(ASC(MID$(text, i, 1))), 2)
    NEXT i
    
    HexEncode = result
END FUNCTION

' ============================================================================
' FUNCTION: HexDecode
' Decode hex string to bytes
' ============================================================================
FUNCTION HexDecode$ (hexText AS STRING)
    DIM result AS STRING
    DIM i AS LONG
    DIM hexPair AS STRING
    
    result = ""
    FOR i = 1 TO LEN(hexText) STEP 2
        hexPair = MID$(hexText, i, 2)
        result = result + CHR$(VAL("&H" + hexPair))
    NEXT i
    
    HexDecode = result
END FUNCTION
