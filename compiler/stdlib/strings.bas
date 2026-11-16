'===============================================================================
' QBNex Standard Library - strings ('$IMPORT:'strings')
'===============================================================================
' Text helpers like Python's str methods. Splitting into parts and joining
' them are List_Split / List_Join$ ('$IMPORT:'collections').
'
'   Str_Has&("hello", "ell")            -1        Python "ell" in s
'   Str_StartsWith&(s$, "he")           Str_EndsWith&(s$, "lo")
'   Str_Replace$("a-b-c", "-", "+")     "a+b+c"   every match
'   Str_Trim$(s$)                       spaces, tabs and line breaks off both ends
'   Str_PadLeft$("7", 3, "0")           "007"     Str_PadRight$ fills on the right
'   Str_Repeat$("ab", 3)                "ababab"
'===============================================================================

' -1 when text contains part ("" is in every text).
FUNCTION Str_Has& (text AS STRING, part AS STRING)
    Str_Has& = (INSTR(text, part) > 0 OR LEN(part) = 0)
END FUNCTION

' -1 when text starts with prefix.
FUNCTION Str_StartsWith& (text AS STRING, prefix AS STRING)
    Str_StartsWith& = (LEFT$(text, LEN(prefix)) = prefix)
END FUNCTION

' -1 when text ends with suffix.
FUNCTION Str_EndsWith& (text AS STRING, suffix AS STRING)
    Str_EndsWith& = (RIGHT$(text, LEN(suffix)) = suffix)
END FUNCTION

' text with every find replaced by replacement.
FUNCTION Str_Replace$ (text AS STRING, find AS STRING, replacement AS STRING)
    DIM p AS LONG, q AS LONG, r AS STRING
    IF LEN(find) = 0 THEN Str_Replace$ = text: EXIT FUNCTION
    p = 1
    DO
        q = INSTR(p, text, find)
        IF q = 0 THEN EXIT DO
        r = r + MID$(text, p, q - p) + replacement
        p = q + LEN(find)
    LOOP
    Str_Replace$ = r + MID$(text, p)
END FUNCTION

' text without spaces, tabs and line breaks at both ends (Python strip).
FUNCTION Str_Trim$ (text AS STRING)
    DIM a AS LONG, b AS LONG
    a = 1: b = LEN(text)
    DO WHILE a <= b
        IF INSTR(" " + CHR$(9) + CHR$(10) + CHR$(13), MID$(text, a, 1)) = 0 THEN EXIT DO
        a = a + 1
    LOOP
    DO WHILE b >= a
        IF INSTR(" " + CHR$(9) + CHR$(10) + CHR$(13), MID$(text, b, 1)) = 0 THEN EXIT DO
        b = b - 1
    LOOP
    Str_Trim$ = MID$(text, a, b - a + 1)
END FUNCTION

' text repeated count times.
FUNCTION Str_Repeat$ (text AS STRING, count AS LONG)
    DIM i AS LONG, r AS STRING
    FOR i = 1 TO count: r = r + text: NEXT
    Str_Repeat$ = r
END FUNCTION

' text filled on the left with fill up to size bytes (Python rjust).
FUNCTION Str_PadLeft$ (text AS STRING, size AS LONG, OPTIONAL fill AS STRING = " ")
    DIM n AS LONG
    n = size - LEN(text)
    IF n <= 0 OR LEN(fill) = 0 THEN Str_PadLeft$ = text: EXIT FUNCTION
    Str_PadLeft$ = LEFT$(Str_Repeat$(fill, n \ LEN(fill) + 1), n) + text
END FUNCTION

' text filled on the right with fill up to size bytes (Python ljust).
FUNCTION Str_PadRight$ (text AS STRING, size AS LONG, OPTIONAL fill AS STRING = " ")
    DIM n AS LONG
    n = size - LEN(text)
    IF n <= 0 OR LEN(fill) = 0 THEN Str_PadRight$ = text: EXIT FUNCTION
    Str_PadRight$ = text + LEFT$(Str_Repeat$(fill, n \ LEN(fill) + 1), n)
END FUNCTION
