' ============================================================================
' QBNex Standard Library - Collections: Set (Unique Values)
' ============================================================================
' Set implementation with union, intersect operations
' Built on top of QBNex_List with uniqueness enforcement
' ============================================================================

'$INCLUDE:'list.bas'

TYPE QBNex_Set
    List AS QBNex_List
END TYPE

' ============================================================================
' SUB: Set_Init
' Initialize a new set
' ============================================================================
SUB Set_Init (s AS QBNex_Set)
    List_Init s.List
END SUB

' ============================================================================
' SUB: Set_Add
' Add an item to the set (only if not already present)
' ============================================================================
SUB Set_Add (s AS QBNex_Set, item AS STRING)
    IF NOT List_Contains(s.List, item) THEN
        List_Add s.List, item
    END IF
END SUB

' ============================================================================
' FUNCTION: Set_Contains
' Check if set contains an item
' ============================================================================
FUNCTION Set_Contains& (s AS QBNex_Set, item AS STRING)
    Set_Contains = List_Contains(s.List, item)
END FUNCTION

' ============================================================================
' SUB: Set_Remove
' Remove an item from the set
' ============================================================================
SUB Set_Remove (s AS QBNex_Set, item AS STRING)
    DIM idx AS LONG
    idx = List_IndexOf(s.List, item)
    IF idx >= 0 THEN
        List_RemoveAt s.List, idx
    END IF
END SUB

' ============================================================================
' FUNCTION: Set_Count
' Get number of items in set
' ============================================================================
FUNCTION Set_Count& (s AS QBNex_Set)
    Set_Count = s.List.Count
END FUNCTION

' ============================================================================
' SUB: Set_Union
' Create union of two sets (result = a ∪ b)
' ============================================================================
SUB Set_Union (result AS QBNex_Set, a AS QBNex_Set, b AS QBNex_Set)
    DIM i AS LONG
    
    Set_Init result
    
    ' Add all from a
    FOR i = 0 TO a.List.Count - 1
        Set_Add result, List_Get(a.List, i)
    NEXT i
    
    ' Add all from b (duplicates automatically filtered)
    FOR i = 0 TO b.List.Count - 1
        Set_Add result, List_Get(b.List, i)
    NEXT i
END SUB

' ============================================================================
' SUB: Set_Intersect
' Create intersection of two sets (result = a ∩ b)
' ============================================================================
SUB Set_Intersect (result AS QBNex_Set, a AS QBNex_Set, b AS QBNex_Set)
    DIM i AS LONG
    DIM item AS STRING
    
    Set_Init result
    
    ' Add items that exist in both sets
    FOR i = 0 TO a.List.Count - 1
        item = List_Get(a.List, i)
        IF Set_Contains(b, item) THEN
            Set_Add result, item
        END IF
    NEXT i
END SUB

' ============================================================================
' SUB: Set_Difference
' Create difference of two sets (result = a - b)
' ============================================================================
SUB Set_Difference (result AS QBNex_Set, a AS QBNex_Set, b AS QBNex_Set)
    DIM i AS LONG
    DIM item AS STRING
    
    Set_Init result
    
    ' Add items from a that are not in b
    FOR i = 0 TO a.List.Count - 1
        item = List_Get(a.List, i)
        IF NOT Set_Contains(b, item) THEN
            Set_Add result, item
        END IF
    NEXT i
END SUB

' ============================================================================
' SUB: Set_Clear
' Remove all items
' ============================================================================
SUB Set_Clear (s AS QBNex_Set)
    List_Clear s.List
END SUB

' ============================================================================
' SUB: Set_Free
' Free the set
' ============================================================================
SUB Set_Free (s AS QBNex_Set)
    List_Free s.List
END SUB
