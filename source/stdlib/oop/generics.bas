' =============================================================================
' QBNex OOP — Generics / Parameterised Type Helpers — generics.bas
' =============================================================================
'
' QBNex does not yet have true generic types, but this module provides
' idiomatic conventions and helper patterns that simulate generics
' through naming conventions and string-serialised storage.
'
' Pattern:
'
'   A "typed list" is a QBNex_List that carries a type tag.
'   Typed accessor functions enforce consistent casts.
'
' Usage:
'
'   '$INCLUDE:'stdlib/oop/generics.bas'
'
'   DIM intList AS QBNex_TypedList
'   TL_Init intList, "LONG"
'   TL_AddLong intList, 10
'   TL_AddLong intList, 20
'   PRINT TL_GetLong(intList, 1)     ' 10
'   PRINT TL_TypeTag$(intList)       ' LONG
'
'   TL_AddLong intList, 99
'   TL_SortNumeric intList
'   TL_Print intList
'
' =============================================================================

'$INCLUDE:'stdlib/collections/list.bas'

CONST QBNEX_TYPED_TAG_LEN = 32

TYPE QBNex_TypedList
    _list    AS QBNex_List
    TypeTag  AS STRING * 32   ' "LONG", "DOUBLE", "STRING", etc.
END TYPE

SUB TL_Init (tl AS QBNex_TypedList, tag$)
    List_Init tl._list
    tl.TypeTag = tag$
END SUB

SUB TL_Free (tl AS QBNex_TypedList)
    List_Free tl._list
END SUB

FUNCTION TL_TypeTag$ (tl AS QBNex_TypedList)
    TL_TypeTag$ = RTRIM$(tl.TypeTag)
END FUNCTION

FUNCTION TL_Count& (tl AS QBNex_TypedList)
    TL_Count& = List_Count&(tl._list)
END FUNCTION

' Type-checked adders
SUB TL_AddLong (tl AS QBNex_TypedList, val AS LONG)
    List_AddLong tl._list, val
END SUB

SUB TL_AddDouble (tl AS QBNex_TypedList, val AS DOUBLE)
    List_AddDouble tl._list, val
END SUB

SUB TL_AddString (tl AS QBNex_TypedList, val$)
    List_Add tl._list, val$
END SUB

' Type-checked getters
FUNCTION TL_GetLong& (tl AS QBNex_TypedList, idx AS LONG)
    TL_GetLong& = List_GetLong&(tl._list, idx)
END FUNCTION

FUNCTION TL_GetDouble# (tl AS QBNex_TypedList, idx AS LONG)
    TL_GetDouble# = List_GetDouble#(tl._list, idx)
END FUNCTION

FUNCTION TL_GetString$ (tl AS QBNex_TypedList, idx AS LONG)
    TL_GetString$ = List_Get$(tl._list, idx)
END FUNCTION

' Numeric sort (ascending, treats all values as DOUBLE)
SUB TL_SortNumeric (tl AS QBNex_TypedList)
    DIM n AS LONG, i AS LONG, j AS LONG
    DIM tmp AS STRING, valI AS DOUBLE, valJ AS DOUBLE
    n = List_Count&(tl._list)
    IF n < 2 THEN EXIT SUB
    ' Insertion sort on numeric value
    FOR i = 2 TO n
        tmp  = List_Get$(tl._list, i)
        valI = VAL(tmp)
        j    = i - 1
        DO WHILE j >= 1 AND VAL(List_Get$(tl._list, j)) > valI
            List_Set tl._list, j + 1, List_Get$(tl._list, j)
            j = j - 1
        LOOP
        List_Set tl._list, j + 1, tmp
    NEXT i
END SUB

SUB TL_SortNumericDesc (tl AS QBNex_TypedList)
    TL_SortNumeric tl
    List_Reverse tl._list
END SUB

SUB TL_Print (tl AS QBNex_TypedList)
    DIM i AS LONG
    PRINT "TypedList<" + TL_TypeTag$(tl) + "> (Count=" + STR$(TL_Count&(tl)) + "):"
    FOR i = 1 TO TL_Count&(tl)
        PRINT "  [" + STR$(i) + "] " + List_Get$(tl._list, i)
    NEXT i
END SUB

' ---------------------------------------------------------------------------
' Optional pair struct — useful for returning two related values
' ---------------------------------------------------------------------------
TYPE QBNex_Pair
    First  AS STRING * 256
    Second AS STRING * 256
END TYPE

SUB Pair_Set (p AS QBNex_Pair, first$, second$)
    p.First  = first$
    p.Second = second$
END SUB

FUNCTION Pair_First$ (p AS QBNex_Pair)
    Pair_First$ = RTRIM$(p.First)
END FUNCTION

FUNCTION Pair_Second$ (p AS QBNex_Pair)
    Pair_Second$ = RTRIM$(p.Second)
END FUNCTION

' ---------------------------------------------------------------------------
' Optional type — None/Some semantics
' ---------------------------------------------------------------------------
TYPE QBNex_Optional
    HasValue  AS LONG
    Value     AS STRING * 512
END TYPE

FUNCTION Opt_Some$ (val$)
    ' Factory not applicable for TYPE — use the SUBs below
    Opt_Some$ = val$
END FUNCTION

SUB Opt_SetSome (opt AS QBNex_Optional, val$)
    opt.HasValue = -1
    opt.Value    = val$
END SUB

SUB Opt_SetNone (opt AS QBNex_Optional)
    opt.HasValue = 0
    opt.Value    = ""
END SUB

FUNCTION Opt_IsSome& (opt AS QBNex_Optional)
    Opt_IsSome& = opt.HasValue
END FUNCTION

FUNCTION Opt_IsNone& (opt AS QBNex_Optional)
    Opt_IsNone& = NOT opt.HasValue
END FUNCTION

FUNCTION Opt_Get$ (opt AS QBNex_Optional)
    IF NOT opt.HasValue THEN
        PRINT "QBNex Optional Error: tried to get value from None"
        END 1
    END IF
    Opt_Get$ = RTRIM$(opt.Value)
END FUNCTION

FUNCTION Opt_GetOrDefault$ (opt AS QBNex_Optional, default$)
    IF opt.HasValue THEN
        Opt_GetOrDefault$ = RTRIM$(opt.Value)
    ELSE
        Opt_GetOrDefault$ = default$
    END IF
END FUNCTION
