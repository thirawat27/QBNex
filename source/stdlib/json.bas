'===============================================================================
' QBNex Standard Library - JSON Module
'===============================================================================
' JSON parsing and serialization module for QBNex.
' Provides complete JSON data interchange functionality.
'
' Features:
' - Parse JSON strings to QBNex data structures
' - Serialize QBNex data to JSON strings
' - Support for objects, arrays, strings, numbers, booleans, null
' - Error handling with detailed messages
' - Pretty printing option
'===============================================================================

'-------------------------------------------------------------------------------
' JSON VALUE TYPES
'-------------------------------------------------------------------------------

CONST JSON_NULL = 0
CONST JSON_BOOLEAN = 1
CONST JSON_NUMBER = 2
CONST JSON_STRING = 3
CONST JSON_ARRAY = 4
CONST JSON_OBJECT = 5

'-------------------------------------------------------------------------------
' JSON VALUE TYPE
'-------------------------------------------------------------------------------

TYPE JsonValue
    valueType AS INTEGER
    boolValue AS _BYTE
    numValue AS DOUBLE
    strValue AS STRING * 1024
    
    'For arrays
    arraySize AS INTEGER
    arrayValues(1 TO 100) AS INTEGER 'Indices to JsonValue pool
    
    'For objects
    objectSize AS INTEGER
    objectKeys(1 TO 50) AS STRING * 64
    objectValues(1 TO 50) AS INTEGER 'Indices to JsonValue pool
END TYPE

'-------------------------------------------------------------------------------
' JSON PARSER STATE
'-------------------------------------------------------------------------------

TYPE JsonParser
    source AS STRING * 8192
    position AS INTEGER
    length AS INTEGER
    errorMessage AS STRING * 256
    hasError AS _BYTE
END TYPE

'-------------------------------------------------------------------------------
' MODULE STATE
'-------------------------------------------------------------------------------

DIM SHARED JsonValuePool(1 TO 500) AS JsonValue
DIM SHARED JsonValuePoolSize AS INTEGER
DIM SHARED JsonValuePoolCapacity AS INTEGER

DIM SHARED JsonPrettyPrint AS _BYTE
DIM SHARED JsonIndentSize AS INTEGER

CONST JSON_POOL_INITIAL = 500

'-------------------------------------------------------------------------------
' INITIALIZATION
'-------------------------------------------------------------------------------

SUB Json_Init
    JsonValuePoolCapacity = JSON_POOL_INITIAL
    REDIM JsonValuePool(1 TO JsonValuePoolCapacity) AS JsonValue
    JsonValuePoolSize = 0
    
    JsonPrettyPrint = 0
    JsonIndentSize = 2
END SUB

SUB Json_Cleanup
    ERASE JsonValuePool
    JsonValuePoolSize = 0
    JsonValuePoolCapacity = 0
END SUB

'-------------------------------------------------------------------------------
' VALUE POOL MANAGEMENT
'-------------------------------------------------------------------------------

FUNCTION JsonAllocValue% ()
    IF JsonValuePoolSize >= JsonValuePoolCapacity THEN
        'Expand pool
        JsonValuePoolCapacity = JsonValuePoolCapacity * 2
        REDIM _PRESERVE JsonValuePool(1 TO JsonValuePoolCapacity) AS JsonValue
    END IF
    
    JsonValuePoolSize = JsonValuePoolSize + 1
    
    'Initialize to null
    JsonValuePool(JsonValuePoolSize).valueType = JSON_NULL
    JsonValuePool(JsonValuePoolSize).arraySize = 0
    JsonValuePool(JsonValuePoolSize).objectSize = 0
    
    JsonAllocValue% = JsonValuePoolSize
END FUNCTION

'-------------------------------------------------------------------------------
' Serialize JSON value to string
'-------------------------------------------------------------------------------

FUNCTION JSON_Stringify$ (valueIndex AS INTEGER)
    IF valueIndex < 1 OR valueIndex > JsonValuePoolSize THEN
        JSON_Stringify$ = "null"
        EXIT FUNCTION
    END IF
    
    DIM result AS STRING
    result = JsonStringifyValue(valueIndex, 0)
    JSON_Stringify$ = result
END FUNCTION

FUNCTION JSON_StringifyPretty$ (valueIndex AS INTEGER)
    JsonPrettyPrint = -1
    DIM result AS STRING
    result = JSON_Stringify$(valueIndex)
    JsonPrettyPrint = 0
    JSON_StringifyPretty$ = result
END FUNCTION

FUNCTION JsonStringifyValue$ (valueIndex AS INTEGER, indent AS INTEGER)
    DIM result AS STRING
    DIM i AS INTEGER
    DIM val AS JsonValue
    
    val = JsonValuePool(valueIndex)
    
    SELECT CASE val.valueType
        CASE JSON_NULL
            result = "null"
            
        CASE JSON_BOOLEAN
            IF val.boolValue THEN
                result = "true"
            ELSE
                result = "false"
            END IF
            
        CASE JSON_NUMBER
            result = STR$(val.numValue)
            'Remove leading space from STR$
            IF LEFT$(result, 1) = " " THEN
                result = MID$(result, 2)
            END IF
            
        CASE JSON_STRING
            result = JsonEscapeString(RTRIM$(val.strValue))
            
        CASE JSON_ARRAY
            result = "[" + JsonNewLine
            FOR i = 1 TO val.arraySize
                result = result + JsonIndent(indent + 1)
                result = result + JsonStringifyValue(val.arrayValues(i), indent + 1)
                IF i < val.arraySize THEN
                    result = result + ","
                END IF
                result = result + JsonNewLine
            NEXT
            result = result + JsonIndent(indent) + "]"
            
        CASE JSON_OBJECT
            result = "{" + JsonNewLine
            FOR i = 1 TO val.objectSize
                result = result + JsonIndent(indent + 1)
                result = result + JsonEscapeString(RTRIM$(val.objectKeys(i)))
                result = result + ": "
                result = result + JsonStringifyValue(val.objectValues(i), indent + 1)
                IF i < val.objectSize THEN
                    result = result + ","
                END IF
                result = result + JsonNewLine
            NEXT
            result = result + JsonIndent(indent) + "}"
    END SELECT
    
    JsonStringifyValue$ = result
END FUNCTION

FUNCTION JsonEscapeString$ (s AS STRING)
    DIM result AS STRING
    DIM i AS INTEGER
    DIM ch AS STRING * 1
    
    result = CHR$(34) 'Opening quote
    
    FOR i = 1 TO LEN(s)
        ch = MID$(s, i, 1)
        SELECT CASE ch
            CASE CHR$(34): result = result + "\"" '" 
            CASE "\": result = result + "\\"
            CASE CHR$(8): result = result + "\b"
            CASE CHR$(12): result = result + "\f"
            CASE CHR$(10): result = result + "\n"
            CASE CHR$(13): result = result + "\r"
            CASE CHR$(9): result = result + "\t"
            CASE ELSE
                IF ASC(ch) >= 32 AND ASC(ch) <= 126 THEN
                    result = result + ch
                ELSE
                    result = result + "\u" + RIGHT$("0000" + HEX$(ASC(ch)), 4)
                END IF
        END SELECT
    NEXT
    
    result = result + CHR$(34) 'Closing quote
    JsonEscapeString$ = result
END FUNCTION

FUNCTION JsonNewLine$ ()
    IF JsonPrettyPrint THEN
        JsonNewLine$ = CHR$(13) + CHR$(10)
    ELSE
        JsonNewLine$ = ""
    END IF
END FUNCTION

FUNCTION JsonIndent$ (level AS INTEGER)
    DIM result AS STRING
    DIM i AS INTEGER
    
    IF JsonPrettyPrint THEN
        result = ""
        FOR i = 1 TO level * JsonIndentSize
            result = result + " "
        NEXT
    END IF
    
    JsonIndent$ = result
END FUNCTION

'-------------------------------------------------------------------------------
' Parse JSON string to value
'-------------------------------------------------------------------------------

FUNCTION JSON_Parse% (jsonString AS STRING)
    DIM parser AS JsonParser
    DIM resultIndex AS INTEGER
    
    parser.source = jsonString
    parser.position = 1
    parser.length = LEN(jsonString)
    parser.hasError = 0
    parser.errorMessage = ""
    
    JsonSkipWhitespace parser
    
    IF parser.position > parser.length THEN
        parser.hasError = -1
        parser.errorMessage = "Empty JSON string"
        JSON_Parse% = 0
        EXIT FUNCTION
    END IF
    
    resultIndex = JsonParseValue(parser)
    
    IF parser.hasError THEN
        PRINT "JSON Parse Error: "; RTRIM$(parser.errorMessage)
        PRINT "Position: "; parser.position
        JSON_Parse% = 0
    ELSE
        JSON_Parse% = resultIndex
    END IF
END FUNCTION

FUNCTION JsonParseValue% (parser AS JsonParser)
    DIM ch AS STRING * 1
    
    JsonSkipWhitespace parser
    
    IF parser.position > parser.length THEN
        parser.hasError = -1
        parser.errorMessage = "Unexpected end of input"
        JsonParseValue% = 0
        EXIT FUNCTION
    END IF
    
    ch = MID$(parser.source, parser.position, 1)
    
    SELECT CASE ch
        CASE "n"
            JsonParseValue% = JsonParseNull(parser)
        CASE "t", "f"
            JsonParseValue% = JsonParseBoolean(parser)
        CASE CHR$(34)
            JsonParseValue% = JsonParseString(parser)
        CASE "["
            JsonParseValue% = JsonParseArray(parser)
        CASE "{"
            JsonParseValue% = JsonParseObject(parser)
        CASE "-", "0" TO "9"
            JsonParseValue% = JsonParseNumber(parser)
        CASE ELSE
            parser.hasError = -1
            parser.errorMessage = "Unexpected character: " + ch
            JsonParseValue% = 0
    END SELECT
END FUNCTION

FUNCTION JsonParseNull% (parser AS JsonParser)
    DIM valueIndex AS INTEGER
    
    IF MID$(parser.source, parser.position, 4) = "null" THEN
        valueIndex = JsonAllocValue%
        JsonValuePool(valueIndex).valueType = JSON_NULL
        parser.position = parser.position + 4
        JsonParseNull% = valueIndex
    ELSE
        parser.hasError = -1
        parser.errorMessage = "Expected 'null'"
        JsonParseNull% = 0
    END IF
END FUNCTION

FUNCTION JsonParseBoolean% (parser AS JsonParser)
    DIM valueIndex AS INTEGER
    
    IF MID$(parser.source, parser.position, 4) = "true" THEN
        valueIndex = JsonAllocValue%
        JsonValuePool(valueIndex).valueType = JSON_BOOLEAN
        JsonValuePool(valueIndex).boolValue = -1
        parser.position = parser.position + 4
        JsonParseBoolean% = valueIndex
    ELSEIF MID$(parser.source, parser.position, 5) = "false" THEN
        valueIndex = JsonAllocValue%
        JsonValuePool(valueIndex).valueType = JSON_BOOLEAN
        JsonValuePool(valueIndex).boolValue = 0
        parser.position = parser.position + 5
        JsonParseBoolean% = valueIndex
    ELSE
        parser.hasError = -1
        parser.errorMessage = "Expected 'true' or 'false'"
        JsonParseBoolean% = 0
    END IF
END FUNCTION

FUNCTION JsonParseNumber% (parser AS JsonParser)
    DIM valueIndex AS INTEGER
    DIM startPos AS INTEGER
    DIM numStr AS STRING
    
    startPos = parser.position
    
    'Optional minus sign
    IF MID$(parser.source, parser.position, 1) = "-" THEN
        parser.position = parser.position + 1
    END IF
    
    'Integer part
    DO WHILE parser.position <= parser.length
        DIM ch AS STRING * 1
        ch = MID$(parser.source, parser.position, 1)
        IF ch >= "0" AND ch <= "9" THEN
            parser.position = parser.position + 1
        ELSE
            EXIT DO
        END IF
    LOOP
    
    'Fractional part
    IF MID$(parser.source, parser.position, 1) = "." THEN
        parser.position = parser.position + 1
        DO WHILE parser.position <= parser.length
            ch = MID$(parser.source, parser.position, 1)
            IF ch >= "0" AND ch <= "9" THEN
                parser.position = parser.position + 1
            ELSE
                EXIT DO
            END IF
        LOOP
    END IF
    
    'Exponent part
    ch = MID$(parser.source, parser.position, 1)
    IF ch = "e" OR ch = "E" THEN
        parser.position = parser.position + 1
        ch = MID$(parser.source, parser.position, 1)
        IF ch = "+" OR ch = "-" THEN
            parser.position = parser.position + 1
        END IF
        DO WHILE parser.position <= parser.length
            ch = MID$(parser.source, parser.position, 1)
            IF ch >= "0" AND ch <= "9" THEN
                parser.position = parser.position + 1
            ELSE
                EXIT DO
            END IF
        LOOP
    END IF
    
    numStr = MID$(parser.source, startPos, parser.position - startPos)
    
    valueIndex = JsonAllocValue%
    JsonValuePool(valueIndex).valueType = JSON_NUMBER
    JsonValuePool(valueIndex).numValue = VAL(numStr)
    
    JsonParseNumber% = valueIndex
END FUNCTION

FUNCTION JsonParseString% (parser AS JsonParser)
    DIM valueIndex AS INTEGER
    DIM result AS STRING
    DIM ch AS STRING * 1
    
    'Skip opening quote
    parser.position = parser.position + 1
    
    result = ""
    
    DO WHILE parser.position <= parser.length
        ch = MID$(parser.source, parser.position, 1)
        
        IF ch = CHR$(34) THEN
            'End of string
            parser.position = parser.position + 1
            EXIT DO
        ELSEIF ch = "\" THEN
            'Escape sequence
            parser.position = parser.position + 1
            IF parser.position <= parser.length THEN
                ch = MID$(parser.source, parser.position, 1)
                SELECT CASE ch
                    CASE CHR$(34): result = result + CHR$(34)
                    CASE "\": result = result + "\"
                    CASE "/": result = result + "/"
                    CASE "b": result = result + CHR$(8)
                    CASE "f": result = result + CHR$(12)
                    CASE "n": result = result + CHR$(10)
                    CASE "r": result = result + CHR$(13)
                    CASE "t": result = result + CHR$(9)
                    CASE "u"
                        'Unicode escape
                        IF parser.position + 4 <= parser.length THEN
                            DIM hexCode AS STRING
                            hexCode = MID$(parser.source, parser.position + 1, 4)
                            result = result + CHR$(VAL("&H" + hexCode))
                            parser.position = parser.position + 4
                        END IF
                    CASE ELSE
                        result = result + ch
                END SELECT
                parser.position = parser.position + 1
            END IF
        ELSE
            result = result + ch
            parser.position = parser.position + 1
        END IF
    LOOP
    
    valueIndex = JsonAllocValue%
    JsonValuePool(valueIndex).valueType = JSON_STRING
    JsonValuePool(valueIndex).strValue = result
    
    JsonParseString% = valueIndex
END FUNCTION

FUNCTION JsonParseArray% (parser AS JsonParser)
    DIM valueIndex AS INTEGER
    DIM elementIndex AS INTEGER
    
    'Skip [
    parser.position = parser.position + 1
    
    valueIndex = JsonAllocValue%
    JsonValuePool(valueIndex).valueType = JSON_ARRAY
    JsonValuePool(valueIndex).arraySize = 0
    
    JsonSkipWhitespace parser
    
    'Check for empty array
    IF MID$(parser.source, parser.position, 1) = "]" THEN
        parser.position = parser.position + 1
        JsonParseArray% = valueIndex
        EXIT FUNCTION
    END IF
    
    'Parse elements
    DO
        elementIndex = JsonParseValue(parser)
        IF parser.hasError THEN
            JsonParseArray% = 0
            EXIT FUNCTION
        END IF
        
        JsonValuePool(valueIndex).arraySize = JsonValuePool(valueIndex).arraySize + 1
        JsonValuePool(valueIndex).arrayValues(JsonValuePool(valueIndex).arraySize) = elementIndex
        
        JsonSkipWhitespace parser
        
        ch = MID$(parser.source, parser.position, 1)
        IF ch = "," THEN
            parser.position = parser.position + 1
            JsonSkipWhitespace parser
        ELSEIF ch = "]" THEN
            parser.position = parser.position + 1
            EXIT DO
        ELSE
            parser.hasError = -1
            parser.errorMessage = "Expected ',' or ']' in array"
            JsonParseArray% = 0
            EXIT FUNCTION
        END IF
    LOOP
    
    JsonParseArray% = valueIndex
END FUNCTION

FUNCTION JsonParseObject% (parser AS JsonParser)
    DIM valueIndex AS INTEGER
    DIM keyIndex AS INTEGER
    DIM valueValIndex AS INTEGER
    DIM key AS STRING
    
    'Skip {
    parser.position = parser.position + 1
    
    valueIndex = JsonAllocValue%
    JsonValuePool(valueIndex).valueType = JSON_OBJECT
    JsonValuePool(valueIndex).objectSize = 0
    
    JsonSkipWhitespace parser
    
    'Check for empty object
    IF MID$(parser.source, parser.position, 1) = "}" THEN
        parser.position = parser.position + 1
        JsonParseObject% = valueIndex
        EXIT FUNCTION
    END IF
    
    'Parse key-value pairs
    DO
        'Parse key (must be string)
        JsonSkipWhitespace parser
        
        IF MID$(parser.source, parser.position, 1) <> CHR$(34) THEN
            parser.hasError = -1
            parser.errorMessage = "Expected string key in object"
            JsonParseObject% = 0
            EXIT FUNCTION
        END IF
        
        keyIndex = JsonParseString(parser)
        key = RTRIM$(JsonValuePool(keyIndex).strValue)
        
        JsonSkipWhitespace parser
        
        IF MID$(parser.source, parser.position, 1) <> ":" THEN
            parser.hasError = -1
            parser.errorMessage = "Expected ':' after object key"
            JsonParseObject% = 0
            EXIT FUNCTION
        END IF
        parser.position = parser.position + 1
        
        'Parse value
        valueValIndex = JsonParseValue(parser)
        IF parser.hasError THEN
            JsonParseObject% = 0
            EXIT FUNCTION
        END IF
        
        'Add to object
        WITH JsonValuePool(valueIndex)
            .objectSize = .objectSize + 1
            .objectKeys(.objectSize) = key
            .objectValues(.objectSize) = valueValIndex
        END WITH
        
        JsonSkipWhitespace parser
        
        ch = MID$(parser.source, parser.position, 1)
        IF ch = "," THEN
            parser.position = parser.position + 1
        ELSEIF ch = "}" THEN
            parser.position = parser.position + 1
            EXIT DO
        ELSE
            parser.hasError = -1
            parser.errorMessage = "Expected ',' or '}' in object"
            JsonParseObject% = 0
            EXIT FUNCTION
        END IF
    LOOP
    
    JsonParseObject% = valueIndex
END FUNCTION

SUB JsonSkipWhitespace (parser AS JsonParser)
    DO WHILE parser.position <= parser.length
        DIM ch AS STRING * 1
        ch = MID$(parser.source, parser.position, 1)
        IF ch = " " OR ch = CHR$(9) OR ch = CHR$(10) OR ch = CHR$(13) THEN
            parser.position = parser.position + 1
        ELSE
            EXIT DO
        END IF
    LOOP
END SUB

'-------------------------------------------------------------------------------
' BUILDER API (Programmatic JSON construction)
'-------------------------------------------------------------------------------

FUNCTION JsonCreateNull% ()
    DIM idx AS INTEGER
    idx = JsonAllocValue%
    JsonValuePool(idx).valueType = JSON_NULL
    JsonCreateNull% = idx
END FUNCTION

FUNCTION JsonCreateBoolean% (value AS _BYTE)
    DIM idx AS INTEGER
    idx = JsonAllocValue%
    JsonValuePool(idx).valueType = JSON_BOOLEAN
    JsonValuePool(idx).boolValue = value
    JsonCreateBoolean% = idx
END FUNCTION

FUNCTION JsonCreateNumber% (value AS DOUBLE)
    DIM idx AS INTEGER
    idx = JsonAllocValue%
    JsonValuePool(idx).valueType = JSON_NUMBER
    JsonValuePool(idx).numValue = value
    JsonCreateNumber% = idx
END FUNCTION

FUNCTION JsonCreateString% (value AS STRING)
    DIM idx AS INTEGER
    idx = JsonAllocValue%
    JsonValuePool(idx).valueType = JSON_STRING
    JsonValuePool(idx).strValue = value
    JsonCreateString% = idx
END FUNCTION

FUNCTION JsonCreateArray% ()
    DIM idx AS INTEGER
    idx = JsonAllocValue%
    JsonValuePool(idx).valueType = JSON_ARRAY
    JsonValuePool(idx).arraySize = 0
    JsonCreateArray% = idx
END FUNCTION

FUNCTION JsonCreateObject% ()
    DIM idx AS INTEGER
    idx = JsonAllocValue%
    JsonValuePool(idx).valueType = JSON_OBJECT
    JsonValuePool(idx).objectSize = 0
    JsonCreateObject% = idx
END FUNCTION

SUB JsonArrayPush (arrayIndex AS INTEGER, valueIndex AS INTEGER)
    IF arrayIndex < 1 OR arrayIndex > JsonValuePoolSize THEN EXIT SUB
    IF JsonValuePool(arrayIndex).valueType <> JSON_ARRAY THEN EXIT SUB
    
    WITH JsonValuePool(arrayIndex)
        IF .arraySize < UBOUND(.arrayValues) THEN
            .arraySize = .arraySize + 1
            .arrayValues(.arraySize) = valueIndex
        END IF
    END WITH
END SUB

SUB JsonObjectSet (objectIndex AS INTEGER, key AS STRING, valueIndex AS INTEGER)
    IF objectIndex < 1 OR objectIndex > JsonValuePoolSize THEN EXIT SUB
    IF JsonValuePool(objectIndex).valueType <> JSON_OBJECT THEN EXIT SUB
    
    WITH JsonValuePool(objectIndex)
        IF .objectSize < UBOUND(.objectValues) THEN
            .objectSize = .objectSize + 1
            .objectKeys(.objectSize) = key
            .objectValues(.objectSize) = valueIndex
        END IF
    END WITH
END SUB

'-------------------------------------------------------------------------------
' ACCESSOR API (Retrieve values from parsed JSON)
'-------------------------------------------------------------------------------

FUNCTION JsonGetType% (valueIndex AS INTEGER)
    IF valueIndex < 1 OR valueIndex > JsonValuePoolSize THEN
        JsonGetType% = JSON_NULL
    ELSE
        JsonGetType% = JsonValuePool(valueIndex).valueType
    END IF
END FUNCTION

FUNCTION JsonGetBoolean% (valueIndex AS INTEGER)
    IF valueIndex < 1 OR valueIndex > JsonValuePoolSize THEN
        JsonGetBoolean% = 0
    ELSEIF JsonValuePool(valueIndex).valueType = JSON_BOOLEAN THEN
        JsonGetBoolean% = JsonValuePool(valueIndex).boolValue
    ELSE
        JsonGetBoolean% = 0
    END IF
END FUNCTION

FUNCTION JsonGetNumber# (valueIndex AS INTEGER)
    IF valueIndex < 1 OR valueIndex > JsonValuePoolSize THEN
        JsonGetNumber# = 0
    ELSEIF JsonValuePool(valueIndex).valueType = JSON_NUMBER THEN
        JsonGetNumber# = JsonValuePool(valueIndex).numValue
    ELSE
        JsonGetNumber# = 0
    END IF
END FUNCTION

FUNCTION JsonGetString$ (valueIndex AS INTEGER)
    IF valueIndex < 1 OR valueIndex > JsonValuePoolSize THEN
        JsonGetString$ = ""
    ELSEIF JsonValuePool(valueIndex).valueType = JSON_STRING THEN
        JsonGetString$ = RTRIM$(JsonValuePool(valueIndex).strValue)
    ELSE
        JsonGetString$ = ""
    END IF
END FUNCTION

FUNCTION JsonObjectGet% (objectIndex AS INTEGER, key AS STRING)
    DIM i AS INTEGER
    
    IF objectIndex < 1 OR objectIndex > JsonValuePoolSize THEN
        JsonObjectGet% = 0
        EXIT FUNCTION
    END IF
    
    IF JsonValuePool(objectIndex).valueType <> JSON_OBJECT THEN
        JsonObjectGet% = 0
        EXIT FUNCTION
    END IF
    
    WITH JsonValuePool(objectIndex)
        FOR i = 1 TO .objectSize
            IF RTRIM$(.objectKeys(i)) = key THEN
                JsonObjectGet% = .objectValues(i)
                EXIT FUNCTION
            END IF
        NEXT
    END WITH
    
    JsonObjectGet% = 0
END FUNCTION

FUNCTION JsonArrayGet% (arrayIndex AS INTEGER, index AS INTEGER)
    IF arrayIndex < 1 OR arrayIndex > JsonValuePoolSize THEN
        JsonArrayGet% = 0
        EXIT FUNCTION
    END IF
    
    IF JsonValuePool(arrayIndex).valueType <> JSON_ARRAY THEN
        JsonArrayGet% = 0
        EXIT FUNCTION
    END IF
    
    IF index < 1 OR index > JsonValuePool(arrayIndex).arraySize THEN
        JsonArrayGet% = 0
    ELSE
        JsonArrayGet% = JsonValuePool(arrayIndex).arrayValues(index)
    END IF
END FUNCTION

FUNCTION JsonArrayLength% (arrayIndex AS INTEGER)
    IF arrayIndex < 1 OR arrayIndex > JsonValuePoolSize THEN
        JsonArrayLength% = 0
        EXIT FUNCTION
    END IF
    
    IF JsonValuePool(arrayIndex).valueType <> JSON_ARRAY THEN
        JsonArrayLength% = 0
    ELSE
        JsonArrayLength% = JsonValuePool(arrayIndex).arraySize
    END IF
END FUNCTION

