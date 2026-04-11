' =============================================================================
' QBNex Collections — Stack (LIFO) — stack.bas
' =============================================================================
'
' Usage:
'
'   '$INCLUDE:'stdlib/collections/stack.bas'
'
'   DIM s AS QBNex_Stack
'   Stack_Init s
'
'   Stack_Push s, "first"
'   Stack_Push s, "second"
'   Stack_Push s, "third"
'
'   PRINT Stack_Peek$(s)        ' third  (no removal)
'   PRINT Stack_Pop$(s)         ' third  (removes it)
'   PRINT Stack_Count(s)        ' 2
'
'   Stack_Free s
'
' =============================================================================

'$INCLUDE:'stdlib/collections/list.bas'

' Stack is simply a QBNex_List with push/pop semantics
TYPE QBNex_Stack
    _list AS QBNex_List
END TYPE

SUB Stack_Init (s AS QBNex_Stack)
    List_Init s._list
END SUB

SUB Stack_Free (s AS QBNex_Stack)
    List_Free s._list
END SUB

FUNCTION Stack_Count& (s AS QBNex_Stack)
    Stack_Count& = List_Count&(s._list)
END FUNCTION

FUNCTION Stack_IsEmpty& (s AS QBNex_Stack)
    Stack_IsEmpty& = (List_Count&(s._list) = 0)
END FUNCTION

SUB Stack_Push (s AS QBNex_Stack, value$)
    List_Add s._list, value$
END SUB

SUB Stack_PushLong (s AS QBNex_Stack, value AS LONG)
    List_AddLong s._list, value
END SUB

FUNCTION Stack_Pop$ (s AS QBNex_Stack)
    DIM n AS LONG
    n = List_Count&(s._list)
    IF n = 0 THEN Stack_Pop$ = "": EXIT FUNCTION
    Stack_Pop$ = List_Get$(s._list, n)
    List_RemoveAt s._list, n
END FUNCTION

FUNCTION Stack_PopLong& (s AS QBNex_Stack)
    Stack_PopLong& = VAL(Stack_Pop$(s))
END FUNCTION

FUNCTION Stack_Peek$ (s AS QBNex_Stack)
    DIM n AS LONG
    n = List_Count&(s._list)
    IF n = 0 THEN Stack_Peek$ = "": EXIT FUNCTION
    Stack_Peek$ = List_Get$(s._list, n)
END FUNCTION

SUB Stack_Clear (s AS QBNex_Stack)
    List_Clear s._list
END SUB
