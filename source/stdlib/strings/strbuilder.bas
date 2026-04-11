' =============================================================================
' QBNex String Library — String Builder — strbuilder.bas
' =============================================================================
'
' Avoids repeated string concatenation overhead by accumulating parts
' in a list and joining once with SB_ToString$().
'
' Usage:
'
'   '$INCLUDE:'stdlib/strings/strbuilder.bas'
'
'   DIM sb AS QBNex_StringBuilder
'   SB_Init sb
'
'   SB_Append sb, "Hello, "
'   SB_Append sb, "World"
'   SB_AppendLine sb, "!"
'   SB_AppendLong sb, 42
'
'   PRINT SB_ToString$(sb)    ' Hello, World!<newline>42
'   PRINT SB_Length&(sb)      ' total character count
'
'   SB_Clear sb
'   SB_Free sb
'
' =============================================================================

'$INCLUDE:'stdlib/collections/list.bas'

TYPE QBNex_StringBuilder
    _parts  AS QBNex_List
    _length AS LONG   ' cached total length
END TYPE

SUB SB_Init (sb AS QBNex_StringBuilder)
    List_Init sb._parts
    sb._length = 0
END SUB

SUB SB_Free (sb AS QBNex_StringBuilder)
    List_Free sb._parts
    sb._length = 0
END SUB

SUB SB_Clear (sb AS QBNex_StringBuilder)
    List_Clear sb._parts
    sb._length = 0
END SUB

SUB SB_Append (sb AS QBNex_StringBuilder, s$)
    IF LEN(s$) = 0 THEN EXIT SUB
    List_Add sb._parts, s$
    sb._length = sb._length + LEN(s$)
END SUB

SUB SB_AppendLine (sb AS QBNex_StringBuilder, s$)
    SB_Append sb, s$ + CHR$(13) + CHR$(10)
END SUB

SUB SB_AppendLong (sb AS QBNex_StringBuilder, value AS LONG)
    SB_Append sb, _TRIM$(STR$(value))
END SUB

SUB SB_AppendDouble (sb AS QBNex_StringBuilder, value AS DOUBLE)
    SB_Append sb, _TRIM$(STR$(value))
END SUB

SUB SB_AppendChar (sb AS QBNex_StringBuilder, ch AS LONG)
    SB_Append sb, CHR$(ch)
END SUB

SUB SB_PrependLine (sb AS QBNex_StringBuilder, s$)
    List_Insert sb._parts, 1, s$ + CHR$(13) + CHR$(10)
    sb._length = sb._length + LEN(s$) + 2
END SUB

FUNCTION SB_Length& (sb AS QBNex_StringBuilder)
    SB_Length& = sb._length
END FUNCTION

FUNCTION SB_ToString$ (sb AS QBNex_StringBuilder)
    DIM i AS LONG, result AS STRING
    result = ""
    FOR i = 1 TO List_Count&(sb._parts)
        result = result + List_Get$(sb._parts, i)
    NEXT i
    SB_ToString$ = result
END FUNCTION

' Replace all occurrences of find$ with replace$ within the builder
SUB SB_Replace (sb AS QBNex_StringBuilder, find$, replace$)
    DIM full AS STRING, a AS STRING, p AS LONG, base AS LONG
    IF LEN(find$) = 0 THEN EXIT SUB
    full  = SB_ToString$(sb)
    a     = full
    base  = 1
    p     = INSTR(base, a, find$)
    DO WHILE p > 0
        a    = LEFT$(a, p - 1) + replace$ + MID$(a, p + LEN(find$))
        base = p + LEN(replace$)
        IF base > LEN(a) THEN EXIT DO
        p    = INSTR(base, a, find$)
    LOOP
    SB_Clear sb
    SB_Append sb, a
END SUB
