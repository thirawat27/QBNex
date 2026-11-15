' ============================================================================
' QBNex Standard Library - Collections: List
' ============================================================================
' Every item of every list is one element of the shared item pool
' (QBNEX_ListItem); a list keeps the pool element numbers of its items, in
' order, as 4-byte MKL$ values in QBNEX_ListPool(handle). So an item is read
' or replaced in constant time, and items may hold any bytes. The pools grow
' as needed (there is no fixed number of lists or items); the elements of a
' removed item and the handle of a freed list are used again.
' ============================================================================

TYPE QBNex_List
    Handle AS LONG
    Count AS LONG
END TYPE

' per handle: item slots, in use, number of items
DIM SHARED QBNEX_ListPool() AS STRING
DIM SHARED QBNEX_ListPoolUsed() AS LONG
DIM SHARED QBNEX_ListPoolCount() AS LONG
DIM SHARED QBNEX_ListHandleCap AS LONG
DIM SHARED QBNEX_ListHandleTop AS LONG
DIM SHARED QBNEX_ListFreeHandle AS LONG 'first freed handle; QBNEX_ListPoolCount of a freed one is the next
' item pool; QBNEX_ListNextFree links the free elements, from QBNEX_ListFreeItem
DIM SHARED QBNEX_ListItem() AS STRING
DIM SHARED QBNEX_ListNextFree() AS LONG
DIM SHARED QBNEX_ListItemCap AS LONG
DIM SHARED QBNEX_ListItemTop AS LONG
DIM SHARED QBNEX_ListFreeItem AS LONG

SUB List_Init (listRef AS QBNex_List)
    DIM handle AS LONG

    IF QBNEX_ListFreeHandle THEN
        handle = QBNEX_ListFreeHandle
        QBNEX_ListFreeHandle = QBNEX_ListPoolCount(handle)
    ELSE
        QBNEX_ListHandleTop = QBNEX_ListHandleTop + 1
        handle = QBNEX_ListHandleTop
        IF handle > QBNEX_ListHandleCap THEN
            QBNEX_ListHandleCap = QBNEX_ListHandleCap * 2
            IF QBNEX_ListHandleCap < 16 THEN QBNEX_ListHandleCap = 16
            REDIM _PRESERVE QBNEX_ListPool(1 TO QBNEX_ListHandleCap) AS STRING
            REDIM _PRESERVE QBNEX_ListPoolUsed(1 TO QBNEX_ListHandleCap) AS LONG
            REDIM _PRESERVE QBNEX_ListPoolCount(1 TO QBNEX_ListHandleCap) AS LONG
        END IF
    END IF

    QBNEX_ListPoolUsed(handle) = -1
    QBNEX_ListPool(handle) = ""
    QBNEX_ListPoolCount(handle) = 0
    listRef.Handle = handle
    listRef.Count = 0
END SUB

' The handle of a list in use, 0 for one that was not initialised or is freed.
FUNCTION QBNEX_ListHandleOf& (listRef AS QBNex_List)
    DIM handle AS LONG

    handle = listRef.Handle
    IF handle < 1 OR handle > QBNEX_ListHandleTop THEN EXIT FUNCTION
    IF QBNEX_ListPoolUsed(handle) = 0 THEN EXIT FUNCTION
    QBNEX_ListHandleOf = handle
END FUNCTION

FUNCTION QBNEX_ListNewItem& (item AS STRING)
    DIM slot AS LONG

    IF QBNEX_ListFreeItem THEN
        slot = QBNEX_ListFreeItem
        QBNEX_ListFreeItem = QBNEX_ListNextFree(slot)
    ELSE
        QBNEX_ListItemTop = QBNEX_ListItemTop + 1
        slot = QBNEX_ListItemTop
        IF slot > QBNEX_ListItemCap THEN
            QBNEX_ListItemCap = QBNEX_ListItemCap * 2
            IF QBNEX_ListItemCap < 64 THEN QBNEX_ListItemCap = 64
            REDIM _PRESERVE QBNEX_ListItem(1 TO QBNEX_ListItemCap) AS STRING
            REDIM _PRESERVE QBNEX_ListNextFree(1 TO QBNEX_ListItemCap) AS LONG
        END IF
    END IF
    QBNEX_ListItem(slot) = item
    QBNEX_ListNewItem = slot
END FUNCTION

SUB QBNEX_ListDropItem (slot AS LONG)
    QBNEX_ListItem(slot) = ""
    QBNEX_ListNextFree(slot) = QBNEX_ListFreeItem
    QBNEX_ListFreeItem = slot
END SUB

SUB QBNEX_ListDropItems (handle AS LONG)
    DIM i AS LONG

    FOR i = 1 TO LEN(QBNEX_ListPool(handle)) STEP 4
        QBNEX_ListDropItem CVL(MID$(QBNEX_ListPool(handle), i, 4))
    NEXT
    QBNEX_ListPool(handle) = ""
    QBNEX_ListPoolCount(handle) = 0
END SUB

FUNCTION List_Count& (listRef AS QBNex_List)
    DIM handle AS LONG

    ' the count kept by the list itself: a copy of the TYPE value may be out of date
    handle = QBNEX_ListHandleOf&(listRef)
    IF handle THEN listRef.Count = QBNEX_ListPoolCount(handle) ELSE listRef.Count = 0
    List_Count = listRef.Count
END FUNCTION

SUB List_Add (listRef AS QBNex_List, item AS STRING)
    DIM handle AS LONG

    handle = QBNEX_ListHandleOf&(listRef)
    IF handle = 0 THEN EXIT SUB

    QBNEX_ListPool(handle) = QBNEX_ListPool(handle) + MKL$(QBNEX_ListNewItem&(item))
    QBNEX_ListPoolCount(handle) = QBNEX_ListPoolCount(handle) + 1
    listRef.Count = QBNEX_ListPoolCount(handle)
END SUB

FUNCTION List_Get$ (listRef AS QBNex_List, index AS LONG)
    DIM handle AS LONG

    handle = QBNEX_ListHandleOf&(listRef)
    IF handle = 0 THEN EXIT FUNCTION
    IF index < 0 OR index >= QBNEX_ListPoolCount(handle) THEN EXIT FUNCTION

    List_Get = QBNEX_ListItem(CVL(MID$(QBNEX_ListPool(handle), index * 4 + 1, 4)))
END FUNCTION

SUB List_Set (listRef AS QBNex_List, index AS LONG, item AS STRING)
    DIM handle AS LONG

    handle = QBNEX_ListHandleOf&(listRef)
    IF handle = 0 THEN EXIT SUB
    IF index < 0 OR index >= QBNEX_ListPoolCount(handle) THEN EXIT SUB

    QBNEX_ListItem(CVL(MID$(QBNEX_ListPool(handle), index * 4 + 1, 4))) = item
END SUB

FUNCTION List_IndexOf& (listRef AS QBNex_List, item AS STRING)
    DIM handle AS LONG
    DIM i AS LONG

    List_IndexOf = -1
    handle = QBNEX_ListHandleOf&(listRef)
    IF handle = 0 THEN EXIT FUNCTION

    FOR i = 0 TO QBNEX_ListPoolCount(handle) - 1
        IF QBNEX_ListItem(CVL(MID$(QBNEX_ListPool(handle), i * 4 + 1, 4))) = item THEN
            List_IndexOf = i
            EXIT FUNCTION
        END IF
    NEXT
END FUNCTION

FUNCTION List_Contains& (listRef AS QBNex_List, item AS STRING)
    List_Contains = List_IndexOf&(listRef, item) >= 0
END FUNCTION

SUB List_RemoveAt (listRef AS QBNex_List, index AS LONG)
    DIM handle AS LONG
    DIM position AS LONG
    DIM slot AS LONG

    handle = QBNEX_ListHandleOf&(listRef)
    IF handle = 0 THEN EXIT SUB
    IF index < 0 OR index >= QBNEX_ListPoolCount(handle) THEN EXIT SUB

    position = index * 4 + 1
    slot = CVL(MID$(QBNEX_ListPool(handle), position, 4))
    QBNEX_ListDropItem slot
    QBNEX_ListPool(handle) = LEFT$(QBNEX_ListPool(handle), position - 1) + MID$(QBNEX_ListPool(handle), position + 4)
    QBNEX_ListPoolCount(handle) = QBNEX_ListPoolCount(handle) - 1
    listRef.Count = QBNEX_ListPoolCount(handle)
END SUB

SUB List_Clear (listRef AS QBNex_List)
    DIM handle AS LONG

    handle = QBNEX_ListHandleOf&(listRef)
    IF handle = 0 THEN EXIT SUB

    QBNEX_ListDropItems handle
    listRef.Count = 0
END SUB

FUNCTION List_Join$ (listRef AS QBNex_List, separator AS STRING)
    DIM handle AS LONG
    DIM i AS LONG
    DIM joined AS STRING

    handle = QBNEX_ListHandleOf&(listRef)
    IF handle = 0 THEN EXIT FUNCTION

    FOR i = 0 TO QBNEX_ListPoolCount(handle) - 1
        IF i > 0 THEN joined = joined + separator
        joined = joined + QBNEX_ListItem(CVL(MID$(QBNEX_ListPool(handle), i * 4 + 1, 4)))
    NEXT

    List_Join = joined
END FUNCTION

SUB List_Free (listRef AS QBNex_List)
    DIM handle AS LONG

    handle = QBNEX_ListHandleOf&(listRef)
    IF handle = 0 THEN EXIT SUB

    QBNEX_ListDropItems handle
    QBNEX_ListPoolUsed(handle) = 0
    QBNEX_ListPoolCount(handle) = QBNEX_ListFreeHandle
    QBNEX_ListFreeHandle = handle
    listRef.Handle = 0
    listRef.Count = 0
END SUB
