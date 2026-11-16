'===============================================================================
' QBNex Standard Library - csv ('$IMPORT:'csv')
'===============================================================================
' One CSV row (RFC 4180) to and from a List, like Python's csv module.
'
'   DIM row AS List
'   row.Add "plain": row.Add "a,b"
'   PRINT Csv_Row(row)                 plain,"a,b"
'   Csv_Parse row, text$                the fields of a row back in a List
'===============================================================================

'$IMPORT:'collections'

' A cell written for CSV: quoted when it has sep, a quote or a line break.
FUNCTION Csv_Quote$ (cell AS STRING, OPTIONAL sep AS STRING = ",")
    IF INSTR(cell, sep) OR INSTR(cell, CHR$(34)) OR INSTR(cell, CHR$(10)) OR INSTR(cell, CHR$(13)) THEN
        Csv_Quote$ = CHR$(34) + Csv_Doubled$(cell) + CHR$(34)
    ELSE
        Csv_Quote$ = cell
    END IF
END FUNCTION

FUNCTION Csv_Doubled$ (cell AS STRING)
    DIM i AS LONG, c AS STRING, r AS STRING
    FOR i = 1 TO LEN(cell)
        c = MID$(cell, i, 1)
        IF c = CHR$(34) THEN r = r + c
        r = r + c
    NEXT
    Csv_Doubled$ = r
END FUNCTION

' The items of row as one CSV line.
FUNCTION Csv_Row$ (row AS List, OPTIONAL sep AS STRING = ",")
    DIM i AS LONG, r AS STRING
    FOR i = 0 TO List_Count&(row) - 1
        IF i THEN r = r + sep
        r = r + Csv_Quote$(List_Get$(row, i), sep)
    NEXT
    Csv_Row$ = r
END FUNCTION

' Replaces the items of row with the fields of one CSV line.
SUB Csv_Parse (row AS List, text AS STRING, OPTIONAL sep AS STRING = ",")
    DIM i AS LONG, c AS STRING, cell AS STRING, quoted AS LONG
    List_Clear row
    i = 1
    DO WHILE i <= LEN(text)
        c = MID$(text, i, 1)
        IF quoted THEN
            IF c = CHR$(34) THEN
                IF MID$(text, i + 1, 1) = CHR$(34) THEN cell = cell + c: i = i + 1 ELSE quoted = 0
            ELSE
                cell = cell + c
            END IF
        ELSEIF c = CHR$(34) THEN
            quoted = -1
        ELSEIF c = sep THEN
            List_Add row, cell
            cell = ""
        ELSE
            cell = cell + c
        END IF
        i = i + 1
    LOOP
    List_Add row, cell
END SUB
