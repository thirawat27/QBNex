$CONSOLE:ONLY
'$IMPORT:'qbnex'
' List_Set / List_RemoveAt splice in place (a 2000-item queue drains at once),
' Date_FromUnixMs keeps milliseconds below 1000, Result_AddContext compares
' whole entries.
DIM q AS QBNex_Queue: Queue_Init q
FOR i& = 1 TO 2000: Queue_Enqueue q, "item" + STR$(i&): NEXT
FOR i& = 1 TO 2000: s$ = Queue_Dequeue$(q): NEXT
PRINT s$; Queue_Count&(q)
DIM l AS QBNex_List: List_Init l
List_Add l, "a": List_Add l, "bb": List_Add l, "ccc"
List_Set l, 1, "XYZW": List_RemoveAt l, 0
PRINT List_Get$(l, 0); "|"; List_Get$(l, 1); "|"; l.Count
List_Set l, 1, "": List_RemoveAt l, 1
PRINT List_Get$(l, 0); l.Count
DIM d AS QBNex_Date
Date_FromUnixMs d, 999.6#
PRINT Date_ToISOString$(d)
DIM r AS QBNex_Result
Result_FailWithContext r, 2, "boom", "saving report", "x"
Result_AddContext r, "port"
PRINT "["; Result_Context$(r); "]"
SYSTEM
