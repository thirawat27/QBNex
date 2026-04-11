' ============================================================================
' QBNex Standard Library - Collections: Dictionary (Hash Map)
' ============================================================================
' Implements a hash-table key-value map with collision resolution
' Uses DJB hash with 256 buckets and linked-list chaining
' ============================================================================

' Dictionary structure
TYPE QBNex_Dict
    Handle AS LONG
    Count AS LONG
END TYPE

' Dictionary entry
TYPE QBNex_DictEntry
    KEY AS STRING * 128
    Value AS STRING * 256
    NextIndex AS LONG
    InUse AS LONG
END TYPE

' Global storage (256 dictionaries × 1024 entries each)
DIM SHARED QBNEX_DictBuckets(1 TO 256, 0 TO 255) AS LONG
DIM SHARED QBNEX_DictEntries(1 TO 256, 1 TO 1024) AS QBNex_DictEntry
DIM SHARED QBNEX_DictUsed(1 TO 256) AS LONG
DIM SHARED QBNEX_DictCount(1 TO 256) AS LONG
DIM SHARED QBNEX_DictNextFree(1 TO 256) AS LONG

' ============================================================================
' FUNCTION: Dict_Hash
' DJB hash function
' ============================================================================
FUNCTION Dict_Hash& (KEY AS STRING)
    DIM hash AS LONG
    DIM i AS LONG
    
    hash = 5381
    FOR i = 1 TO LEN(KEY)
        hash = ((hash * 33) XOR ASC(MID$(KEY, i, 1))) AND &H7FFFFFFF
    NEXT i
    
    Dict_Hash = hash MOD 256
END FUNCTION

' ============================================================================
' SUB: Dict_Init
' Initialize a new dictionary
' ============================================================================
SUB Dict_Init (dict AS QBNex_Dict)
    DIM i AS LONG
    DIM j AS LONG
    
    ' Find free pool slot
    FOR i = 1 TO 256
        IF QBNEX_DictUsed(i) = 0 THEN
            dict.Handle = i
            dict.Count = 0
            QBNEX_DictUsed(i) = -1
            QBNEX_DictCount(i) = 0
            QBNEX_DictNextFree(i) = 1
            
            ' Clear buckets
            FOR j = 0 TO 255
                QBNEX_DictBuckets(i, j) = 0
            NEXT j
            
            ' Clear entries
            FOR j = 1 TO 1024
                QBNEX_DictEntries(i, j).InUse = 0
                QBNEX_DictEntries(i, j).NextIndex = 0
            NEXT j
            
            EXIT SUB
        END IF
    NEXT i
    
    PRINT "ERROR: Dictionary pool exhausted (max 256 dictionaries)"
END SUB

' ============================================================================
' SUB: Dict_Set
' Set a key-value pair
' ============================================================================
SUB Dict_Set (dict AS QBNex_Dict, KEY AS STRING, value AS STRING)
    DIM h AS LONG
    DIM bucket AS LONG
    DIM entryIdx AS LONG
    DIM prevIdx AS LONG
    
    h = dict.Handle
    IF h < 1 OR h > 256 THEN EXIT SUB
    
    bucket = Dict_Hash(KEY)
    entryIdx = QBNEX_DictBuckets(h, bucket)
    
    ' Check if key exists
    DO WHILE entryIdx > 0
        IF RTRIM$(QBNEX_DictEntries(h, entryIdx).KEY) = KEY THEN
            ' Update existing
            QBNEX_DictEntries(h, entryIdx).Value = value
            EXIT SUB
        END IF
        prevIdx = entryIdx
        entryIdx = QBNEX_DictEntries(h, entryIdx).NextIndex
    LOOP
    
    ' Add new entry
    IF QBNEX_DictNextFree(h) > 1024 THEN
        PRINT "ERROR: Dictionary capacity exceeded (max 1024 entries)"
        EXIT SUB
    END IF
    
    entryIdx = QBNEX_DictNextFree(h)
    QBNEX_DictNextFree(h) = QBNEX_DictNextFree(h) + 1
    
    QBNEX_DictEntries(h, entryIdx).KEY = KEY
    QBNEX_DictEntries(h, entryIdx).Value = value
    QBNEX_DictEntries(h, entryIdx).InUse = -1
    QBNEX_DictEntries(h, entryIdx).NextIndex = 0
    
    IF QBNEX_DictBuckets(h, bucket) = 0 THEN
        QBNEX_DictBuckets(h, bucket) = entryIdx
    ELSE
        QBNEX_DictEntries(h, prevIdx).NextIndex = entryIdx
    END IF
    
    QBNEX_DictCount(h) = QBNEX_DictCount(h) + 1
    dict.Count = QBNEX_DictCount(h)
END SUB

' ============================================================================
' FUNCTION: Dict_Get
' Get value for a key (returns empty string if not found)
' ============================================================================
FUNCTION Dict_Get$ (dict AS QBNex_Dict, KEY AS STRING)
    DIM h AS LONG
    DIM bucket AS LONG
    DIM entryIdx AS LONG
    
    h = dict.Handle
    IF h < 1 OR h > 256 THEN
        Dict_Get = ""
        EXIT FUNCTION
    END IF
    
    bucket = Dict_Hash(KEY)
    entryIdx = QBNEX_DictBuckets(h, bucket)
    
    DO WHILE entryIdx > 0
        IF RTRIM$(QBNEX_DictEntries(h, entryIdx).KEY) = KEY THEN
            Dict_Get = RTRIM$(QBNEX_DictEntries(h, entryIdx).Value)
            EXIT FUNCTION
        END IF
        entryIdx = QBNEX_DictEntries(h, entryIdx).NextIndex
    LOOP
    
    Dict_Get = ""
END FUNCTION

' ============================================================================
' FUNCTION: Dict_Has
' Check if dictionary contains a key
' ============================================================================
FUNCTION Dict_Has& (dict AS QBNex_Dict, KEY AS STRING)
    DIM h AS LONG
    DIM bucket AS LONG
    DIM entryIdx AS LONG
    
    h = dict.Handle
    IF h < 1 OR h > 256 THEN
        Dict_Has = 0
        EXIT FUNCTION
    END IF
    
    bucket = Dict_Hash(KEY)
    entryIdx = QBNEX_DictBuckets(h, bucket)
    
    DO WHILE entryIdx > 0
        IF RTRIM$(QBNEX_DictEntries(h, entryIdx).KEY) = KEY THEN
            Dict_Has = -1
            EXIT FUNCTION
        END IF
        entryIdx = QBNEX_DictEntries(h, entryIdx).NextIndex
    LOOP
    
    Dict_Has = 0
END FUNCTION

' ============================================================================
' FUNCTION: Dict_GetOrDefault
' Get value or return default if key not found
' ============================================================================
FUNCTION Dict_GetOrDefault$ (dict AS QBNex_Dict, KEY AS STRING, defaultValue AS STRING)
    IF Dict_Has(dict, KEY) THEN
        Dict_GetOrDefault = Dict_Get(dict, KEY)
    ELSE
        Dict_GetOrDefault = defaultValue
    END IF
END FUNCTION

' ============================================================================
' SUB: Dict_Clear
' Remove all entries
' ============================================================================
SUB Dict_Clear (dict AS QBNex_Dict)
    DIM h AS LONG
    DIM i AS LONG
    
    h = dict.Handle
    IF h < 1 OR h > 256 THEN EXIT SUB
    
    FOR i = 0 TO 255
        QBNEX_DictBuckets(h, i) = 0
    NEXT i
    
    FOR i = 1 TO 1024
        QBNEX_DictEntries(h, i).InUse = 0
        QBNEX_DictEntries(h, i).NextIndex = 0
    NEXT i
    
    QBNEX_DictCount(h) = 0
    QBNEX_DictNextFree(h) = 1
    dict.Count = 0
END SUB

' ============================================================================
' SUB: Dict_Free
' Free the dictionary
' ============================================================================
SUB Dict_Free (dict AS QBNex_Dict)
    DIM h AS LONG
    h = dict.Handle
    IF h < 1 OR h > 256 THEN EXIT SUB
    
    Dict_Clear dict
    QBNEX_DictUsed(h) = 0
    dict.Handle = 0
END SUB
