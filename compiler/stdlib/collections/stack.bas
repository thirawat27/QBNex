' ============================================================================
' QBNex Standard Library - Collections: Stack
' ============================================================================

$IMPORT:'collections.list'

TYPE QBNex_Stack
    Items AS QBNex_List
END TYPE

SUB Stack_Init (stackRef AS QBNex_Stack)
    List_Init stackRef.Items
END SUB

SUB Stack_Push (stackRef AS QBNex_Stack, item AS STRING)
    List_Add stackRef.Items, item
END SUB

FUNCTION Stack_Peek$ (stackRef AS QBNex_Stack)
    IF stackRef.Items.Count = 0 THEN
        Stack_Peek = ""
        EXIT FUNCTION
    END IF
    Stack_Peek = List_Get$(stackRef.Items, stackRef.Items.Count - 1)
END FUNCTION

FUNCTION Stack_Pop$ (stackRef AS QBNex_Stack)
    DIM valueText AS STRING

    IF stackRef.Items.Count = 0 THEN
        Stack_Pop = ""
        EXIT FUNCTION
    END IF
    valueText = List_Get$(stackRef.Items, stackRef.Items.Count - 1)
    List_RemoveAt stackRef.Items, stackRef.Items.Count - 1
    Stack_Pop = valueText
END FUNCTION

FUNCTION Stack_Count& (stackRef AS QBNex_Stack)
    Stack_Count = stackRef.Items.Count
END FUNCTION

SUB Stack_Clear (stackRef AS QBNex_Stack)
    List_Clear stackRef.Items
END SUB

SUB Stack_Free (stackRef AS QBNex_Stack)
    List_Free stackRef.Items
END SUB
