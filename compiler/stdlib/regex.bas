'===============================================================================
' QBNex Standard Library - regex ('$IMPORT:'regex')
'===============================================================================
' Regular expressions, like Python's re. Rust regex syntax
' (https://docs.rs/regex): \d \w \s, classes, (groups), (?<name>...), (?i) for
' case-insensitive. No backreferences or look-around. Positions are 1-based
' bytes like INSTR. A bad pattern gives 0 / "" and LastError$ says why.
'
'   Regex_Test&("^\d+$", s$)                -1 when s$ matches
'   m$ = Regex_Find$("(\w+)@(\w+)", s$)     the first match ("" if none);
'        Regex_Group$(1), Regex_Pos& of that match
'   Regex_Replace$("\s+", s$, " ")          every match ($1, ${name}, $$)
'   Regex_Replace$("\s+", s$, " ", -1)      the first match only
'   Regex_Split ",\s*", s$, parts           parts (a List) between matches
'   Regex_FindAll "\d+", s$, found          every match in a List
'===============================================================================

'$IMPORT:'ext'
'$IMPORT:'collections'

DECLARE LIBRARY
    FUNCTION qb_ext_regex_match& (BYVAL p AS _OFFSET, BYVAL pl AS _INTEGER64, BYVAL t AS _OFFSET, BYVAL tl AS _INTEGER64)
    FUNCTION qb_ext_regex_find&& (BYVAL p AS _OFFSET, BYVAL pl AS _INTEGER64, BYVAL t AS _OFFSET, BYVAL tl AS _INTEGER64, BYVAL start AS _INTEGER64)
    FUNCTION qb_ext_regex_replace&& (BYVAL p AS _OFFSET, BYVAL pl AS _INTEGER64, BYVAL t AS _OFFSET, BYVAL tl AS _INTEGER64, BYVAL r AS _OFFSET, BYVAL rl AS _INTEGER64, BYVAL all AS LONG)
    FUNCTION qb_ext_regex_split& (BYVAL p AS _OFFSET, BYVAL pl AS _INTEGER64, BYVAL t AS _OFFSET, BYVAL tl AS _INTEGER64)
END DECLARE

DIM SHARED Regex_LastPos AS LONG
DIM SHARED Regex_Groups AS List

' -1 when text has a match of pattern.
FUNCTION Regex_Test& (pattern AS STRING, text AS STRING)
    DIM r AS LONG
    r = qb_ext_regex_match&(_OFFSET(pattern), LEN(pattern), _OFFSET(text), LEN(text))
    Ext_Sync
    Regex_Test& = (r = 1)
END FUNCTION

' The first match of pattern in text at or after byte start; "" when none.
' Regex_Pos& and Regex_Group$ then describe it.
FUNCTION Regex_Find$ (pattern AS STRING, text AS STRING, OPTIONAL start AS LONG = 1)
    DIM r AS _INTEGER64, i AS LONG, m AS STRING
    r = qb_ext_regex_find&&(_OFFSET(pattern), LEN(pattern), _OFFSET(text), LEN(text), start)
    Ext_Sync
    List_Clear Regex_Groups
    Regex_LastPos = 0
    IF r <= 0 THEN EXIT FUNCTION
    m = Ext_Result$
    Regex_LastPos = r
    FOR i = 0 TO Ext_PartCount& - 1
        List_Add Regex_Groups, Ext_Part$(i)
    NEXT
    Regex_Find$ = m
END FUNCTION

' Byte position of the last Regex_Find$ match (0 = no match).
FUNCTION Regex_Pos& ()
    Regex_Pos& = Regex_LastPos
END FUNCTION

' Group index of the last Regex_Find$ match (0 = the whole match).
FUNCTION Regex_Group$ (index AS LONG)
    Regex_Group$ = List_Get$(Regex_Groups, index)
END FUNCTION

' text with the matches of pattern replaced (only the first when firstOnly).
FUNCTION Regex_Replace$ (pattern AS STRING, text AS STRING, replacement AS STRING, OPTIONAL firstOnly AS LONG = 0)
    DIM r AS _INTEGER64, all AS LONG
    IF firstOnly = 0 THEN all = 1
    r = qb_ext_regex_replace&&(_OFFSET(pattern), LEN(pattern), _OFFSET(text), LEN(text), _OFFSET(replacement), LEN(replacement), all)
    Ext_Sync
    IF r >= 0 THEN Regex_Replace$ = Ext_Result$
END FUNCTION

' Replaces the items of parts with the pieces of text between matches.
SUB Regex_Split (pattern AS STRING, text AS STRING, parts AS List)
    DIM n AS LONG, i AS LONG
    n = qb_ext_regex_split&(_OFFSET(pattern), LEN(pattern), _OFFSET(text), LEN(text))
    Ext_Sync
    List_Clear parts
    FOR i = 0 TO n - 1
        List_Add parts, Ext_Part$(i)
    NEXT
END SUB

' Replaces the items of found with every match of pattern in text.
SUB Regex_FindAll (pattern AS STRING, text AS STRING, found AS List)
    DIM p AS LONG, m AS STRING
    List_Clear found
    p = 1
    DO WHILE p <= LEN(text) + 1
        m = Regex_Find$(pattern, text, p)
        IF Regex_LastPos = 0 THEN EXIT DO
        List_Add found, m
        IF LEN(m) THEN p = Regex_LastPos + LEN(m) ELSE p = Regex_LastPos + 1
    LOOP
END SUB
