' ============================================================================
' QBNex Standard Library - Collections: Dynamic List (ArrayList)
' ============================================================================
' Implements a dynamic resizable list using global pool storage
' Items separated by CHR$(1) within string buffers
' ============================================================================

' List structure
TYPE QBNex_List
    Handle AS LONG
    Count AS LONG
    Capacity AS LONG
END TYPE

' Global pool storage (max 256 lists)
DIM SHARED QBNEX_ListPool(1 TO 256) AS STRING
DIM SHARED QBNEX_ListPoolUsed(1 TO 256) AS LONG
DIM SHARED QBNEX_ListPoolCount(1 TO 256) AS LONG

CONST QBNEX_LIST_SEPARATOR = 1 ' CHR$(1)

' ============================================================================
' SUB: List_Init
' Initialize a new list
' ============================================================================
SUB List_Init (list AS QBNex_List)
    DIM i AS LONG
    
    ' Find free pool slot
    FOR i = 1 TO 256
        IF QBNEX_ListPoolUsed(i) = 0 THEN
            list.Handle = i
            list.Count = 0
            list.Capacity = 0
            QBNEX_ListPoolUsed(i) = -1
            QBNEX_ListPool(i) = ""
            QBNEX_ListPoolCount(i) = 0
            EXIT SUB
        END IF
    NEXT i
    
    PRINT "ERROR: List pool exhausted (max 256 lists)"
END SUB

' ============================================================================
' SUB: List_Add
' Add an item to the end of the list
' ============================================================================
SUB List_Add (list AS QBNex_List, item AS STRING)
    DIM h AS LONG
    h = list.Handle
    
    IF h < 1 OR h > 256 THEN EXIT SUB
    
    IF QBNEX_ListPoolCount(h) > 0 THEN
        QBNEX_ListPool(h) = QBNEX_ListPool(h) + CHR$(QBNEX_LIST_SEPARATOR) + item
    ELSE
        QBNEX_ListPool(h) = item
    END IF
    
    QBNEX_ListPoolCount(h) = QBNEX_ListPoolCount(h) + 1
    list.Count = QBNEX_ListPoolCount(h)
END SUB

' ============================================================================
' FUNCTION: List_Get
' Get an item at specified index (0-based)
' ============================================================================
FUNCTION List_Get$ (list AS QBNex_List, index AS LONG)
    DIM h AS LONG
    DIM POS AS LONG
    DIM startPos AS LONG
    DIM endPos AS LONG
    DIM i AS LONG
    DIM currentIndex AS LONG
    
    h = list.Handle
    IF h < 1 OR h > 256 THEN
        List_Get = ""
        EXIT FUNCTION
    END IF
    
    IF index < 0 OR index >= QBNEX_ListPoolCount(h) THEN
        List_Get = ""
        EXIT FUNCTION
    END IF
    
    ' Find the item by scanning separators
    startPos = 1
    currentIndex = 0
    
    FOR i = 1 TO LEN(QBNEX_ListPool(h))
        IF ASC(MID$(QBNEX_ListPool(h), i, 1)) = QBNEX_LIST_SEPARATOR THEN
            IF currentIndex = index THEN
                endPos = i - 1
                List_Get = MID$(QBNEX_ListPool(h), startPos, endPos - startPos + 1)
                EXIT FUNCTION
            END IF
            currentIndex = currentIndex + 1
            startPos = i + 1
        END IF
    NEXT i
    
    ' Last item (no trailing separator)
    IF currentIndex = index THEN
        List_Get = MID$(QBNEX_ListPool(h), startPos)
    ELSE
        List_Get = ""
    END IF
END FUNCTION

' ============================================================================
' SUB: List_Set
' Set an item at specified index
' ============================================================================
SUB List_Set (list AS QBNex_List, index AS LONG, item AS STRING)
    DIM h AS LONG
    DIM temp AS STRING
    DIM i AS LONG
    
    h = list.Handle
    IF h < 1 OR h > 256 THEN EXIT SUB
    IF index < 0 OR index >= QBNEX_ListPoolCount(h) THEN EXIT SUB
    
    ' Rebuild list with new item
    temp = ""
    FOR i = 0 TO QBNEX_ListPoolCount(h) - 1
        IF i = index THEN
            IF i > 0 THEN temp = temp + CHR$(QBNEX_LIST_SEPARATOR)
            temp = temp + item
        ELSE
            IF i > 0 THEN temp = temp + CHR$(QBNEX_LIST_SEPARATOR)
            temp = temp + List_Get(list, i)
        END IF
    NEXT i
    
    QBNEX_ListPool(h) = temp
END SUB

' ============================================================================
' FUNCTION: List_Contains
' Check if list contains an item
' ============================================================================
FUNCTION List_Contains& (list AS QBNex_List, item AS STRING)
    DIM i AS LONG
    FOR i = 0 TO list.Count - 1
        IF List_Get(list, i) = item THEN
            List_Contains = -1
            EXIT FUNCTION
        END IF
    NEXT i
    List_Contains = 0
END FUNCTION

' ============================================================================
' FUNCTION: List_IndexOf
' Find index of an item (-1 if not found)
' ============================================================================
FUNCTION List_IndexOf& (list AS QBNex_List, item AS STRING)
    DIM i AS LONG
    FOR i = 0 TO list.Count - 1
        IF List_Get(list, i) = item THEN
            List_IndexOf = i
            EXIT FUNCTION
        END IF
    NEXT i
    List_IndexOf = -1
END FUNCTION

' ============================================================================
' SUB: List_RemoveAt
' Remove item at specified index
' ============================================================================
SUB List_RemoveAt (list AS QBNex_List, index AS LONG)
    DIM h AS LONG
    DIM temp AS STRING
    DIM i AS LONG
    DIM first AS LONG
    
    h = list.Handle
    IF h < 1 OR h > 256 THEN EXIT SUB
    IF index < 0 OR index >= QBNEX_ListPoolCount(h) THEN EXIT SUB
    
    ' Rebuild list without the item
    temp = ""
    first = -1
    FOR i = 0 TO QBNEX_ListPoolCount(h) - 1
        IF i <> index THEN
            IF first THEN
                first = 0
            ELSE
                temp = temp + CHR$(QBNEX_LIST_SEPARATOR)
            END IF
            temp = temp + List_Get(list, i)
        END IF
    NEXT i
    
    QBNEX_ListPool(h) = temp
    QBNEX_ListPoolCount(h) = QBNEX_ListPoolCount(h) - 1
    list.Count = QBNEX_ListPoolCount(h)
END SUB

' ============================================================================
' SUB: List_Clear
' Remove all items from the list
' ============================================================================
SUB List_Clear (list AS QBNex_List)
    DIM h AS LONG
    h = list.Handle
    IF h < 1 OR h > 256 THEN EXIT SUB
    
    QBNEX_ListPool(h) = ""
    QBNEX_ListPoolCount(h) = 0
    list.Count = 0
END SUB

' ============================================================================
' SUB: List_Free
' Free the list and return pool slot
' ============================================================================
SUB List_Free (list AS QBNex_List)
    DIM h AS LONG
    h = list.Handle
    IF h < 1 OR h > 256 THEN EXIT SUB
    
    QBNEX_ListPool(h) = ""
    QBNEX_ListPoolUsed(h) = 0
    QBNEX_ListPoolCount(h) = 0
    list.Handle = 0
    list.Count = 0
END SUB
