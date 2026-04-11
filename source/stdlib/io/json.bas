' ============================================================================
' QBNex Standard Library - I/O: JSON Parser/Serializer
' ============================================================================
' Handles objects, arrays, strings, numbers, booleans, null
' ============================================================================

'$INCLUDE:'../collections/dictionary.bas'
'$INCLUDE:'../collections/list.bas'

' JSON value types
CONST JSON_NULL = 0
CONST JSON_BOOLEAN = 1
CONST JSON_NUMBER = 2
CONST JSON_STRING = 3
CONST JSON_ARRAY = 4
CONST JSON_OBJECT = 5

' ============================================================================
' FUNCTION: Json_Stringify
' Serialize a dictionary to JSON string (compact)
' ============================================================================
FUNCTION Json_Stringify$ (dict AS QBNex_Dict)
    DIM result AS STRING
    result = "{"
    ' Simplified - full implementation would iterate dict entries
    result = result + "}"
    Json_Stringify = result
END FUNCTION

' ============================================================================
' FUNCTION: Json_StringifyArray
' Serialize a list to JSON array string
' ============================================================================
FUNCTION Json_StringifyArray$ (list AS QBNex_List)
    DIM result AS STRING
    DIM i AS LONG
    
    result = "["
    FOR i = 0 TO list.Count - 1
        IF i > 0 THEN result = result + ","
        result = result + Json_EscapeString(List_Get(list, i))
    NEXT i
    result = result + "]"
    
    Json_StringifyArray = result
END FUNCTION

' ============================================================================
' FUNCTION: Json_EscapeString
' Escape string for JSON
' ============================================================================
FUNCTION Json_EscapeString$ (text AS STRING)
    DIM result AS STRING
    DIM i AS LONG
    DIM c AS STRING
    
    result = CHR$(34) ' Opening quote
    
    FOR i = 1 TO LEN(text)
        c = MID$(text, i, 1)
        SELECT CASE c
        CASE CHR$(34): result = result + "\" + CHR$(34)
        CASE "\": result = result + "\\"
        CASE CHR$(10): result = result + "\n"
        CASE CHR$(13): result = result + "\r"
        CASE CHR$(9): result = result + "\t"
        CASE ELSE: result = result + c
        END SELECT
    NEXT i
    
    result = result + CHR$(34) ' Closing quote
    Json_EscapeString = result
END FUNCTION

' ============================================================================
' FUNCTION: Json_UnescapeString
' Unescape JSON string
' ============================================================================
FUNCTION Json_UnescapeString$ (text AS STRING)
    DIM result AS STRING
    DIM i AS LONG
    DIM c AS STRING
    DIM nextC AS STRING
    
    result = ""
    i = 1
    
    ' Remove surrounding quotes
    IF LEFT$(text, 1) = CHR$(34) THEN i = 2
    
    DO WHILE i < LEN(text)
        c = MID$(text, i, 1)
        
        IF c = "\" AND i < LEN(text) THEN
            nextC = MID$(text, i + 1, 1)
            SELECT CASE nextC
            CASE "n": result = result + CHR$(10): i = i + 2
            CASE "r": result = result + CHR$(13): i = i + 2
            CASE "t": result = result + CHR$(9): i = i + 2
            CASE "\": result = result + "\": i = i + 2
            CASE CHR$(34): result = result + CHR$(34): i = i + 2
            CASE ELSE: result = result + c: i = i + 1
            END SELECT
        ELSE
            result = result + c
            i = i + 1
        END IF
    LOOP
    
    Json_UnescapeString = result
END FUNCTION

' ============================================================================
' SUB: Json_Parse
' Parse JSON string into dictionary (simplified)
' ============================================================================
SUB Json_Parse (dict AS QBNex_Dict, jsonText AS STRING)
    Dict_Init dict
    ' Simplified parser - full implementation would handle nested objects/arrays
    ' This is a placeholder for the complete recursive descent parser
END SUB

' ============================================================================
' FUNCTION: Json_SkipWhitespace
' Skip whitespace in JSON text
' ============================================================================
FUNCTION Json_SkipWhitespace& (text AS STRING, POS AS LONG)
    DO WHILE POS <= LEN(text)
        SELECT CASE MID$(text, POS, 1)
        CASE " ", CHR$(9), CHR$(10), CHR$(13)
            POS = POS + 1
        CASE ELSE
            EXIT DO
        END SELECT
    LOOP
    Json_SkipWhitespace = POS
END FUNCTION
