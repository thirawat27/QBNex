' ============================================================================
' QBNex Standard Library - Collections: Dictionary
' ============================================================================

'$IMPORT:'collections.list'

TYPE QBNex_Dictionary
    Keys AS QBNex_List
    Values AS QBNex_List
END TYPE

SUB Dict_Init (dictRef AS QBNex_Dictionary)
    List_Init dictRef.Keys
    List_Init dictRef.Values
END SUB

SUB Dict_Set (dictRef AS QBNex_Dictionary, keyText AS STRING, valueText AS STRING)
    DIM index AS LONG

    index = List_IndexOf&(dictRef.Keys, keyText)
    IF index >= 0 THEN
        List_Set dictRef.Values, index, valueText
    ELSE
        List_Add dictRef.Keys, keyText
        List_Add dictRef.Values, valueText
    END IF
END SUB

FUNCTION Dict_Get$ (dictRef AS QBNex_Dictionary, keyText AS STRING, defaultValue AS STRING)
    DIM index AS LONG

    index = List_IndexOf&(dictRef.Keys, keyText)
    IF index >= 0 THEN
        Dict_Get = List_Get$(dictRef.Values, index)
    ELSE
        Dict_Get = defaultValue
    END IF
END FUNCTION

FUNCTION Dict_Has& (dictRef AS QBNex_Dictionary, keyText AS STRING)
    IF List_IndexOf&(dictRef.Keys, keyText) >= 0 THEN Dict_Has = -1
END FUNCTION

SUB Dict_Remove (dictRef AS QBNex_Dictionary, keyText AS STRING)
    DIM index AS LONG

    index = List_IndexOf&(dictRef.Keys, keyText)
    IF index < 0 THEN EXIT SUB
    List_RemoveAt dictRef.Keys, index
    List_RemoveAt dictRef.Values, index
END SUB

FUNCTION Dict_Count& (dictRef AS QBNex_Dictionary)
    Dict_Count = dictRef.Keys.Count
END FUNCTION

FUNCTION Dict_KeyAt$ (dictRef AS QBNex_Dictionary, itemIndex AS LONG)
    Dict_KeyAt = List_Get$(dictRef.Keys, itemIndex)
END FUNCTION

FUNCTION Dict_ValueAt$ (dictRef AS QBNex_Dictionary, itemIndex AS LONG)
    Dict_ValueAt = List_Get$(dictRef.Values, itemIndex)
END FUNCTION

SUB Dict_Clear (dictRef AS QBNex_Dictionary)
    List_Clear dictRef.Keys
    List_Clear dictRef.Values
END SUB

SUB Dict_Free (dictRef AS QBNex_Dictionary)
    List_Free dictRef.Keys
    List_Free dictRef.Values
END SUB
