' ============================================================================
' QBNex Standard Library - Collections: List
' ============================================================================
' String values are stored with a 4-byte length prefix so item contents may
' contain any printable character without colliding with a delimiter.
' ============================================================================

TYPE QBNex_List
    Handle AS LONG
    Count AS LONG
END TYPE

DIM SHARED QBNEX_ListPool(1 TO 256) AS STRING
DIM SHARED QBNEX_ListPoolUsed(1 TO 256) AS LONG
DIM SHARED QBNEX_ListPoolCount(1 TO 256) AS LONG

SUB List_Init (listRef AS QBNex_List)
    DIM i AS LONG

    FOR i = 1 TO 256
        IF QBNEX_ListPoolUsed(i) = 0 THEN
            listRef.Handle = i
            listRef.Count = 0
            QBNEX_ListPoolUsed(i) = -1
            QBNEX_ListPool(i) = ""
            QBNEX_ListPoolCount(i) = 0
            EXIT SUB
        END IF
    NEXT

    PRINT "ERROR: List pool exhausted"
    SYSTEM 1
END SUB

FUNCTION List_Count& (listRef AS QBNex_List)
    List_Count = listRef.Count
END FUNCTION

SUB List_Add (listRef AS QBNex_List, item AS STRING)
    DIM handle AS LONG

    handle = listRef.Handle
    IF handle < 1 OR handle > 256 THEN EXIT SUB

    QBNEX_ListPool(handle) = QBNEX_ListPool(handle) + MKL$(LEN(item)) + item
    QBNEX_ListPoolCount(handle) = QBNEX_ListPoolCount(handle) + 1
    listRef.Count = QBNEX_ListPoolCount(handle)
END SUB

FUNCTION List_Get$ (listRef AS QBNex_List, index AS LONG)
    DIM handle AS LONG
    DIM position AS LONG
    DIM currentIndex AS LONG
    DIM itemLength AS LONG

    handle = listRef.Handle
    IF handle < 1 OR handle > 256 THEN EXIT FUNCTION
    IF index < 0 OR index >= QBNEX_ListPoolCount(handle) THEN EXIT FUNCTION

    position = 1
    FOR currentIndex = 0 TO QBNEX_ListPoolCount(handle) - 1
        itemLength = CVL(MID$(QBNEX_ListPool(handle), position, 4))
        position = position + 4
        IF currentIndex = index THEN
            List_Get = MID$(QBNEX_ListPool(handle), position, itemLength)
            EXIT FUNCTION
        END IF
        position = position + itemLength
    NEXT
END FUNCTION

SUB List_Set (listRef AS QBNex_List, index AS LONG, item AS STRING)
    DIM rebuilt AS STRING
    DIM i AS LONG

    IF index < 0 OR index >= listRef.Count THEN EXIT SUB

    FOR i = 0 TO listRef.Count - 1
        IF i = index THEN
            rebuilt = rebuilt + MKL$(LEN(item)) + item
        ELSE
            rebuilt = rebuilt + MKL$(LEN(List_Get$(listRef, i))) + List_Get$(listRef, i)
        END IF
    NEXT

    QBNEX_ListPool(listRef.Handle) = rebuilt
END SUB

FUNCTION List_IndexOf& (listRef AS QBNex_List, item AS STRING)
    DIM i AS LONG

    FOR i = 0 TO listRef.Count - 1
        IF List_Get$(listRef, i) = item THEN
            List_IndexOf = i
            EXIT FUNCTION
        END IF
    NEXT

    List_IndexOf = -1
END FUNCTION

FUNCTION List_Contains& (listRef AS QBNex_List, item AS STRING)
    List_Contains = List_IndexOf&(listRef, item) >= 0
END FUNCTION

SUB List_RemoveAt (listRef AS QBNex_List, index AS LONG)
    DIM rebuilt AS STRING
    DIM i AS LONG
    DIM value AS STRING

    IF index < 0 OR index >= listRef.Count THEN EXIT SUB

    FOR i = 0 TO listRef.Count - 1
        IF i <> index THEN
            value = List_Get$(listRef, i)
            rebuilt = rebuilt + MKL$(LEN(value)) + value
        END IF
    NEXT

    QBNEX_ListPool(listRef.Handle) = rebuilt
    QBNEX_ListPoolCount(listRef.Handle) = QBNEX_ListPoolCount(listRef.Handle) - 1
    listRef.Count = QBNEX_ListPoolCount(listRef.Handle)
END SUB

SUB List_Clear (listRef AS QBNex_List)
    IF listRef.Handle < 1 OR listRef.Handle > 256 THEN EXIT SUB

    QBNEX_ListPool(listRef.Handle) = ""
    QBNEX_ListPoolCount(listRef.Handle) = 0
    listRef.Count = 0
END SUB

FUNCTION List_Join$ (listRef AS QBNex_List, separator AS STRING)
    DIM i AS LONG
    DIM joined AS STRING

    FOR i = 0 TO listRef.Count - 1
        IF i > 0 THEN joined = joined + separator
        joined = joined + List_Get$(listRef, i)
    NEXT

    List_Join = joined
END FUNCTION

SUB List_Free (listRef AS QBNex_List)
    IF listRef.Handle < 1 OR listRef.Handle > 256 THEN EXIT SUB

    QBNEX_ListPool(listRef.Handle) = ""
    QBNEX_ListPoolCount(listRef.Handle) = 0
    QBNEX_ListPoolUsed(listRef.Handle) = 0
    listRef.Handle = 0
    listRef.Count = 0
END SUB
