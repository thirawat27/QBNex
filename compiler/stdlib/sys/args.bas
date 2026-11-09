' ============================================================================
' QBNex Standard Library - System: Arguments
' ============================================================================

FUNCTION Args_Count& ()
    Args_Count = _COMMANDCOUNT
END FUNCTION

FUNCTION Args_Get$ (argIndex AS LONG, defaultValue AS STRING)
    IF argIndex < 1 OR argIndex > _COMMANDCOUNT THEN
        Args_Get = defaultValue
    ELSE
        Args_Get = COMMAND$(argIndex)
    END IF
END FUNCTION

FUNCTION Args_Program$ ()
    Args_Program = COMMAND$(0)
END FUNCTION

FUNCTION Args_All$ ()
    DIM argIndex AS LONG
    DIM resultText AS STRING

    FOR argIndex = 1 TO _COMMANDCOUNT
        IF argIndex > 1 THEN resultText = resultText + " "
        resultText = resultText + COMMAND$(argIndex)
    NEXT
    Args_All = resultText
END FUNCTION
