' ============================================================================
' QBNex Standard Library - OOP Foundation: Generic Types
' ============================================================================
' Provides TypedList, Optional<T>, and Pair generic patterns
' ============================================================================

' TypedList - Type-safe list wrapper
TYPE QBNex_TypedList
    TypeName AS STRING * 32
    Handle AS LONG
END TYPE

' Optional<T> - Nullable value wrapper
TYPE QBNex_Optional
    HasValue AS LONG
    Value AS STRING * 256
END TYPE

' Pair - Key-value pair
TYPE QBNex_Pair
    First AS STRING * 128
    Second AS STRING * 128
END TYPE

' ============================================================================
' TypedList Functions
' ============================================================================

SUB TL_Init (list AS QBNex_TypedList, typeName AS STRING)
    list.TypeName = typeName
    list.Handle = 0
END SUB

SUB TL_AddLong (list AS QBNex_TypedList, value AS LONG)
    ' Implementation requires list.bas integration
END SUB

SUB TL_AddDouble (list AS QBNex_TypedList, value AS DOUBLE)
    ' Implementation requires list.bas integration
END SUB

SUB TL_AddString (list AS QBNex_TypedList, value AS STRING)
    ' Implementation requires list.bas integration
END SUB

FUNCTION TL_GetLong& (list AS QBNex_TypedList, index AS LONG)
    ' Implementation requires list.bas integration
    TL_GetLong = 0
END FUNCTION

FUNCTION TL_GetDouble# (list AS QBNex_TypedList, index AS LONG)
    ' Implementation requires list.bas integration
    TL_GetDouble = 0
END FUNCTION

FUNCTION TL_GetString$ (list AS QBNex_TypedList, index AS LONG)
    ' Implementation requires list.bas integration
    TL_GetString = ""
END FUNCTION

' ============================================================================
' Optional<T> Functions
' ============================================================================

SUB Opt_SetSome (opt AS QBNex_Optional, value AS STRING)
    opt.HasValue = -1
    opt.Value = value
END SUB

SUB Opt_SetNone (opt AS QBNex_Optional)
    opt.HasValue = 0
    opt.Value = ""
END SUB

FUNCTION Opt_IsSome& (opt AS QBNex_Optional)
    Opt_IsSome = opt.HasValue
END FUNCTION

FUNCTION Opt_IsNone& (opt AS QBNex_Optional)
    Opt_IsNone = NOT opt.HasValue
END FUNCTION

FUNCTION Opt_Get$ (opt AS QBNex_Optional)
    IF opt.HasValue THEN
        Opt_Get = RTRIM$(opt.Value)
    ELSE
        PRINT "ERROR: Attempted to get value from None Optional"
        Opt_Get = ""
    END IF
END FUNCTION

FUNCTION Opt_GetOrDefault$ (opt AS QBNex_Optional, defaultValue AS STRING)
    IF opt.HasValue THEN
        Opt_GetOrDefault = RTRIM$(opt.Value)
    ELSE
        Opt_GetOrDefault = defaultValue
    END IF
END FUNCTION

' ============================================================================
' Pair Functions
' ============================================================================

SUB Pair_Set (p AS QBNex_Pair, first AS STRING, second AS STRING)
    p.First = first
    p.Second = second
END SUB

FUNCTION Pair_First$ (p AS QBNex_Pair)
    Pair_First = RTRIM$(p.First)
END FUNCTION

FUNCTION Pair_Second$ (p AS QBNex_Pair)
    Pair_Second = RTRIM$(p.Second)
END FUNCTION

SUB Pair_SetFirst (p AS QBNex_Pair, value AS STRING)
    p.First = value
END SUB

SUB Pair_SetSecond (p AS QBNex_Pair, value AS STRING)
    p.Second = value
END SUB
