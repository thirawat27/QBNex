' =============================================================================
' QBNex Collections — Key-Value Dictionary (hash-table backed) — dictionary.bas
' =============================================================================
'
' A string-keyed dictionary (map) with O(1) average-case lookup.
' Keys and values are both stored as strings (use VAL/STR$ for numerics).
'
' Usage:
'
'   '$INCLUDE:'stdlib/collections/dictionary.bas'
'
'   DIM d AS QBNex_Dict
'   Dict_Init d
'
'   Dict_Set d, "name",  "QBNex"
'   Dict_Set d, "major", "1"
'   Dict_Set d, "minor", "0"
'
'   IF Dict_Has(d, "name") THEN
'       PRINT "name = "; Dict_Get$(d, "name")   ' QBNex
'   END IF
'
'   Dict_Delete d, "minor"
'   PRINT "Count = "; Dict_Count(d)             '  2
'
'   Dict_Keys d, keyList  ' fill a QBNex_List with all keys
'
'   Dict_Free d
'
' =============================================================================

'$INCLUDE:'stdlib/collections/list.bas'   ' for Dict_Keys / Dict_Values

' ---------------------------------------------------------------------------
' Internal constants
' ---------------------------------------------------------------------------
CONST QBNEX_DICT_BUCKETS  = 256   ' power-of-two hash table size (per dict)
CONST QBNEX_DICT_POOL_MAX = 512   ' max simultaneous dictionary instances

' ---------------------------------------------------------------------------
' A single key-value entry
' ---------------------------------------------------------------------------
TYPE QBNex_DictEntry
    KeyStr   AS STRING * 128
    ValueStr AS STRING * 512
    InUse    AS LONG
    NextIdx  AS LONG   ' chaining index (0 = end)
END TYPE

' ---------------------------------------------------------------------------
' Per-dictionary metadata
' ---------------------------------------------------------------------------
TYPE QBNex_Dict
    Count   AS LONG     ' total entries
    _handle AS LONG     ' index into global pool
END TYPE

' ---------------------------------------------------------------------------
' Global storage pool
'
' Each dict instance owns QBNEX_DICT_BUCKETS bucket-head slots and a
' dynamically grown entry chain stored per-handle in two parallel arrays.
' ---------------------------------------------------------------------------

' Bucket heads: BucketHead(handle, bucket) = index into entry chain (0=empty)
DIM SHARED QBNEX_DictBucket(1 TO QBNEX_DICT_POOL_MAX, 0 TO QBNEX_DICT_BUCKETS - 1) AS LONG

' Entry pool per handle — each handle has its own linked list of entries
' We store them in flat arrays indexed by (handle, entryIndex)
CONST QBNEX_DICT_ENTRY_MAX = 2048   ' max entries per dictionary

DIM SHARED QBNEX_DictKey   (1 TO QBNEX_DICT_POOL_MAX, 1 TO QBNEX_DICT_ENTRY_MAX) AS STRING * 128
DIM SHARED QBNEX_DictVal   (1 TO QBNEX_DICT_POOL_MAX, 1 TO QBNEX_DICT_ENTRY_MAX) AS STRING * 512
DIM SHARED QBNEX_DictInUse (1 TO QBNEX_DICT_POOL_MAX, 1 TO QBNEX_DICT_ENTRY_MAX) AS LONG
DIM SHARED QBNEX_DictNext  (1 TO QBNEX_DICT_POOL_MAX, 1 TO QBNEX_DICT_ENTRY_MAX) AS LONG
DIM SHARED QBNEX_DictEC    (1 TO QBNEX_DICT_POOL_MAX) AS LONG   ' entry count (high watermark)

DIM SHARED QBNEX_DictPoolUsed(1 TO QBNEX_DICT_POOL_MAX) AS LONG
DIM SHARED QBNEX_DictPoolCount AS LONG
DIM SHARED QBNEX_DictPoolFree (1 TO QBNEX_DICT_POOL_MAX) AS LONG
DIM SHARED QBNEX_DictPoolFreeN AS LONG
QBNEX_DictPoolCount = 0
QBNEX_DictPoolFreeN = 0

' ---------------------------------------------------------------------------
' PRIVATE: simple DJB-like hash for a string, returns 0..(QBNEX_DICT_BUCKETS-1)
' ---------------------------------------------------------------------------
FUNCTION _Dict_Hash& (key$)
    DIM h AS LONG, i AS LONG
    h = 5381
    FOR i = 1 TO LEN(key$)
        h = ((h * 33) XOR ASC(MID$(key$, i, 1))) AND 16777215
    NEXT i
    _Dict_Hash& = h MOD QBNEX_DICT_BUCKETS
END FUNCTION

' ---------------------------------------------------------------------------
' PRIVATE: allocate a handle from the pool
' ---------------------------------------------------------------------------
FUNCTION _Dict_AllocPool& ()
    DIM h AS LONG, i AS LONG
    IF QBNEX_DictPoolFreeN > 0 THEN
        h = QBNEX_DictPoolFree(QBNEX_DictPoolFreeN)
        QBNEX_DictPoolFreeN = QBNEX_DictPoolFreeN - 1
    ELSE
        QBNEX_DictPoolCount = QBNEX_DictPoolCount + 1
        IF QBNEX_DictPoolCount > QBNEX_DICT_POOL_MAX THEN
            PRINT "QBNex Dict Error: pool overflow"
            END 1
        END IF
        h = QBNEX_DictPoolCount
    END IF
    QBNEX_DictPoolUsed(h) = -1
    QBNEX_DictEC(h) = 0
    ' clear bucket heads
    FOR i = 0 TO QBNEX_DICT_BUCKETS - 1
        QBNEX_DictBucket(h, i) = 0
    NEXT i
    _Dict_AllocPool& = h
END FUNCTION

' ---------------------------------------------------------------------------
' SUB  Dict_Init(d)
' ---------------------------------------------------------------------------
SUB Dict_Init (d AS QBNex_Dict)
    d.Count   = 0
    d._handle = _Dict_AllocPool&()
END SUB

' ---------------------------------------------------------------------------
' SUB  Dict_Free(d)
' ---------------------------------------------------------------------------
SUB Dict_Free (d AS QBNex_Dict)
    IF d._handle < 1 THEN EXIT SUB
    QBNEX_DictPoolUsed(d._handle) = 0
    QBNEX_DictPoolFreeN = QBNEX_DictPoolFreeN + 1
    QBNEX_DictPoolFree(QBNEX_DictPoolFreeN) = d._handle
    d._handle = 0
    d.Count   = 0
END SUB

' ---------------------------------------------------------------------------
' FUNCTION  Dict_Count&(d)
' ---------------------------------------------------------------------------
FUNCTION Dict_Count& (d AS QBNex_Dict)
    Dict_Count& = d.Count
END FUNCTION

' ---------------------------------------------------------------------------
' FUNCTION  Dict_Has&(d, key$)  — returns -1 if key exists
' ---------------------------------------------------------------------------
FUNCTION Dict_Has& (d AS QBNex_Dict, key$)
    DIM h AS LONG, idx AS LONG
    IF d._handle < 1 THEN Dict_Has& = 0: EXIT FUNCTION
    h   = _Dict_Hash&(key$)
    idx = QBNEX_DictBucket(d._handle, h)
    DO WHILE idx > 0
        IF QBNEX_DictInUse(d._handle, idx) AND _
           RTRIM$(QBNEX_DictKey(d._handle, idx)) = key$ THEN
            Dict_Has& = -1: EXIT FUNCTION
        END IF
        idx = QBNEX_DictNext(d._handle, idx)
    LOOP
    Dict_Has& = 0
END FUNCTION

' ---------------------------------------------------------------------------
' SUB  Dict_Set(d, key$, value$)  — insert or update
' ---------------------------------------------------------------------------
SUB Dict_Set (d AS QBNex_Dict, key$, value$)
    DIM h AS LONG, idx AS LONG, handle AS LONG

    IF d._handle < 1 THEN Dict_Init d
    handle = d._handle
    h      = _Dict_Hash&(key$)
    idx    = QBNEX_DictBucket(handle, h)

    ' search existing
    DO WHILE idx > 0
        IF QBNEX_DictInUse(handle, idx) AND RTRIM$(QBNEX_DictKey(handle, idx)) = key$ THEN
            QBNEX_DictVal(handle, idx) = value$
            EXIT SUB
        END IF
        idx = QBNEX_DictNext(handle, idx)
    LOOP

    ' new entry
    QBNEX_DictEC(handle) = QBNEX_DictEC(handle) + 1
    IF QBNEX_DictEC(handle) > QBNEX_DICT_ENTRY_MAX THEN
        PRINT "QBNex Dict Error: entry overflow (max " + STR$(QBNEX_DICT_ENTRY_MAX) + ")"
        END 1
    END IF

    idx = QBNEX_DictEC(handle)
    QBNEX_DictKey   (handle, idx) = key$
    QBNEX_DictVal   (handle, idx) = value$
    QBNEX_DictInUse (handle, idx) = -1
    ' chain into bucket
    QBNEX_DictNext  (handle, idx) = QBNEX_DictBucket(handle, h)
    QBNEX_DictBucket(handle, h)   = idx

    d.Count = d.Count + 1
END SUB

' ---------------------------------------------------------------------------
' SUB  Dict_SetLong(d, key$, value&)
' ---------------------------------------------------------------------------
SUB Dict_SetLong (d AS QBNex_Dict, key$, value AS LONG)
    Dict_Set d, key$, _TRIM$(STR$(value))
END SUB

' ---------------------------------------------------------------------------
' FUNCTION  Dict_Get$(d, key$)  — returns "" if not found
' ---------------------------------------------------------------------------
FUNCTION Dict_Get$ (d AS QBNex_Dict, key$)
    DIM h AS LONG, idx AS LONG, handle AS LONG
    IF d._handle < 1 THEN Dict_Get$ = "": EXIT FUNCTION
    handle = d._handle
    h      = _Dict_Hash&(key$)
    idx    = QBNEX_DictBucket(handle, h)
    DO WHILE idx > 0
        IF QBNEX_DictInUse(handle, idx) AND RTRIM$(QBNEX_DictKey(handle, idx)) = key$ THEN
            Dict_Get$ = RTRIM$(QBNEX_DictVal(handle, idx))
            EXIT FUNCTION
        END IF
        idx = QBNEX_DictNext(handle, idx)
    LOOP
    Dict_Get$ = ""
END FUNCTION

FUNCTION Dict_GetLong& (d AS QBNex_Dict, key$)
    Dict_GetLong& = VAL(Dict_Get$(d, key$))
END FUNCTION

FUNCTION Dict_GetDouble# (d AS QBNex_Dict, key$)
    Dict_GetDouble# = VAL(Dict_Get$(d, key$))
END FUNCTION

' ---------------------------------------------------------------------------
' FUNCTION  Dict_GetOrDefault$(d, key$, default$)
' ---------------------------------------------------------------------------
FUNCTION Dict_GetOrDefault$ (d AS QBNex_Dict, key$, default$)
    IF Dict_Has&(d, key$) THEN
        Dict_GetOrDefault$ = Dict_Get$(d, key$)
    ELSE
        Dict_GetOrDefault$ = default$
    END IF
END FUNCTION

' ---------------------------------------------------------------------------
' SUB  Dict_Delete(d, key$)  — remove a key (marks slot deleted)
' ---------------------------------------------------------------------------
SUB Dict_Delete (d AS QBNex_Dict, key$)
    DIM h AS LONG, idx AS LONG, handle AS LONG
    IF d._handle < 1 THEN EXIT SUB
    handle = d._handle
    h      = _Dict_Hash&(key$)
    idx    = QBNEX_DictBucket(handle, h)
    DO WHILE idx > 0
        IF QBNEX_DictInUse(handle, idx) AND RTRIM$(QBNEX_DictKey(handle, idx)) = key$ THEN
            QBNEX_DictInUse(handle, idx) = 0
            d.Count = d.Count - 1
            EXIT SUB
        END IF
        idx = QBNEX_DictNext(handle, idx)
    LOOP
END SUB

' ---------------------------------------------------------------------------
' SUB  Dict_Clear(d)  — remove all entries
' ---------------------------------------------------------------------------
SUB Dict_Clear (d AS QBNex_Dict)
    DIM i AS LONG, handle AS LONG
    IF d._handle < 1 THEN EXIT SUB
    handle = d._handle
    FOR i = 1 TO QBNEX_DictEC(handle)
        QBNEX_DictInUse(handle, i) = 0
    NEXT i
    FOR i = 0 TO QBNEX_DICT_BUCKETS - 1
        QBNEX_DictBucket(handle, i) = 0
    NEXT i
    QBNEX_DictEC(handle) = 0
    d.Count = 0
END SUB

' ---------------------------------------------------------------------------
' SUB  Dict_Keys(d, lst)   — populate a QBNex_List with all keys
' ---------------------------------------------------------------------------
SUB Dict_Keys (d AS QBNex_Dict, lst AS QBNex_List)
    DIM i AS LONG, handle AS LONG
    List_Clear lst
    IF d._handle < 1 THEN EXIT SUB
    handle = d._handle
    FOR i = 1 TO QBNEX_DictEC(handle)
        IF QBNEX_DictInUse(handle, i) THEN
            List_Add lst, RTRIM$(QBNEX_DictKey(handle, i))
        END IF
    NEXT i
END SUB

' ---------------------------------------------------------------------------
' SUB  Dict_Values(d, lst) — populate a QBNex_List with all values
' ---------------------------------------------------------------------------
SUB Dict_Values (d AS QBNex_Dict, lst AS QBNex_List)
    DIM i AS LONG, handle AS LONG
    List_Clear lst
    IF d._handle < 1 THEN EXIT SUB
    handle = d._handle
    FOR i = 1 TO QBNEX_DictEC(handle)
        IF QBNEX_DictInUse(handle, i) THEN
            List_Add lst, RTRIM$(QBNEX_DictVal(handle, i))
        END IF
    NEXT i
END SUB

' ---------------------------------------------------------------------------
' SUB  Dict_Print(d)  — debug dump
' ---------------------------------------------------------------------------
SUB Dict_Print (d AS QBNex_Dict)
    DIM i AS LONG, handle AS LONG
    PRINT "Dict (Count=" + STR$(d.Count) + "):"
    IF d._handle < 1 THEN EXIT SUB
    handle = d._handle
    FOR i = 1 TO QBNEX_DictEC(handle)
        IF QBNEX_DictInUse(handle, i) THEN
            PRINT "  [" + RTRIM$(QBNEX_DictKey(handle, i)) + "] = " + _
                  RTRIM$(QBNEX_DictVal(handle, i))
        END IF
    NEXT i
END SUB
