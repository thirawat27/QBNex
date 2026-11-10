'===============================================================================
' QBNex Standard Library - Regular expressions ('$IMPORT:'ext.regex')
'===============================================================================
' Rust regex syntax (https://docs.rs/regex): \d \w \s, classes, (groups),
' (?<name>...), (?i) for case-insensitive, (?-u) to match raw bytes.
' Positions are 1-based like INSTR; 0 = no match. Invalid patterns return
' 0 / "" / -1 and set Ext_LastError$.
'
'   Regex_IsMatch&("^\d+$", s$)              -1 / 0
'   p& = Regex_Find&("(\w+)@(\w+)", s$, 1)   position; then Regex_Match$,
'   Regex_Group$(1), Regex_GroupCount&
'   Regex_Replace$("\s+", s$, " ")           every match ($1, ${name}, $$)
'   Regex_ReplaceFirst$(...)                 first match only
'   n& = Regex_Split&(",\s*", s$)            then Regex_Part$(0 .. n& - 1)
'===============================================================================

'$IMPORT:'ext.core'

DECLARE LIBRARY
    FUNCTION qb_ext_regex_match& (BYVAL p AS _OFFSET, BYVAL pl AS _INTEGER64, BYVAL t AS _OFFSET, BYVAL tl AS _INTEGER64)
    FUNCTION qb_ext_regex_find&& (BYVAL p AS _OFFSET, BYVAL pl AS _INTEGER64, BYVAL t AS _OFFSET, BYVAL tl AS _INTEGER64, BYVAL start AS _INTEGER64)
    FUNCTION qb_ext_regex_replace&& (BYVAL p AS _OFFSET, BYVAL pl AS _INTEGER64, BYVAL t AS _OFFSET, BYVAL tl AS _INTEGER64, BYVAL r AS _OFFSET, BYVAL rl AS _INTEGER64, BYVAL all AS LONG)
    FUNCTION qb_ext_regex_split& (BYVAL p AS _OFFSET, BYVAL pl AS _INTEGER64, BYVAL t AS _OFFSET, BYVAL tl AS _INTEGER64)
END DECLARE

DIM SHARED RegexLastMatch$

FUNCTION Regex_IsMatch& (pattern AS STRING, text AS STRING)
    IF qb_ext_regex_match&(_OFFSET(pattern), LEN(pattern), _OFFSET(text), LEN(text)) = 1 THEN Regex_IsMatch& = -1
END FUNCTION

FUNCTION Regex_Find& (pattern AS STRING, text AS STRING, start AS LONG)
    DIM r AS _INTEGER64
    r = qb_ext_regex_find&&(_OFFSET(pattern), LEN(pattern), _OFFSET(text), LEN(text), start)
    IF r > 0 THEN
        RegexLastMatch$ = Ext_Result$
        Regex_Find& = r
    ELSE
        RegexLastMatch$ = ""
    END IF
END FUNCTION

' Text of the last Regex_Find& match.
FUNCTION Regex_Match$ ()
    Regex_Match$ = RegexLastMatch$
END FUNCTION

' Capture group of the last Regex_Find& (0 = whole match).
FUNCTION Regex_Group$ (index AS LONG)
    Regex_Group$ = Ext_Part$(index)
END FUNCTION

' Number of groups of the last Regex_Find& including group 0 (0 after no match).
FUNCTION Regex_GroupCount& ()
    Regex_GroupCount& = Ext_PartCount&
END FUNCTION

FUNCTION Regex_Replace$ (pattern AS STRING, text AS STRING, replacement AS STRING)
    IF qb_ext_regex_replace&&(_OFFSET(pattern), LEN(pattern), _OFFSET(text), LEN(text), _OFFSET(replacement), LEN(replacement), 1) < 0 THEN EXIT FUNCTION
    Regex_Replace$ = Ext_Result$
END FUNCTION

FUNCTION Regex_ReplaceFirst$ (pattern AS STRING, text AS STRING, replacement AS STRING)
    IF qb_ext_regex_replace&&(_OFFSET(pattern), LEN(pattern), _OFFSET(text), LEN(text), _OFFSET(replacement), LEN(replacement), 0) < 0 THEN EXIT FUNCTION
    Regex_ReplaceFirst$ = Ext_Result$
END FUNCTION

' Splits at every match; returns the number of parts (-1 = bad pattern).
FUNCTION Regex_Split& (pattern AS STRING, text AS STRING)
    Regex_Split& = qb_ext_regex_split&(_OFFSET(pattern), LEN(pattern), _OFFSET(text), LEN(text))
END FUNCTION

FUNCTION Regex_Part$ (index AS LONG)
    Regex_Part$ = Ext_Part$(index)
END FUNCTION
