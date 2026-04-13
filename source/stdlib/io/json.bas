' ============================================================================
' QBNex Standard Library - I/O: JSON Builder Helpers
' ============================================================================

FUNCTION Json_Escape$ (valueText AS STRING)
    DIM index AS LONG
    DIM currentChar AS STRING
    DIM resultText AS STRING

    FOR index = 1 TO LEN(valueText)
        currentChar = MID$(valueText, index, 1)
        SELECT CASE ASC(currentChar)
            CASE 34
                resultText = resultText + CHR$(92) + CHR$(34)
            CASE 92
                resultText = resultText + CHR$(92) + CHR$(92)
            CASE 9
                resultText = resultText + CHR$(92) + "t"
            CASE 10
                resultText = resultText + CHR$(92) + "n"
            CASE 13
                resultText = resultText + CHR$(92) + "r"
            CASE ELSE
                resultText = resultText + currentChar
        END SELECT
    NEXT

    Json_Escape = resultText
END FUNCTION

FUNCTION Json_String$ (valueText AS STRING)
    Json_String = CHR$(34) + Json_Escape$(valueText) + CHR$(34)
END FUNCTION

FUNCTION Json_Pair$ (keyText AS STRING, valueJson AS STRING)
    Json_Pair = Json_String$(keyText) + ":" + valueJson
END FUNCTION

FUNCTION Json_Object2$ (keyA AS STRING, valueAJson AS STRING, keyB AS STRING, valueBJson AS STRING)
    Json_Object2 = "{" + Json_Pair$(keyA, valueAJson) + "," + Json_Pair$(keyB, valueBJson) + "}"
END FUNCTION

FUNCTION Json_Object3$ (keyA AS STRING, valueAJson AS STRING, keyB AS STRING, valueBJson AS STRING, keyC AS STRING, valueCJson AS STRING)
    Json_Object3 = "{" + Json_Pair$(keyA, valueAJson) + "," + Json_Pair$(keyB, valueBJson) + "," + Json_Pair$(keyC, valueCJson) + "}"
END FUNCTION

FUNCTION Json_Array2$ (valueAJson AS STRING, valueBJson AS STRING)
    Json_Array2 = "[" + valueAJson + "," + valueBJson + "]"
END FUNCTION

FUNCTION Json_Array3$ (valueAJson AS STRING, valueBJson AS STRING, valueCJson AS STRING)
    Json_Array3 = "[" + valueAJson + "," + valueBJson + "," + valueCJson + "]"
END FUNCTION
