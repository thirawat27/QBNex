' ============================================================================
' QBNex Standard Library - Strings: StringBuilder
' ============================================================================
' Mutable string builder to avoid O(n²) concatenation
' Parts stored as a list; ToString joins once for O(n) performance
' ============================================================================

'$INCLUDE:'../collections/list.bas'

TYPE QBNex_StringBuilder
    Parts AS QBNex_List
END TYPE

' ============================================================================
' SUB: SB_Init
' Initialize a new StringBuilder
' ============================================================================
SUB SB_Init (sb AS QBNex_StringBuilder)
    List_Init sb.Parts
END SUB

' ============================================================================
' SUB: SB_Append
' Append a string
' ============================================================================
SUB SB_Append (sb AS QBNex_StringBuilder, text AS STRING)
    List_Add sb.Parts, text
END SUB

' ============================================================================
' SUB: SB_AppendLine
' Append a string with newline
' ============================================================================
SUB SB_AppendLine (sb AS QBNex_StringBuilder, text AS STRING)
    List_Add sb.Parts, text + CHR$(13) + CHR$(10)
END SUB

' ============================================================================
' SUB: SB_PrependLine
' Prepend a string with newline
' ============================================================================
SUB SB_PrependLine (sb AS QBNex_StringBuilder, text AS STRING)
    DIM temp AS QBNex_List
    DIM i AS LONG
    
    List_Init temp
    List_Add temp, text + CHR$(13) + CHR$(10)
    
    FOR i = 0 TO sb.Parts.Count - 1
        List_Add temp, List_Get(sb.Parts, i)
    NEXT i
    
    List_Free sb.Parts
    sb.Parts = temp
END SUB

' ============================================================================
' FUNCTION: SB_ToString
' Join all parts into a single string
' ============================================================================
FUNCTION SB_ToString$ (sb AS QBNex_StringBuilder)
    DIM result AS STRING
    DIM i AS LONG
    
    result = ""
    FOR i = 0 TO sb.Parts.Count - 1
        result = result + List_Get(sb.Parts, i)
    NEXT i
    
    SB_ToString = result
END FUNCTION

' ============================================================================
' SUB: SB_Clear
' Clear all parts
' ============================================================================
SUB SB_Clear (sb AS QBNex_StringBuilder)
    List_Clear sb.Parts
END SUB

' ============================================================================
' FUNCTION: SB_Length
' Get total length of all parts
' ============================================================================
FUNCTION SB_Length& (sb AS QBNex_StringBuilder)
    DIM total AS LONG
    DIM i AS LONG
    
    total = 0
    FOR i = 0 TO sb.Parts.Count - 1
        total = total + LEN(List_Get(sb.Parts, i))
    NEXT i
    
    SB_Length = total
END FUNCTION

' ============================================================================
' SUB: SB_Replace
' Replace all occurrences of a substring
' ============================================================================
SUB SB_Replace (sb AS QBNex_StringBuilder, oldText AS STRING, newText AS STRING)
    DIM assembled AS STRING
    DIM POS AS LONG
    DIM result AS STRING
    
    assembled = SB_ToString(sb)
    result = ""
    
    DO WHILE LEN(assembled) > 0
        POS = INSTR(assembled, oldText)
        IF POS = 0 THEN
            result = result + assembled
            EXIT DO
        END IF
        
        result = result + LEFT$(assembled, POS - 1) + newText
        assembled = MID$(assembled, POS + LEN(oldText))
    LOOP
    
    SB_Clear sb
    SB_Append sb, result
END SUB

' ============================================================================
' SUB: SB_Free
' Free the StringBuilder
' ============================================================================
SUB SB_Free (sb AS QBNex_StringBuilder)
    List_Free sb.Parts
END SUB
