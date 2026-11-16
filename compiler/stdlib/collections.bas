'===============================================================================
' QBNex Standard Library - collections ('$IMPORT:'collections')
'===============================================================================
' List, Dict and Set of strings. A new variable is ready to use (no Init); a
' copy of it (b = a) is the same collection, as in Python. Free gives the
' memory back early; otherwise it lasts until the program ends.
'
'   DIM names AS List
'   names.Add "Ann": names.Add "Bob"
'   PRINT names.Count, names.Get(-1), names.Join(", ")
'   DIM ages AS Dict
'   ages.Set "Ann", "42": PRINT ages.Get("Ann")
'
' Indexes start at 0; a negative index counts from the end (-1 = last).
' Numbers are stored as text: names.Add STR$(n), VAL(names.Get(0)).
'===============================================================================

TYPE List
    Handle AS LONG
END TYPE

TYPE Dict
    KeyList AS List
    ValueList AS List
END TYPE

TYPE Set
    Items AS List
END TYPE

' Every item of every list is one element of the item pool; a list keeps the
' element numbers of its items, in order, as 4-byte MKL$ values. Pools grow as
' needed; freed elements and handles are used again.
DIM SHARED Coll_Slots() AS STRING
DIM SHARED Coll_Used() AS LONG
DIM SHARED Coll_Count() AS LONG
DIM SHARED Coll_HandleCap AS LONG
DIM SHARED Coll_HandleTop AS LONG
DIM SHARED Coll_FreeHandle AS LONG
DIM SHARED Coll_Item() AS STRING
DIM SHARED Coll_NextFree() AS LONG
DIM SHARED Coll_ItemCap AS LONG
DIM SHARED Coll_ItemTop AS LONG
DIM SHARED Coll_FreeItem AS LONG

'------------------------------------------------------------------- internals

FUNCTION Coll_NewHandle& ()
    DIM h AS LONG
    IF Coll_FreeHandle THEN
        h = Coll_FreeHandle
        Coll_FreeHandle = Coll_Count(h)
    ELSE
        Coll_HandleTop = Coll_HandleTop + 1
        h = Coll_HandleTop
        IF h > Coll_HandleCap THEN
            Coll_HandleCap = Coll_HandleCap * 2
            IF Coll_HandleCap < 16 THEN Coll_HandleCap = 16
            REDIM _PRESERVE Coll_Slots(1 TO Coll_HandleCap) AS STRING
            REDIM _PRESERVE Coll_Used(1 TO Coll_HandleCap) AS LONG
            REDIM _PRESERVE Coll_Count(1 TO Coll_HandleCap) AS LONG
        END IF
    END IF
    Coll_Used(h) = -1
    Coll_Slots(h) = ""
    Coll_Count(h) = 0
    Coll_NewHandle& = h
END FUNCTION

' The handle of a list; a list never used gets one when create is set.
FUNCTION Coll_Of& (l AS List, create AS LONG)
    DIM h AS LONG
    h = l.Handle
    IF h >= 1 AND h <= Coll_HandleTop THEN
        IF Coll_Used(h) THEN Coll_Of& = h: EXIT FUNCTION
    END IF
    IF create THEN
        l.Handle = Coll_NewHandle&
        Coll_Of& = l.Handle
    END IF
END FUNCTION

FUNCTION Coll_NewItem& (item AS STRING)
    DIM slot AS LONG
    IF Coll_FreeItem THEN
        slot = Coll_FreeItem
        Coll_FreeItem = Coll_NextFree(slot)
    ELSE
        Coll_ItemTop = Coll_ItemTop + 1
        slot = Coll_ItemTop
        IF slot > Coll_ItemCap THEN
            Coll_ItemCap = Coll_ItemCap * 2
            IF Coll_ItemCap < 64 THEN Coll_ItemCap = 64
            REDIM _PRESERVE Coll_Item(1 TO Coll_ItemCap) AS STRING
            REDIM _PRESERVE Coll_NextFree(1 TO Coll_ItemCap) AS LONG
        END IF
    END IF
    Coll_Item(slot) = item
    Coll_NewItem& = slot
END FUNCTION

SUB Coll_DropItem (slot AS LONG)
    Coll_Item(slot) = ""
    Coll_NextFree(slot) = Coll_FreeItem
    Coll_FreeItem = slot
END SUB

' Pool element of item index (already checked) of list h.
FUNCTION Coll_SlotAt& (h AS LONG, index AS LONG)
    Coll_SlotAt& = CVL(MID$(Coll_Slots(h), index * 4 + 1, 4))
END FUNCTION

' 0-based position for index (negative counts from the end); -1 if outside.
FUNCTION Coll_Pos& (h AS LONG, index AS LONG)
    DIM p AS LONG
    p = index
    IF p < 0 THEN p = Coll_Count(h) + p
    IF p < 0 OR p >= Coll_Count(h) THEN p = -1
    Coll_Pos& = p
END FUNCTION

'------------------------------------------------------------------------ List

' Adds item at the end (Python append, JS push).
SUB List_Add (l AS List, item AS STRING)
    DIM h AS LONG
    h = Coll_Of&(l, -1)
    Coll_Slots(h) = Coll_Slots(h) + MKL$(Coll_NewItem&(item))
    Coll_Count(h) = Coll_Count(h) + 1
END SUB

' Puts item before position index (0 = first; Count or more = at the end).
SUB List_Insert (l AS List, index AS LONG, item AS STRING)
    DIM h AS LONG, p AS LONG
    h = Coll_Of&(l, -1)
    p = index
    IF p < 0 THEN p = Coll_Count(h) + p
    IF p < 0 THEN p = 0
    IF p > Coll_Count(h) THEN p = Coll_Count(h)
    Coll_Slots(h) = LEFT$(Coll_Slots(h), p * 4) + MKL$(Coll_NewItem&(item)) + MID$(Coll_Slots(h), p * 4 + 1)
    Coll_Count(h) = Coll_Count(h) + 1
END SUB

' Number of items (Python len).
FUNCTION List_Count& (l AS List)
    DIM h AS LONG
    h = Coll_Of&(l, 0)
    IF h THEN List_Count& = Coll_Count(h)
END FUNCTION

' Item at index; "" when there is none.
FUNCTION List_Get$ (l AS List, index AS LONG)
    DIM h AS LONG, p AS LONG
    h = Coll_Of&(l, 0)
    IF h = 0 THEN EXIT FUNCTION
    p = Coll_Pos&(h, index)
    IF p >= 0 THEN List_Get$ = Coll_Item(Coll_SlotAt&(h, p))
END FUNCTION

' Replaces the item at index (nothing happens outside the list).
SUB List_Set (l AS List, index AS LONG, item AS STRING)
    DIM h AS LONG, p AS LONG
    h = Coll_Of&(l, 0)
    IF h = 0 THEN EXIT SUB
    p = Coll_Pos&(h, index)
    IF p >= 0 THEN Coll_Item(Coll_SlotAt&(h, p)) = item
END SUB

' Removes the item at index and returns it (default: the last one).
FUNCTION List_Pop$ (l AS List, OPTIONAL index AS LONG = -1)
    DIM h AS LONG, p AS LONG, slot AS LONG
    h = Coll_Of&(l, 0)
    IF h = 0 THEN EXIT FUNCTION
    p = Coll_Pos&(h, index)
    IF p < 0 THEN EXIT FUNCTION
    slot = Coll_SlotAt&(h, p)
    List_Pop$ = Coll_Item(slot)
    Coll_DropItem slot
    Coll_Slots(h) = LEFT$(Coll_Slots(h), p * 4) + MID$(Coll_Slots(h), p * 4 + 5)
    Coll_Count(h) = Coll_Count(h) - 1
END FUNCTION

' Removes the item at index.
SUB List_RemoveAt (l AS List, index AS LONG)
    DIM gone AS STRING
    gone = List_Pop$(l, index)
END SUB

' Removes the first item equal to item (Python remove).
SUB List_Remove (l AS List, item AS STRING)
    DIM p AS LONG
    p = List_IndexOf&(l, item)
    IF p >= 0 THEN List_RemoveAt l, p
END SUB

' Position of the first item equal to item; -1 when there is none.
FUNCTION List_IndexOf& (l AS List, item AS STRING)
    DIM h AS LONG, i AS LONG
    List_IndexOf& = -1
    h = Coll_Of&(l, 0)
    IF h = 0 THEN EXIT FUNCTION
    FOR i = 0 TO Coll_Count(h) - 1
        IF Coll_Item(Coll_SlotAt&(h, i)) = item THEN List_IndexOf& = i: EXIT FUNCTION
    NEXT
END FUNCTION

' -1 when an item equals item (Python "in").
FUNCTION List_Has& (l AS List, item AS STRING)
    List_Has& = (List_IndexOf&(l, item) >= 0)
END FUNCTION

' The items with sep between them (Python sep.join, JS join).
FUNCTION List_Join$ (l AS List, OPTIONAL sep AS STRING = ", ")
    DIM h AS LONG, i AS LONG, r AS STRING
    h = Coll_Of&(l, 0)
    IF h = 0 THEN EXIT FUNCTION
    FOR i = 0 TO Coll_Count(h) - 1
        IF i THEN r = r + sep
        r = r + Coll_Item(Coll_SlotAt&(h, i))
    NEXT
    List_Join$ = r
END FUNCTION

' Replaces the items with the parts of text between seps (Python split).
SUB List_Split (l AS List, text AS STRING, OPTIONAL sep AS STRING = ",")
    DIM p AS LONG, q AS LONG
    List_Clear l
    IF LEN(sep) = 0 THEN
        FOR p = 1 TO LEN(text): List_Add l, MID$(text, p, 1): NEXT
        EXIT SUB
    END IF
    p = 1
    DO
        q = INSTR(p, text, sep)
        IF q = 0 THEN List_Add l, MID$(text, p): EXIT DO
        List_Add l, MID$(text, p, q - p)
        p = q + LEN(sep)
    LOOP
END SUB

' Sorts the items by their bytes (descending when descending is set).
SUB List_Sort (l AS List, OPTIONAL descending AS LONG = 0)
    DIM h AS LONG, n AS LONG, gap AS LONG, i AS LONG, j AS LONG, t AS LONG, s AS STRING
    h = Coll_Of&(l, 0)
    IF h = 0 THEN EXIT SUB
    n = Coll_Count(h)
    IF n < 2 THEN EXIT SUB
    REDIM order(0 TO n - 1) AS LONG
    FOR i = 0 TO n - 1: order(i) = Coll_SlotAt&(h, i): NEXT
    gap = n \ 2
    DO WHILE gap > 0
        FOR i = gap TO n - 1
            t = order(i)
            j = i
            DO WHILE j >= gap
                IF descending THEN
                    IF Coll_Item(order(j - gap)) >= Coll_Item(t) THEN EXIT DO
                ELSE
                    IF Coll_Item(order(j - gap)) <= Coll_Item(t) THEN EXIT DO
                END IF
                order(j) = order(j - gap)
                j = j - gap
            LOOP
            order(j) = t
        NEXT
        gap = gap \ 2
    LOOP
    FOR i = 0 TO n - 1: s = s + MKL$(order(i)): NEXT
    Coll_Slots(h) = s
END SUB

' Reverses the order of the items.
SUB List_Reverse (l AS List)
    DIM h AS LONG, i AS LONG, s AS STRING
    h = Coll_Of&(l, 0)
    IF h = 0 THEN EXIT SUB
    FOR i = Coll_Count(h) - 1 TO 0 STEP -1
        s = s + MID$(Coll_Slots(h), i * 4 + 1, 4)
    NEXT
    Coll_Slots(h) = s
END SUB

' Removes every item.
SUB List_Clear (l AS List)
    DIM h AS LONG, i AS LONG
    h = Coll_Of&(l, 0)
    IF h = 0 THEN EXIT SUB
    FOR i = 0 TO Coll_Count(h) - 1
        Coll_DropItem Coll_SlotAt&(h, i)
    NEXT
    Coll_Slots(h) = ""
    Coll_Count(h) = 0
END SUB

' Gives the memory back; the variable is then an empty, new list.
SUB List_Free (l AS List)
    DIM h AS LONG
    h = Coll_Of&(l, 0)
    IF h = 0 THEN EXIT SUB
    List_Clear l
    Coll_Used(h) = 0
    Coll_Count(h) = Coll_FreeHandle
    Coll_FreeHandle = h
    l.Handle = 0
END SUB

'------------------------------------------------------------------------ Dict

' Sets the value of key (a new key goes at the end; keys keep their order).
SUB Dict_Set (d AS Dict, keyName AS STRING, value AS STRING)
    DIM p AS LONG
    p = List_IndexOf&(d.KeyList, keyName)
    IF p >= 0 THEN
        List_Set d.ValueList, p, value
    ELSE
        List_Add d.KeyList, keyName
        List_Add d.ValueList, value
    END IF
END SUB

' Value of key, or fallback when the key is not there (Python get).
FUNCTION Dict_Get$ (d AS Dict, keyName AS STRING, OPTIONAL fallback AS STRING = "")
    DIM p AS LONG
    p = List_IndexOf&(d.KeyList, keyName)
    IF p >= 0 THEN Dict_Get$ = List_Get$(d.ValueList, p) ELSE Dict_Get$ = fallback
END FUNCTION

' -1 when key is there (Python "in").
FUNCTION Dict_Has& (d AS Dict, keyName AS STRING)
    Dict_Has& = List_Has&(d.KeyList, keyName)
END FUNCTION

' Removes key and its value.
SUB Dict_Remove (d AS Dict, keyName AS STRING)
    DIM p AS LONG
    p = List_IndexOf&(d.KeyList, keyName)
    IF p < 0 THEN EXIT SUB
    List_RemoveAt d.KeyList, p
    List_RemoveAt d.ValueList, p
END SUB

' Number of keys.
FUNCTION Dict_Count& (d AS Dict)
    Dict_Count& = List_Count&(d.KeyList)
END FUNCTION

' Key number index (0-based, in the order they were added).
FUNCTION Dict_Key$ (d AS Dict, index AS LONG)
    Dict_Key$ = List_Get$(d.KeyList, index)
END FUNCTION

' Value number index (0-based, in the order the keys were added).
FUNCTION Dict_Value$ (d AS Dict, index AS LONG)
    Dict_Value$ = List_Get$(d.ValueList, index)
END FUNCTION

' Removes every key.
SUB Dict_Clear (d AS Dict)
    List_Clear d.KeyList
    List_Clear d.ValueList
END SUB

' Gives the memory back.
SUB Dict_Free (d AS Dict)
    List_Free d.KeyList
    List_Free d.ValueList
END SUB

'------------------------------------------------------------------------- Set

' Adds item when it is not there yet.
SUB Set_Add (s AS Set, item AS STRING)
    IF List_Has&(s.Items, item) = 0 THEN List_Add s.Items, item
END SUB

' -1 when item is there.
FUNCTION Set_Has& (s AS Set, item AS STRING)
    Set_Has& = List_Has&(s.Items, item)
END FUNCTION

' Removes item.
SUB Set_Remove (s AS Set, item AS STRING)
    List_Remove s.Items, item
END SUB

' Number of items.
FUNCTION Set_Count& (s AS Set)
    Set_Count& = List_Count&(s.Items)
END FUNCTION

' The items, in the order they were added, with sep between them.
FUNCTION Set_Join$ (s AS Set, OPTIONAL sep AS STRING = ", ")
    Set_Join$ = List_Join$(s.Items, sep)
END FUNCTION

' Removes every item.
SUB Set_Clear (s AS Set)
    List_Clear s.Items
END SUB

' Gives the memory back.
SUB Set_Free (s AS Set)
    List_Free s.Items
END SUB
