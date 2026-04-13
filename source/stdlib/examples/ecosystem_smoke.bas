' ============================================================================
' QBNex Ecosystem Smoke Test
' ============================================================================

SUB EcosystemSmoke ()
    DIM instant AS QBNex_Date

    Date_SetNow instant
    PRINT Text_PadRight$("QBNex", 8, ".")
    PRINT Args_Count&
    PRINT CSV_Row3$("name", "score", "status")
    PRINT Math_Clamp#(12#, 0#, 10#)
    PRINT Date_GetFullYear&(instant)
END SUB

'$IMPORT:'strings.text'
'$IMPORT:'sys.args'
'$IMPORT:'sys.datetime'
'$IMPORT:'io.csv'
'$IMPORT:'math.numeric'
