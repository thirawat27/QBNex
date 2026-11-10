' ============================================================================
' QBNex Standard Library - Collections: Queue
' ============================================================================

'$IMPORT:'collections.list'

TYPE QBNex_Queue
    Items AS QBNex_List
END TYPE

SUB Queue_Init (queueRef AS QBNex_Queue)
    List_Init queueRef.Items
END SUB

SUB Queue_Enqueue (queueRef AS QBNex_Queue, item AS STRING)
    List_Add queueRef.Items, item
END SUB

FUNCTION Queue_Peek$ (queueRef AS QBNex_Queue)
    IF queueRef.Items.Count = 0 THEN
        Queue_Peek = ""
        EXIT FUNCTION
    END IF
    Queue_Peek = List_Get$(queueRef.Items, 0)
END FUNCTION

FUNCTION Queue_Dequeue$ (queueRef AS QBNex_Queue)
    DIM valueText AS STRING

    IF queueRef.Items.Count = 0 THEN
        Queue_Dequeue = ""
        EXIT FUNCTION
    END IF
    valueText = List_Get$(queueRef.Items, 0)
    List_RemoveAt queueRef.Items, 0
    Queue_Dequeue = valueText
END FUNCTION

FUNCTION Queue_Count& (queueRef AS QBNex_Queue)
    Queue_Count = queueRef.Items.Count
END FUNCTION

SUB Queue_Clear (queueRef AS QBNex_Queue)
    List_Clear queueRef.Items
END SUB

SUB Queue_Free (queueRef AS QBNex_Queue)
    List_Free queueRef.Items
END SUB
