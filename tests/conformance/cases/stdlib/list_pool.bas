$CONSOLE:ONLY
'$IMPORT:'qbnex'
' Lists have no fixed limit (there were 256 handles; a Dictionary takes two),
' items are read in constant time, and the count is the list's own.
DIM d(1 TO 300) AS QBNex_Dictionary
FOR i = 1 TO 300: Dict_Init d(i): Dict_Set d(i), "k", STR$(i): NEXT
PRINT Dict_Get$(d(300), "k", "-"); Dict_Count&(d(300))
FOR i = 1 TO 300: Dict_Free d(i): NEXT
DIM h AS QBNex_HashSet
HashSet_Init h
FOR i = 1 TO 3000: HashSet_Add h, STR$(i MOD 1500): NEXT
PRINT HashSet_Count&(h)
DIM l AS QBNex_List, c AS QBNex_List
List_Init l
FOR i = 1 TO 20000: List_Add l, STR$(i): NEXT
s& = 0: FOR i = 0 TO 19999: s& = s& + VAL(List_Get$(l, i)): NEXT
PRINT s&
c = l: List_Add l, "x": PRINT List_Count&(c); List_IndexOf&(l, "x")
List_RemoveAt l, 0: PRINT List_Get$(l, 0); List_Count&(l)
List_Set l, 0, CHR$(0) + "z": PRINT LEN(List_Get$(l, 0))
List_Free l: PRINT List_Count&(l); "["; List_Get$(l, 3); "]"
DIM q AS QBNex_Queue: Queue_Init q: Queue_Enqueue q, "a": Queue_Enqueue q, "b": PRINT Queue_Dequeue$(q); Queue_Count&(q)
DIM st AS QBNex_Stack: Stack_Init st: Stack_Push st, "a": Stack_Push st, "b": PRINT Stack_Pop$(st); Stack_Count&(st)
SYSTEM
