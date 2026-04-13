' ============================================================================
' QBNex Standard Library - Error: Result
' ============================================================================

TYPE QBNex_Result
    Ok AS LONG
    Value AS STRING
    Message AS STRING
END TYPE

SUB Result_Ok (resultRef AS QBNex_Result, valueText AS STRING)
    resultRef.Ok = -1
    resultRef.Value = valueText
    resultRef.Message = ""
END SUB

SUB Result_Fail (resultRef AS QBNex_Result, messageText AS STRING)
    resultRef.Ok = 0
    resultRef.Value = ""
    resultRef.Message = messageText
END SUB

FUNCTION Result_IsOk& (resultRef AS QBNex_Result)
    Result_IsOk = resultRef.Ok
END FUNCTION

FUNCTION Result_Value$ (resultRef AS QBNex_Result, defaultValue AS STRING)
    IF resultRef.Ok THEN
        Result_Value = resultRef.Value
    ELSE
        Result_Value = defaultValue
    END IF
END FUNCTION

FUNCTION Result_Message$ (resultRef AS QBNex_Result)
    Result_Message = resultRef.Message
END FUNCTION
