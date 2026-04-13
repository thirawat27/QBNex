' ============================================================================
' QBNex Data Smoke Test
' ============================================================================

'$IMPORT:'qbnex'

SUB DataSmoke ()
    DIM metadata AS QBNex_Dictionary
    DIM state AS QBNex_Result

    Dict_Init metadata
    Dict_Set metadata, "name", "QBNex"
    Dict_Set metadata, "kind", "compiler"

    PRINT Dict_Get$(metadata, "name", "")
    PRINT Json_Object3$("name", Json_String$(Dict_Get$(metadata, "name", "")), "kind", Json_String$(Dict_Get$(metadata, "kind", "")), "status", Json_String$("ok"))

    Result_Ok state, "ready"
    PRINT Result_Value$(state, "")

    Dict_Free metadata
END SUB
