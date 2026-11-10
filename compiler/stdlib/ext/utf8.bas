'===============================================================================
' QBNex Standard Library - UTF-8 strings ('$IMPORT:'ext.utf8')
'===============================================================================
' BASIC strings are bytes; these helpers treat them as UTF-8 text. Bytes that
' are not valid UTF-8 count as one character each and are kept unchanged.
'
'   Utf8_Len&(s$)                 characters (LEN$ counts bytes)
'   Utf8_Mid$(s$, start&, count&) count& < 0 = to the end; 1-based like MID$
'   Utf8_Left$ / Utf8_Right$      by character
'   Utf8_Upper$ / Utf8_Lower$     full Unicode case mapping ("ß" -> "SS")
'   Utf8_Normalize$(s$, "NFC")    NFC, NFD, NFKC or NFKD
'   Utf8_IsValid&(s$)             -1 if s$ is valid UTF-8
'   Utf8_CodePoint&(s$, index&)   Unicode value of a character (-1 if none)
'   Utf8_Chr$(cp&)                character for a code point ("" if invalid)
'   Cp437_ToUtf8$(s$)             DOS text to UTF-8 (control bytes kept)
'   Cp437_ToUtf8Graphics$(s$)     bytes 1-31 and 127 as the PC symbols
'   Utf8_ToCp437$(s$)             back to CP437; "?" where there is no glyph
'===============================================================================

'$IMPORT:'ext.core'

DECLARE LIBRARY
    FUNCTION qb_ext_utf8_len&& (BYVAL p AS _OFFSET, BYVAL l AS _INTEGER64)
    FUNCTION qb_ext_utf8_slice&& (BYVAL op AS LONG, BYVAL p AS _OFFSET, BYVAL l AS _INTEGER64, BYVAL a AS _INTEGER64, BYVAL b AS _INTEGER64)
    FUNCTION qb_ext_utf8_transform&& (BYVAL op AS LONG, BYVAL p AS _OFFSET, BYVAL l AS _INTEGER64)
    FUNCTION qb_ext_utf8_valid& (BYVAL p AS _OFFSET, BYVAL l AS _INTEGER64)
    FUNCTION qb_ext_utf8_codepoint&& (BYVAL p AS _OFFSET, BYVAL l AS _INTEGER64, BYVAL index AS _INTEGER64)
    FUNCTION qb_ext_utf8_chr&& (BYVAL cp AS _INTEGER64)
    FUNCTION qb_ext_cp437&& (BYVAL op AS LONG, BYVAL p AS _OFFSET, BYVAL l AS _INTEGER64)
END DECLARE

FUNCTION Utf8_Len& (text AS STRING)
    Utf8_Len& = qb_ext_utf8_len&&(_OFFSET(text), LEN(text))
END FUNCTION

FUNCTION Utf8_Mid$ (text AS STRING, start AS LONG, count AS LONG)
    IF qb_ext_utf8_slice&&(1, _OFFSET(text), LEN(text), start, count) > 0 THEN Utf8_Mid$ = Ext_Result$
END FUNCTION

FUNCTION Utf8_Left$ (text AS STRING, count AS LONG)
    IF qb_ext_utf8_slice&&(2, _OFFSET(text), LEN(text), count, 0) > 0 THEN Utf8_Left$ = Ext_Result$
END FUNCTION

FUNCTION Utf8_Right$ (text AS STRING, count AS LONG)
    IF qb_ext_utf8_slice&&(3, _OFFSET(text), LEN(text), count, 0) > 0 THEN Utf8_Right$ = Ext_Result$
END FUNCTION

FUNCTION Utf8_Upper$ (text AS STRING)
    IF qb_ext_utf8_transform&&(1, _OFFSET(text), LEN(text)) > 0 THEN Utf8_Upper$ = Ext_Result$
END FUNCTION

FUNCTION Utf8_Lower$ (text AS STRING)
    IF qb_ext_utf8_transform&&(2, _OFFSET(text), LEN(text)) > 0 THEN Utf8_Lower$ = Ext_Result$
END FUNCTION

' form$: "NFC" (default), "NFD", "NFKC" or "NFKD".
FUNCTION Utf8_Normalize$ (text AS STRING, form AS STRING)
    DIM op AS LONG
    SELECT CASE UCASE$(form)
        CASE "NFD": op = 12
        CASE "NFKC": op = 13
        CASE "NFKD": op = 14
        CASE ELSE: op = 11
    END SELECT
    IF qb_ext_utf8_transform&&(op, _OFFSET(text), LEN(text)) > 0 THEN Utf8_Normalize$ = Ext_Result$
END FUNCTION

FUNCTION Utf8_IsValid& (text AS STRING)
    IF qb_ext_utf8_valid&(_OFFSET(text), LEN(text)) = 1 THEN Utf8_IsValid& = -1
END FUNCTION

FUNCTION Utf8_CodePoint& (text AS STRING, index AS LONG)
    Utf8_CodePoint& = qb_ext_utf8_codepoint&&(_OFFSET(text), LEN(text), index)
END FUNCTION

FUNCTION Utf8_Chr$ (codePoint AS LONG)
    IF qb_ext_utf8_chr&&(codePoint) > 0 THEN Utf8_Chr$ = Ext_Result$
END FUNCTION

FUNCTION Cp437_ToUtf8$ (text AS STRING)
    IF qb_ext_cp437&&(1, _OFFSET(text), LEN(text)) > 0 THEN Cp437_ToUtf8$ = Ext_Result$
END FUNCTION

FUNCTION Cp437_ToUtf8Graphics$ (text AS STRING)
    IF qb_ext_cp437&&(2, _OFFSET(text), LEN(text)) > 0 THEN Cp437_ToUtf8Graphics$ = Ext_Result$
END FUNCTION

FUNCTION Utf8_ToCp437$ (text AS STRING)
    IF qb_ext_cp437&&(3, _OFFSET(text), LEN(text)) > 0 THEN Utf8_ToCp437$ = Ext_Result$
END FUNCTION
