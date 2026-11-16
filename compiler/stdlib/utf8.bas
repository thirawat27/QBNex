'===============================================================================
' QBNex Standard Library - utf8 ('$IMPORT:'utf8')
'===============================================================================
' BASIC strings are bytes; these treat them as UTF-8 text, by character.
' Bytes that are not valid UTF-8 count as one character each and are kept.
'
'   Utf8_Len("สวัสดี")        6 characters (LEN gives 18 bytes)
'   Utf8_Mid(s$, 2, 3)       like MID$; Utf8_Mid(s$, 2) to the end
'   Utf8_Left / Utf8_Right  Utf8_Upper / Utf8_Lower ("ß" -> "SS")
'   Utf8_Normalize(s$)       NFC; or "NFD", "NFKC", "NFKD"
'   Utf8_Asc(s$, i)          code point of character i (like ASC)
'   Utf8_Chr(3647)           character of a code point (like CHR$)
'   Utf8_IsValid(s$)         -1 for valid UTF-8
'   Utf8_FromCp437(s$)       DOS text to UTF-8; Utf8_FromCp437(s$, -1) also
'                             maps bytes 1-31 and 127 to the PC symbols
'   Utf8_ToCp437(s$)         back to DOS text; "?" where there is no glyph
'===============================================================================

'$IMPORT:'ext'

DECLARE LIBRARY
    FUNCTION qb_ext_utf8_len&& (BYVAL p AS _OFFSET, BYVAL l AS _INTEGER64)
    FUNCTION qb_ext_utf8_slice&& (BYVAL op AS LONG, BYVAL p AS _OFFSET, BYVAL l AS _INTEGER64, BYVAL a AS _INTEGER64, BYVAL b AS _INTEGER64)
    FUNCTION qb_ext_utf8_transform&& (BYVAL op AS LONG, BYVAL p AS _OFFSET, BYVAL l AS _INTEGER64)
    FUNCTION qb_ext_utf8_valid& (BYVAL p AS _OFFSET, BYVAL l AS _INTEGER64)
    FUNCTION qb_ext_utf8_codepoint&& (BYVAL p AS _OFFSET, BYVAL l AS _INTEGER64, BYVAL index AS _INTEGER64)
    FUNCTION qb_ext_utf8_chr&& (BYVAL cp AS _INTEGER64)
    FUNCTION qb_ext_cp437&& (BYVAL op AS LONG, BYVAL p AS _OFFSET, BYVAL l AS _INTEGER64)
END DECLARE

' Number of characters.
FUNCTION Utf8_Len& (text AS STRING)
    Utf8_Len& = qb_ext_utf8_len&&(_OFFSET(text), LEN(text))
END FUNCTION

' count characters from character start (1-based); count < 0 = to the end.
FUNCTION Utf8_Mid$ (text AS STRING, start AS LONG, OPTIONAL count AS LONG = -1)
    Utf8_Mid$ = Ext_Text$(qb_ext_utf8_slice&&(1, _OFFSET(text), LEN(text), start, count))
END FUNCTION

' The first count characters.
FUNCTION Utf8_Left$ (text AS STRING, count AS LONG)
    Utf8_Left$ = Ext_Text$(qb_ext_utf8_slice&&(2, _OFFSET(text), LEN(text), count, 0))
END FUNCTION

' The last count characters.
FUNCTION Utf8_Right$ (text AS STRING, count AS LONG)
    Utf8_Right$ = Ext_Text$(qb_ext_utf8_slice&&(3, _OFFSET(text), LEN(text), count, 0))
END FUNCTION

' Upper case (full Unicode case mapping).
FUNCTION Utf8_Upper$ (text AS STRING)
    Utf8_Upper$ = Ext_Text$(qb_ext_utf8_transform&&(1, _OFFSET(text), LEN(text)))
END FUNCTION

' Lower case (full Unicode case mapping).
FUNCTION Utf8_Lower$ (text AS STRING)
    Utf8_Lower$ = Ext_Text$(qb_ext_utf8_transform&&(2, _OFFSET(text), LEN(text)))
END FUNCTION

' Unicode normal form: "NFC" (default), "NFD", "NFKC" or "NFKD".
FUNCTION Utf8_Normalize$ (text AS STRING, OPTIONAL form AS STRING = "NFC")
    DIM op AS LONG
    SELECT CASE UCASE$(form)
        CASE "NFD": op = 12
        CASE "NFKC": op = 13
        CASE "NFKD": op = 14
        CASE ELSE: op = 11
    END SELECT
    Utf8_Normalize$ = Ext_Text$(qb_ext_utf8_transform&&(op, _OFFSET(text), LEN(text)))
END FUNCTION

' -1 when text is valid UTF-8.
FUNCTION Utf8_IsValid& (text AS STRING)
    Utf8_IsValid& = (qb_ext_utf8_valid&(_OFFSET(text), LEN(text)) = 1)
END FUNCTION

' Code point of character index (1-based); -1 when there is none.
FUNCTION Utf8_Asc& (text AS STRING, OPTIONAL index AS LONG = 1)
    Utf8_Asc& = qb_ext_utf8_codepoint&&(_OFFSET(text), LEN(text), index)
END FUNCTION

' The character of a code point; "" when it is not one.
FUNCTION Utf8_Chr$ (codePoint AS LONG)
    Utf8_Chr$ = Ext_Text$(qb_ext_utf8_chr&&(codePoint))
END FUNCTION

' DOS (CP437) text as UTF-8; graphics maps bytes 1-31 and 127 to their symbols.
FUNCTION Utf8_FromCp437$ (text AS STRING, OPTIONAL graphics AS LONG = 0)
    IF graphics THEN
        Utf8_FromCp437$ = Ext_Text$(qb_ext_cp437&&(2, _OFFSET(text), LEN(text)))
    ELSE
        Utf8_FromCp437$ = Ext_Text$(qb_ext_cp437&&(1, _OFFSET(text), LEN(text)))
    END IF
END FUNCTION

' UTF-8 text as DOS (CP437) bytes; "?" where CP437 has no glyph.
FUNCTION Utf8_ToCp437$ (text AS STRING)
    Utf8_ToCp437$ = Ext_Text$(qb_ext_cp437&&(3, _OFFSET(text), LEN(text)))
END FUNCTION
