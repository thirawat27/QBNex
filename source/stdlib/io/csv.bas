' ============================================================================
' QBNex Standard Library - I/O: CSV Reader/Writer
' ============================================================================
' RFC 4180 compliant CSV handling
' ============================================================================

'$INCLUDE:'../collections/list.bas'

TYPE QBNex_CsvWriter
    Fields AS QBNex_List
    FilePath AS STRING * 256
END TYPE

' ============================================================================
' SUB: CSV_WriterInit
' Initialize CSV writer
' ============================================================================
SUB CSV_WriterInit (writer AS QBNex_CsvWriter, filePath AS STRING)
    List_Init writer.Fields
    writer.FilePath = filePath
END SUB

' ============================================================================
' SUB: CSV_AddField
' Add a field to current row
' ============================================================================
SUB CSV_AddField (writer AS QBNex_CsvWriter, FIELD AS STRING)
    DIM escaped AS STRING
    DIM needsQuotes AS LONG
    DIM i AS LONG
    
    needsQuotes = 0
    escaped = ""
    
    ' Check if field needs quoting
    IF INSTR(FIELD, ",") > 0 OR INSTR(FIELD, CHR$(34)) > 0 OR _
    INSTR(FIELD, CHR$(13)) > 0 OR INSTR(FIELD, CHR$(10)) > 0 THEN
    needsQuotes = -1
END IF
    
' Escape quotes
FOR i = 1 TO LEN(FIELD)
    IF MID$(FIELD, i, 1) = CHR$(34) THEN
        escaped = escaped + CHR$(34) + CHR$(34)
    ELSE
        escaped = escaped + MID$(FIELD, i, 1)
    END IF
NEXT i
    
IF needsQuotes THEN
    List_Add writer.Fields, CHR$(34) + escaped + CHR$(34)
ELSE
    List_Add writer.Fields, escaped
END IF
END SUB

' ============================================================================
' SUB: CSV_WriteRow
' Write current row to file and clear fields
' ============================================================================
SUB CSV_WriteRow (writer AS QBNex_CsvWriter)
    DIM row AS STRING
    DIM i AS LONG
    DIM fileNum AS LONG
    
    row = ""
    FOR i = 0 TO writer.Fields.Count - 1
        IF i > 0 THEN row = row + ","
        row = row + List_Get(writer.Fields, i)
    NEXT i
    
    fileNum = FREEFILE
    OPEN RTRIM$(writer.FilePath) FOR APPEND AS #fileNum
    PRINT #fileNum, row
    CLOSE #fileNum
    
    List_Clear writer.Fields
END SUB

' ============================================================================
' FUNCTION: CSV_ParseLine
' Parse a CSV line into fields
' ============================================================================
SUB CSV_ParseLine (LINE AS STRING, fields AS QBNex_List)
    DIM i AS LONG
    DIM inQuotes AS LONG
    DIM FIELD AS STRING
    DIM c AS STRING
    DIM nextC AS STRING
    
    List_Clear fields
    FIELD = ""
    inQuotes = 0
    i = 1
    
    DO WHILE i <= LEN(LINE)
        c = MID$(LINE, i, 1)
        IF i < LEN(LINE) THEN nextC = MID$(LINE, i + 1, 1) ELSE nextC = ""
        
        IF inQuotes THEN
            IF c = CHR$(34) THEN
                IF nextC = CHR$(34) THEN
                    ' Escaped quote
                    FIELD = FIELD + CHR$(34)
                    i = i + 2
                ELSE
                    ' End of quoted field
                    inQuotes = 0
                    i = i + 1
                END IF
            ELSE
                FIELD = FIELD + c
                i = i + 1
            END IF
        ELSE
            IF c = CHR$(34) THEN
                inQuotes = -1
                i = i + 1
            ELSEIF c = "," THEN
                List_Add fields, FIELD
                FIELD = ""
                i = i + 1
            ELSE
                FIELD = FIELD + c
                i = i + 1
            END IF
        END IF
    LOOP
    
    ' Add last field
    List_Add fields, FIELD
END SUB

' ============================================================================
' FUNCTION: CSV_ReadRow
' Read a specific row from CSV file (1-based)
' ============================================================================
FUNCTION CSV_ReadRow$ (filePath AS STRING, rowNum AS LONG)
    DIM fileNum AS LONG
    DIM LINE AS STRING
    DIM currentRow AS LONG
    
    fileNum = FREEFILE
    OPEN filePath FOR INPUT AS #fileNum
    
    currentRow = 0
    DO WHILE NOT EOF(fileNum)
        LINE INPUT #fileNum, LINE
        currentRow = currentRow + 1
        IF currentRow = rowNum THEN
            CLOSE #fileNum
            CSV_ReadRow = LINE
            EXIT FUNCTION
        END IF
    LOOP
    
    CLOSE #fileNum
    CSV_ReadRow = ""
END FUNCTION

' ============================================================================
' FUNCTION: CSV_RowCount
' Count rows in CSV file
' ============================================================================
FUNCTION CSV_RowCount& (filePath AS STRING)
    DIM fileNum AS LONG
    DIM LINE AS STRING
    DIM count AS LONG
    
    fileNum = FREEFILE
    OPEN filePath FOR INPUT AS #fileNum
    
    count = 0
    DO WHILE NOT EOF(fileNum)
        LINE INPUT #fileNum, LINE
        count = count + 1
    LOOP
    
    CLOSE #fileNum
    CSV_RowCount = count
END FUNCTION
