' =============================================================================
' QBNex Collections — Unique-Value Set — set.bas
' =============================================================================
'
' Usage:
'
'   '$INCLUDE:'stdlib/collections/set.bas'
'
'   DIM fruits AS QBNex_Set
'   Set_Init fruits
'
'   Set_Add fruits, "apple"
'   Set_Add fruits, "banana"
'   Set_Add fruits, "apple"   ' duplicate — ignored
'
'   PRINT Set_Count(fruits)                ' 2
'   PRINT Set_Has(fruits, "banana")        ' -1 (TRUE)
'   PRINT Set_Has(fruits, "grape")         ' 0  (FALSE)
'
'   Set_Remove fruits, "banana"
'   Set_Items fruits, myList              ' get all as a List
'
'   Set_Free fruits
'
' =============================================================================

'$INCLUDE:'stdlib/collections/list.bas'

TYPE QBNex_Set
    _list AS QBNex_List
END TYPE

SUB Set_Init (s AS QBNex_Set)
    List_Init s._list
END SUB

SUB Set_Free (s AS QBNex_Set)
    List_Free s._list
END SUB

FUNCTION Set_Count& (s AS QBNex_Set)
    Set_Count& = List_Count&(s._list)
END FUNCTION

FUNCTION Set_Has& (s AS QBNex_Set, value$)
    Set_Has& = List_Contains&(s._list, value$)
END FUNCTION

' Adds value only if not already present; returns -1 if added, 0 if duplicate
FUNCTION Set_Add& (s AS QBNex_Set, value$)
    IF List_Contains&(s._list, value$) THEN
        Set_Add& = 0
    ELSE
        List_Add s._list, value$
        Set_Add& = -1
    END IF
END FUNCTION

SUB Set_Remove (s AS QBNex_Set, value$)
    DIM idx AS LONG
    idx = List_IndexOf&(s._list, value$)
    IF idx > 0 THEN List_RemoveAt s._list, idx
END SUB

SUB Set_Clear (s AS QBNex_Set)
    List_Clear s._list
END SUB

' Copy all elements into a List
SUB Set_Items (s AS QBNex_Set, lst AS QBNex_List)
    DIM i AS LONG
    List_Clear lst
    FOR i = 1 TO List_Count&(s._list)
        List_Add lst, List_Get$(s._list, i)
    NEXT i
END SUB

' Union: adds all items from srcSet into destSet
SUB Set_Union (dest AS QBNex_Set, src AS QBNex_Set)
    DIM i AS LONG
    FOR i = 1 TO List_Count&(src._list)
        Set_Add& dest, List_Get$(src._list, i)
    NEXT i
END SUB

' Intersection: keeps only items that are also in otherSet
SUB Set_Intersect (dest AS QBNex_Set, other AS QBNex_Set)
    DIM i AS LONG, n AS LONG, val AS STRING
    n = List_Count&(dest._list)
    FOR i = n TO 1 STEP -1
        val = List_Get$(dest._list, i)
        IF NOT Set_Has&(other, val) THEN
            List_RemoveAt dest._list, i
        END IF
    NEXT i
END SUB
