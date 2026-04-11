' ============================================================================
' QBNex Standard Library - Collections: Stack (LIFO)
' ============================================================================
' Stack implementation built on top of QBNex_List
' ============================================================================

'$INCLUDE:'list.bas'

TYPE QBNex_Stack
    List AS QBNex_List
END TYPE

' ============================================================================
' SUB: Stack_Init
' Initialize a new stack
' ============================================================================
SUB Stack_Init (stack AS QBNex_Stack)
    List_Init stack.List
END SUB

' ============================================================================
' SUB: Stack_Push
' Push an item onto the stack
' ============================================================================
SUB Stack_Push (stack AS QBNex_Stack, item AS STRING)
    List_Add stack.List, item
END SUB

' ============================================================================
' FUNCTION: Stack_Pop
' Pop and return the top item
' ============================================================================
FUNCTION Stack_Pop$ (stack AS QBNex_Stack)
    DIM result AS STRING
    
    IF stack.List.Count = 0 THEN
        Stack_Pop = ""
        EXIT FUNCTION
    END IF
    
    result = List_Get(stack.List, stack.List.Count - 1)
    List_RemoveAt stack.List, stack.List.Count - 1
    Stack_Pop = result
END FUNCTION

' ============================================================================
' FUNCTION: Stack_Peek
' Return the top item without removing it
' ============================================================================
FUNCTION Stack_Peek$ (stack AS QBNex_Stack)
    IF stack.List.Count = 0 THEN
        Stack_Peek = ""
    ELSE
        Stack_Peek = List_Get(stack.List, stack.List.Count - 1)
    END IF
END FUNCTION

' ============================================================================
' FUNCTION: Stack_IsEmpty
' Check if stack is empty
' ============================================================================
FUNCTION Stack_IsEmpty& (stack AS QBNex_Stack)
    Stack_IsEmpty = (stack.List.Count = 0)
END FUNCTION

' ============================================================================
' FUNCTION: Stack_Count
' Get number of items in stack
' ============================================================================
FUNCTION Stack_Count& (stack AS QBNex_Stack)
    Stack_Count = stack.List.Count
END FUNCTION

' ============================================================================
' SUB: Stack_Clear
' Remove all items
' ============================================================================
SUB Stack_Clear (stack AS QBNex_Stack)
    List_Clear stack.List
END SUB

' ============================================================================
' SUB: Stack_Free
' Free the stack
' ============================================================================
SUB Stack_Free (stack AS QBNex_Stack)
    List_Free stack.List
END SUB
