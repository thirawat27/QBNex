' ============================================================================
' QBNex Standard Library - Strings: Text Utilities
' ============================================================================

FUNCTION Text_Repeat$ (valueText AS STRING, repeatCount AS LONG)
    DIM index AS LONG
    DIM resultText AS STRING

    IF repeatCount <= 0 THEN
        Text_Repeat = ""
        EXIT FUNCTION
    END IF
    FOR index = 1 TO repeatCount
        resultText = resultText + valueText
    NEXT
    Text_Repeat = resultText
END FUNCTION

FUNCTION Text_StartsWith& (valueText AS STRING, prefixText AS STRING)
    IF LEN(prefixText) = 0 THEN
        Text_StartsWith = -1
        EXIT FUNCTION
    END IF
    IF LEN(valueText) < LEN(prefixText) THEN
        Text_StartsWith = 0
        EXIT FUNCTION
    END IF
    IF LEFT$(valueText, LEN(prefixText)) = prefixText THEN
        Text_StartsWith = -1
    ELSE
        Text_StartsWith = 0
    END IF
END FUNCTION

FUNCTION Text_EndsWith& (valueText AS STRING, suffixText AS STRING)
    IF LEN(suffixText) = 0 THEN
        Text_EndsWith = -1
        EXIT FUNCTION
    END IF
    IF LEN(valueText) < LEN(suffixText) THEN
        Text_EndsWith = 0
        EXIT FUNCTION
    END IF
    IF RIGHT$(valueText, LEN(suffixText)) = suffixText THEN
        Text_EndsWith = -1
    ELSE
        Text_EndsWith = 0
    END IF
END FUNCTION

FUNCTION Text_Contains& (valueText AS STRING, searchText AS STRING)
    IF INSTR(valueText, searchText) <> 0 THEN
        Text_Contains = -1
    ELSE
        Text_Contains = 0
    END IF
END FUNCTION

FUNCTION Text_PadLeft$ (valueText AS STRING, totalWidth AS LONG, padText AS STRING)
    DIM resultText AS STRING
    DIM deficit AS LONG

    resultText = valueText
    IF LEN(padText) = 0 THEN padText = " "
    deficit = totalWidth - LEN(resultText)
    IF deficit <= 0 THEN
        Text_PadLeft = resultText
        EXIT FUNCTION
    END IF
    Text_PadLeft = Text_Repeat$(padText, deficit) + resultText
END FUNCTION

FUNCTION Text_PadRight$ (valueText AS STRING, totalWidth AS LONG, padText AS STRING)
    DIM resultText AS STRING
    DIM deficit AS LONG

    resultText = valueText
    IF LEN(padText) = 0 THEN padText = " "
    deficit = totalWidth - LEN(resultText)
    IF deficit <= 0 THEN
        Text_PadRight = resultText
        EXIT FUNCTION
    END IF
    Text_PadRight = resultText + Text_Repeat$(padText, deficit)
END FUNCTION
