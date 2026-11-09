' ============================================================================
' QBNex Standard Library - I/O: CSV Helpers
' ============================================================================

FUNCTION CSV_Escape$ (valueText AS STRING)
    DIM index AS LONG
    DIM currentChar AS STRING
    DIM escapedValue AS STRING
    DIM mustQuote AS LONG

    FOR index = 1 TO LEN(valueText)
        currentChar = MID$(valueText, index, 1)
        IF currentChar = CHR$(34) THEN
            escapedValue = escapedValue + CHR$(34) + CHR$(34)
            mustQuote = -1
        ELSE
            escapedValue = escapedValue + currentChar
            IF currentChar = "," OR currentChar = CHR$(13) OR currentChar = CHR$(10) THEN mustQuote = -1
        END IF
    NEXT

    IF INSTR(valueText, CHR$(34)) THEN mustQuote = -1
    IF mustQuote THEN
        CSV_Escape = CHR$(34) + escapedValue + CHR$(34)
    ELSE
        CSV_Escape = escapedValue
    END IF
END FUNCTION

FUNCTION CSV_Row2$ (valueA AS STRING, valueB AS STRING)
    CSV_Row2 = CSV_Escape$(valueA) + "," + CSV_Escape$(valueB)
END FUNCTION

FUNCTION CSV_Row3$ (valueA AS STRING, valueB AS STRING, valueC AS STRING)
    CSV_Row3 = CSV_Escape$(valueA) + "," + CSV_Escape$(valueB) + "," + CSV_Escape$(valueC)
END FUNCTION

FUNCTION CSV_Row4$ (valueA AS STRING, valueB AS STRING, valueC AS STRING, valueD AS STRING)
    CSV_Row4 = CSV_Escape$(valueA) + "," + CSV_Escape$(valueB) + "," + CSV_Escape$(valueC) + "," + CSV_Escape$(valueD)
END FUNCTION
