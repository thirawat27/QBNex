' ============================================================================
' QBNex Standard Library - Collections: Queue (FIFO)
' ============================================================================
' Queue implementation built on top of QBNex_List
' ============================================================================

'$INCLUDE:'list.bas'

TYPE QBNex_Queue
    List AS QBNex_List
END TYPE

' ============================================================================
' SUB: Queue_Init
' Initialize a new queue
' ============================================================================
SUB Queue_Init (queue AS QBNex_Queue)
    List_Init queue.List
END SUB

' ============================================================================
' SUB: Queue_Enqueue
' Add an item to the end of the queue
' ============================================================================
SUB Queue_Enqueue (queue AS QBNex_Queue, item AS STRING)
    List_Add queue.List, item
END SUB

' ============================================================================
' FUNCTION: Queue_Dequeue
' Remove and return the front item
' ============================================================================
FUNCTION Queue_Dequeue$ (queue AS QBNex_Queue)
    DIM result AS STRING
    
    IF queue.List.Count = 0 THEN
        Queue_Dequeue = ""
        EXIT FUNCTION
    END IF
    
    result = List_Get(queue.List, 0)
    List_RemoveAt queue.List, 0
    Queue_Dequeue = result
END FUNCTION

' ============================================================================
' FUNCTION: Queue_Peek
' Return the front item without removing it
' ============================================================================
FUNCTION Queue_Peek$ (queue AS QBNex_Queue)
    IF queue.List.Count = 0 THEN
        Queue_Peek = ""
    ELSE
        Queue_Peek = List_Get(queue.List, 0)
    END IF
END FUNCTION

' ============================================================================
' FUNCTION: Queue_IsEmpty
' Check if queue is empty
' ============================================================================
FUNCTION Queue_IsEmpty& (queue AS QBNex_Queue)
    Queue_IsEmpty = (queue.List.Count = 0)
END FUNCTION

' ============================================================================
' FUNCTION: Queue_Count
' Get number of items in queue
' ============================================================================
FUNCTION Queue_Count& (queue AS QBNex_Queue)
    Queue_Count = queue.List.Count
END FUNCTION

' ============================================================================
' SUB: Queue_Clear
' Remove all items
' ============================================================================
SUB Queue_Clear (queue AS QBNex_Queue)
    List_Clear queue.List
END SUB

' ============================================================================
' SUB: Queue_Free
' Free the queue
' ============================================================================
SUB Queue_Free (queue AS QBNex_Queue)
    List_Free queue.List
END SUB
