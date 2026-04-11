' =============================================================================
' QBNex Collections — Queue (FIFO) — queue.bas
' =============================================================================
'
' Usage:
'
'   '$INCLUDE:'stdlib/collections/queue.bas'
'
'   DIM q AS QBNex_Queue
'   Queue_Init q
'
'   Queue_Enqueue q, "task1"
'   Queue_Enqueue q, "task2"
'   Queue_Enqueue q, "task3"
'
'   PRINT Queue_Peek$(q)        ' task1
'   PRINT Queue_Dequeue$(q)     ' task1  (removes it)
'   PRINT Queue_Count(q)        ' 2
'
'   Queue_Free q
'
' =============================================================================

'$INCLUDE:'stdlib/collections/list.bas'

TYPE QBNex_Queue
    _list AS QBNex_List
END TYPE

SUB Queue_Init (q AS QBNex_Queue)
    List_Init q._list
END SUB

SUB Queue_Free (q AS QBNex_Queue)
    List_Free q._list
END SUB

FUNCTION Queue_Count& (q AS QBNex_Queue)
    Queue_Count& = List_Count&(q._list)
END FUNCTION

FUNCTION Queue_IsEmpty& (q AS QBNex_Queue)
    Queue_IsEmpty& = (List_Count&(q._list) = 0)
END FUNCTION

SUB Queue_Enqueue (q AS QBNex_Queue, value$)
    List_Add q._list, value$
END SUB

SUB Queue_EnqueueLong (q AS QBNex_Queue, value AS LONG)
    List_AddLong q._list, value
END SUB

FUNCTION Queue_Dequeue$ (q AS QBNex_Queue)
    IF List_Count&(q._list) = 0 THEN Queue_Dequeue$ = "": EXIT FUNCTION
    Queue_Dequeue$ = List_Get$(q._list, 1)
    List_RemoveAt q._list, 1
END FUNCTION

FUNCTION Queue_DequeueLong& (q AS QBNex_Queue)
    Queue_DequeueLong& = VAL(Queue_Dequeue$(q))
END FUNCTION

FUNCTION Queue_Peek$ (q AS QBNex_Queue)
    Queue_Peek$ = List_Get$(q._list, 1)
END FUNCTION

SUB Queue_Clear (q AS QBNex_Queue)
    List_Clear q._list
END SUB
