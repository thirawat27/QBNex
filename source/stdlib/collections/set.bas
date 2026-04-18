' ============================================================================
' QBNex Standard Library - Collections: Set
' ============================================================================

$IMPORT:'collections.list'

TYPE QBNex_HashSet
    Items AS QBNex_List
END TYPE

SUB HashSet_Init (setRef AS QBNex_HashSet)
    List_Init setRef.Items
END SUB

SUB HashSet_Add (setRef AS QBNex_HashSet, item AS STRING)
    IF List_Contains&(setRef.Items, item) THEN EXIT SUB
    List_Add setRef.Items, item
END SUB

FUNCTION HashSet_Contains& (setRef AS QBNex_HashSet, item AS STRING)
    HashSet_Contains = List_Contains&(setRef.Items, item)
END FUNCTION

SUB HashSet_Remove (setRef AS QBNex_HashSet, item AS STRING)
    DIM index AS LONG

    index = List_IndexOf&(setRef.Items, item)
    IF index >= 0 THEN List_RemoveAt setRef.Items, index
END SUB

FUNCTION HashSet_Count& (setRef AS QBNex_HashSet)
    HashSet_Count = setRef.Items.Count
END FUNCTION

FUNCTION HashSet_ToString$ (setRef AS QBNex_HashSet, separator AS STRING)
    HashSet_ToString = List_Join$(setRef.Items, separator)
END FUNCTION

SUB HashSet_Clear (setRef AS QBNex_HashSet)
    List_Clear setRef.Items
END SUB

SUB HashSet_Free (setRef AS QBNex_HashSet)
    List_Free setRef.Items
END SUB
