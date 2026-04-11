' =============================================================================
' QBNex I/O Library — CSV Reader/Writer — csv.bas
' =============================================================================
'
' Usage:
'
'   '$INCLUDE:'stdlib/io/csv.bas'
'
'   ' Write
'   DIM csv AS QBNex_CsvWriter
'   CSV_WriterInit csv, "output.csv"
'   CSV_WriteRow csv, "Name,Age,City"
'   CSV_WriteField csv, "Alice" : CSV_WriteField csv, "30" : CSV_WriteField csv, "Bangkok"
'   CSV_EndRow csv
'   CSV_WriterClose csv
'
'   ' Read
'   DIM row AS QBNex_List
'   List_Init row
'   CSV_ReadFile "output.csv", row, 1  ' row 1
'   PRINT List_Get$(row, 1)            ' Name
'   List_Free row
'
' =============================================================================

'$INCLUDE:'stdlib/collections/list.bas'

TYPE QBNex_CsvWriter
    _fh       AS LONG    ' file handle
    _filename AS STRING
    _atStart  AS LONG   ' -1 = nothing written on current row yet
END TYPE

' ---------------------------------------------------------------------------
' Writer
' ---------------------------------------------------------------------------
SUB CSV_WriterInit (w AS QBNex_CsvWriter, filename$)
    w._fh       = FREEFILE
    w._filename = filename$
    w._atStart  = -1
    OPEN filename$ FOR OUTPUT AS #w._fh
END SUB

SUB CSV_WriterClose (w AS QBNex_CsvWriter)
    IF w._fh > 0 THEN CLOSE #w._fh: w._fh = 0
END SUB

' Write a complete pre-formatted row (fields already comma-joined)
SUB CSV_WriteRow (w AS QBNex_CsvWriter, row$)
    IF w._fh = 0 THEN EXIT SUB
    PRINT #w._fh, row$
END SUB

' Write a single field (auto-quoted if needed), followed by comma or newline
SUB CSV_WriteField (w AS QBNex_CsvWriter, field$)
    IF w._fh = 0 THEN EXIT SUB
    DIM f AS STRING
    ' Quote if field contains comma, quote, or newline
    IF INSTR(field$, ",") OR INSTR(field$, """") OR _
       INSTR(field$, CHR$(10)) OR INSTR(field$, CHR$(13)) THEN
        f = """" + StrReplace$(field$, """", """""") + """"
    ELSE
        f = field$
    END IF
    IF NOT w._atStart THEN PRINT #w._fh, ",";
    PRINT #w._fh, f;
    w._atStart = 0
END SUB

SUB CSV_EndRow (w AS QBNex_CsvWriter)
    IF w._fh = 0 THEN EXIT SUB
    PRINT #w._fh, ""   ' newline
    w._atStart = -1
END SUB

' ---------------------------------------------------------------------------
' Reader helpers
' ---------------------------------------------------------------------------

' Parse a single CSV line into a List of field strings
SUB CSV_ParseLine (line$, fields AS QBNex_List)
    DIM i AS LONG, n AS LONG, ch AS STRING, field AS STRING
    DIM inQuote AS LONG
    List_Clear fields
    n = LEN(line$)
    field = ""
    inQuote = 0
    FOR i = 1 TO n
        ch = MID$(line$, i, 1)
        IF inQuote THEN
            IF ch = """" THEN
                ' peek: escaped quote?
                IF i + 1 <= n AND MID$(line$, i + 1, 1) = """" THEN
                    field = field + """"
                    i     = i + 1  ' skip next quote (NOTE: FOR var increment still happens)
                ELSE
                    inQuote = 0
                END IF
            ELSE
                field = field + ch
            END IF
        ELSE
            SELECT CASE ch
                CASE ","
                    List_Add fields, field
                    field = ""
                CASE """"
                    inQuote = -1
                CASE ELSE
                    field = field + ch
            END SELECT
        END IF
    NEXT i
    List_Add fields, field   ' last field
END SUB

' Read a specific row (1-based) from a CSV file into a List
SUB CSV_ReadRow (filename$, rowNum AS LONG, fields AS QBNex_List)
    DIM fh AS LONG, curRow AS LONG, line AS STRING
    List_Clear fields
    fh     = FREEFILE
    curRow = 0
    OPEN filename$ FOR INPUT AS #fh
    DO WHILE NOT EOF(fh)
        LINE INPUT #fh, line
        curRow = curRow + 1
        IF curRow = rowNum THEN
            CSV_ParseLine line, fields
            EXIT DO
        END IF
    LOOP
    CLOSE #fh
END SUB

' Read all rows from a CSV file; each row becomes a List stored as JSON in outer List
FUNCTION CSV_RowCount& (filename$)
    DIM fh AS LONG, n AS LONG, line AS STRING
    fh = FREEFILE: n = 0
    OPEN filename$ FOR INPUT AS #fh
    DO WHILE NOT EOF(fh): LINE INPUT #fh, line: n = n + 1: LOOP
    CLOSE #fh
    CSV_RowCount& = n
END FUNCTION
