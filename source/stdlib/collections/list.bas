' =============================================================================
' QBNex Collections — Dynamic List (ArrayList semantics) — list.bas
' =============================================================================
'
' A resizable ordered list backed by a STRING buffer (serialised storage).
' Elements can be any scalar type or fixed-length string.
'
' Syntax example:
'
'   '$INCLUDE:'stdlib/collections/list.bas'
'
'   DIM myList AS QBNex_List
'   List_Init myList
'
'   List_Add myList, "Apple"
'   List_Add myList, "Banana"
'   List_Add myList, "Cherry"
'
'   PRINT "Count: "; List_Count(myList)          ' 3
'   PRINT "Item 1: "; List_Get$(myList, 1)       ' Apple
'
'   List_RemoveAt myList, 1                       ' removes "Apple"
'   PRINT "After remove, Item 1: "; List_Get$(myList, 1)  ' Banana
'
'   List_Insert myList, 1, "Apricot"
'   PRINT "After insert, Item 1: "; List_Get$(myList, 1)  ' Apricot
'
'   List_Sort myList                              ' alphabetical sort
'
'   List_Free myList                              ' release resources
'
' =============================================================================
'
' STORAGE FORMAT (internal):
'   Items are stored in a dynamic STRING array (List._data).
'   The List TYPE holds count and capacity metadata.
'   Numeric values are stored as their STR$() representation.
'
' =============================================================================

CONST QBNEX_LIST_INITIAL_CAP = 16  ' initial allocation size

' ---------------------------------------------------------------------------
' Core TYPE
' ---------------------------------------------------------------------------
TYPE QBNex_List
    Count    AS LONG     ' number of active elements
    Capacity AS LONG     ' allocated size of _data array
    _handle  AS LONG     ' index into global list-pool (internal)
END TYPE

' ---------------------------------------------------------------------------
' Global storage pool — allows arrays-of-lists without REDIM inside TYPE
' ---------------------------------------------------------------------------
CONST QBNEX_LIST_POOL_MAX = 4096

DIM SHARED QBNEX_List_PoolUsed(1 TO QBNEX_LIST_POOL_MAX) AS LONG
DIM SHARED QBNEX_List_PoolData(1 TO QBNEX_LIST_POOL_MAX) AS STRING ' delimiter-separated
DIM SHARED QBNEX_List_PoolCount AS LONG
DIM SHARED QBNEX_List_PoolFreeList(1 TO QBNEX_LIST_POOL_MAX) AS LONG
DIM SHARED QBNEX_List_PoolFreeCount AS LONG
QBNEX_List_PoolCount    = 0
QBNEX_List_PoolFreeCount = 0

' item separator (chr 1, unlikely in data)
CONST QBNEX_LIST_SEP = 1

' ---------------------------------------------------------------------------
' PRIVATE: allocate a pool slot, returns handle
' ---------------------------------------------------------------------------
FUNCTION _List_AllocPool& ()
    DIM h AS LONG
    IF QBNEX_List_PoolFreeCount > 0 THEN
        h = QBNEX_List_PoolFreeList(QBNEX_List_PoolFreeCount)
        QBNEX_List_PoolFreeCount = QBNEX_List_PoolFreeCount - 1
    ELSE
        QBNEX_List_PoolCount = QBNEX_List_PoolCount + 1
        IF QBNEX_List_PoolCount > QBNEX_LIST_POOL_MAX THEN
            PRINT "QBNex List Error: pool overflow (max " + STR$(QBNEX_LIST_POOL_MAX) + " lists)"
            END 1
        END IF
        h = QBNEX_List_PoolCount
    END IF
    QBNEX_List_PoolUsed(h) = -1
    QBNEX_List_PoolData(h) = ""
    _List_AllocPool& = h
END FUNCTION

' ---------------------------------------------------------------------------
' SUB  List_Init(lst)         — initialise a new list
' ---------------------------------------------------------------------------
SUB List_Init (lst AS QBNex_List)
    lst.Count    = 0
    lst.Capacity = QBNEX_LIST_INITIAL_CAP
    lst._handle  = _List_AllocPool&()
END SUB

' ---------------------------------------------------------------------------
' SUB  List_Free(lst)         — release resources
' ---------------------------------------------------------------------------
SUB List_Free (lst AS QBNex_List)
    IF lst._handle < 1 THEN EXIT SUB
    QBNEX_List_PoolUsed(lst._handle)  = 0
    QBNEX_List_PoolData(lst._handle)  = ""
    QBNEX_List_PoolFreeCount = QBNEX_List_PoolFreeCount + 1
    QBNEX_List_PoolFreeList(QBNEX_List_PoolFreeCount) = lst._handle
    lst._handle  = 0
    lst.Count    = 0
    lst.Capacity = 0
END SUB

' ---------------------------------------------------------------------------
' FUNCTION  List_Count&(lst)  — number of items
' ---------------------------------------------------------------------------
FUNCTION List_Count& (lst AS QBNex_List)
    List_Count& = lst.Count
END FUNCTION

' ---------------------------------------------------------------------------
' PRIVATE: get raw item string by 1-based index from pool data
' ---------------------------------------------------------------------------
FUNCTION _List_GetRaw$ (lst AS QBNex_List, idx AS LONG)
    DIM buf AS STRING, p AS LONG, i AS LONG, s AS LONG
    IF idx < 1 OR idx > lst.Count THEN _List_GetRaw$ = "": EXIT FUNCTION
    buf = QBNEX_List_PoolData(lst._handle)
    s = 1: i = 0
    DO
        p = INSTR(s, buf, CHR$(QBNEX_LIST_SEP))
        i = i + 1
        IF i = idx THEN
            IF p = 0 THEN
                _List_GetRaw$ = MID$(buf, s)
            ELSE
                _List_GetRaw$ = MID$(buf, s, p - s)
            END IF
            EXIT FUNCTION
        END IF
        IF p = 0 THEN EXIT DO
        s = p + 1
    LOOP
    _List_GetRaw$ = ""
END FUNCTION

' ---------------------------------------------------------------------------
' PRIVATE: rebuild pool data from a temp array (after mutations)
' ---------------------------------------------------------------------------
SUB _List_Rebuild (lst AS QBNex_List, items() AS STRING, n AS LONG)
    DIM i AS LONG, buf AS STRING
    buf = ""
    FOR i = 1 TO n
        IF i > 1 THEN buf = buf + CHR$(QBNEX_LIST_SEP)
        buf = buf + items(i)
    NEXT i
    QBNEX_List_PoolData(lst._handle) = buf
    lst.Count = n
END SUB

' ---------------------------------------------------------------------------
' PRIVATE: extract all items into a temp array
' ---------------------------------------------------------------------------
SUB _List_Explode (lst AS QBNex_List, items() AS STRING)
    DIM buf AS STRING, s AS LONG, p AS LONG, i AS LONG
    REDIM items(1 TO lst.Count + 1) AS STRING
    IF lst.Count = 0 THEN EXIT SUB
    buf = QBNEX_List_PoolData(lst._handle)
    s = 1: i = 0
    DO
        p = INSTR(s, buf, CHR$(QBNEX_LIST_SEP))
        i = i + 1
        IF p = 0 THEN items(i) = MID$(buf, s): EXIT DO
        items(i) = MID$(buf, s, p - s)
        s = p + 1
    LOOP
END SUB

' ---------------------------------------------------------------------------
' SUB  List_Add(lst, value$)  — append an item
' ---------------------------------------------------------------------------
SUB List_Add (lst AS QBNex_List, value$)
    IF lst._handle < 1 THEN List_Init lst
    IF lst.Count > 0 THEN
        QBNEX_List_PoolData(lst._handle) = QBNEX_List_PoolData(lst._handle) + _
                                           CHR$(QBNEX_LIST_SEP) + value$
    ELSE
        QBNEX_List_PoolData(lst._handle) = value$
    END IF
    lst.Count = lst.Count + 1
END SUB

' ---------------------------------------------------------------------------
' SUB  List_AddLong(lst, value&)   — append numeric item
' ---------------------------------------------------------------------------
SUB List_AddLong (lst AS QBNex_List, value AS LONG)
    List_Add lst, _TRIM$(STR$(value))
END SUB

' ---------------------------------------------------------------------------
' SUB  List_AddDouble(lst, value#)
' ---------------------------------------------------------------------------
SUB List_AddDouble (lst AS QBNex_List, value AS DOUBLE)
    List_Add lst, _TRIM$(STR$(value))
END SUB

' ---------------------------------------------------------------------------
' FUNCTION  List_Get$(lst, index&)  — get item by 1-based index
' ---------------------------------------------------------------------------
FUNCTION List_Get$ (lst AS QBNex_List, idx AS LONG)
    List_Get$ = _List_GetRaw$(lst, idx)
END FUNCTION

FUNCTION List_GetLong& (lst AS QBNex_List, idx AS LONG)
    List_GetLong& = VAL(_List_GetRaw$(lst, idx))
END FUNCTION

FUNCTION List_GetDouble# (lst AS QBNex_List, idx AS LONG)
    List_GetDouble# = VAL(_List_GetRaw$(lst, idx))
END FUNCTION

' ---------------------------------------------------------------------------
' SUB  List_Set(lst, index&, value$)  — replace item at index
' ---------------------------------------------------------------------------
SUB List_Set (lst AS QBNex_List, idx AS LONG, value$)
    DIM items() AS STRING
    _List_Explode lst, items()
    IF idx >= 1 AND idx <= lst.Count THEN items(idx) = value$
    _List_Rebuild lst, items(), lst.Count
END SUB

' ---------------------------------------------------------------------------
' SUB  List_Insert(lst, index&, value$)  — insert before index
' ---------------------------------------------------------------------------
SUB List_Insert (lst AS QBNex_List, idx AS LONG, value$)
    DIM items() AS STRING, newItems() AS STRING
    DIM n AS LONG, i AS LONG, j AS LONG
    n = lst.Count
    _List_Explode lst, items()
    REDIM newItems(1 TO n + 2) AS STRING
    j = 0
    FOR i = 1 TO n + 1
        IF i = idx THEN
            j = j + 1: newItems(j) = value$
        END IF
        IF i <= n THEN j = j + 1: newItems(j) = items(i)
    NEXT i
    _List_Rebuild lst, newItems(), n + 1
END SUB

' ---------------------------------------------------------------------------
' SUB  List_RemoveAt(lst, index&)  — remove item at index
' ---------------------------------------------------------------------------
SUB List_RemoveAt (lst AS QBNex_List, idx AS LONG)
    DIM items() AS STRING, newItems() AS STRING
    DIM n AS LONG, i AS LONG, j AS LONG
    IF idx < 1 OR idx > lst.Count THEN EXIT SUB
    n = lst.Count
    _List_Explode lst, items()
    REDIM newItems(1 TO n) AS STRING
    j = 0
    FOR i = 1 TO n
        IF i <> idx THEN j = j + 1: newItems(j) = items(i)
    NEXT i
    _List_Rebuild lst, newItems(), n - 1
END SUB

' ---------------------------------------------------------------------------
' FUNCTION  List_IndexOf&(lst, value$)  — first occurrence (1-based), 0=not found
' ---------------------------------------------------------------------------
FUNCTION List_IndexOf& (lst AS QBNex_List, value$)
    DIM i AS LONG
    FOR i = 1 TO lst.Count
        IF _List_GetRaw$(lst, i) = value$ THEN
            List_IndexOf& = i: EXIT FUNCTION
        END IF
    NEXT i
    List_IndexOf& = 0
END FUNCTION

' ---------------------------------------------------------------------------
' FUNCTION  List_Contains&(lst, value$)  — -1 if found, 0 if not
' ---------------------------------------------------------------------------
FUNCTION List_Contains& (lst AS QBNex_List, value$)
    List_Contains& = (List_IndexOf&(lst, value$) > 0)
END FUNCTION

' ---------------------------------------------------------------------------
' SUB  List_Clear(lst)  — remove all items
' ---------------------------------------------------------------------------
SUB List_Clear (lst AS QBNex_List)
    IF lst._handle > 0 THEN QBNEX_List_PoolData(lst._handle) = ""
    lst.Count = 0
END SUB

' ---------------------------------------------------------------------------
' SUB  List_Sort(lst)   — sort items alphabetically (ascending)
'
'   Uses insertion sort (adequate for typical list sizes in BASIC programs;
'   replace with quicksort for performance-critical paths).
' ---------------------------------------------------------------------------
SUB List_Sort (lst AS QBNex_List)
    DIM items() AS STRING, i AS LONG, j AS LONG, tmp AS STRING
    IF lst.Count < 2 THEN EXIT SUB
    _List_Explode lst, items()
    ' Insertion sort
    FOR i = 2 TO lst.Count
        tmp = items(i)
        j = i - 1
        DO WHILE j >= 1 AND items(j) > tmp
            items(j + 1) = items(j)
            j = j - 1
        LOOP
        items(j + 1) = tmp
    NEXT i
    _List_Rebuild lst, items(), lst.Count
END SUB

' ---------------------------------------------------------------------------
' SUB  List_SortDesc(lst)   — sort descending
' ---------------------------------------------------------------------------
SUB List_SortDesc (lst AS QBNex_List)
    DIM items() AS STRING, i AS LONG, j AS LONG, tmp AS STRING
    IF lst.Count < 2 THEN EXIT SUB
    _List_Explode lst, items()
    FOR i = 2 TO lst.Count
        tmp = items(i)
        j = i - 1
        DO WHILE j >= 1 AND items(j) < tmp
            items(j + 1) = items(j)
            j = j - 1
        LOOP
        items(j + 1) = tmp
    NEXT i
    _List_Rebuild lst, items(), lst.Count
END SUB

' ---------------------------------------------------------------------------
' SUB  List_Reverse(lst)    — reverse order
' ---------------------------------------------------------------------------
SUB List_Reverse (lst AS QBNex_List)
    DIM items() AS STRING, i AS LONG, j AS LONG, tmp AS STRING
    IF lst.Count < 2 THEN EXIT SUB
    _List_Explode lst, items()
    i = 1: j = lst.Count
    DO WHILE i < j
        tmp = items(i): items(i) = items(j): items(j) = tmp
        i = i + 1: j = j - 1
    LOOP
    _List_Rebuild lst, items(), lst.Count
END SUB

' ---------------------------------------------------------------------------
' SUB  List_Print(lst)      — debug dump
' ---------------------------------------------------------------------------
SUB List_Print (lst AS QBNex_List)
    DIM i AS LONG
    PRINT "List (Count=" + STR$(lst.Count) + "):"
    FOR i = 1 TO lst.Count
        PRINT "  [" + STR$(i) + "] = " + List_Get$(lst, i)
    NEXT i
END SUB
