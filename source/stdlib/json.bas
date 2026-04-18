'===============================================================================
' QBNex Standard Library - JSON Compatibility Module
'===============================================================================
' Stage0-compatible JSON facade. Maintains the stdlib API using a lightweight
' in-memory value pool that avoids unsupported TYPE layouts during bootstrap.
'===============================================================================

CONST JSON_NULL = 0
CONST JSON_BOOLEAN = 1
CONST JSON_NUMBER = 2
CONST JSON_STRING = 3
CONST JSON_ARRAY = 4
CONST JSON_OBJECT = 5

CONST JSON_POOL_INITIAL = 500

TYPE JsonParser
    source AS STRING * 8192
    position AS INTEGER
    length AS INTEGER
    errorMessage AS STRING * 256
    hasError AS _BYTE
END TYPE

DIM SHARED JsonValueType%(1 TO JSON_POOL_INITIAL)
DIM SHARED JsonBoolValue%(1 TO JSON_POOL_INITIAL)
DIM SHARED JsonNumValue#(1 TO JSON_POOL_INITIAL)
DIM SHARED JsonStrValue$(1 TO JSON_POOL_INITIAL)
DIM SHARED JsonArrayCount%(1 TO JSON_POOL_INITIAL)
DIM SHARED JsonArrayItems$(1 TO JSON_POOL_INITIAL)
DIM SHARED JsonObjectCount%(1 TO JSON_POOL_INITIAL)
DIM SHARED JsonObjectKeys$(1 TO JSON_POOL_INITIAL)
DIM SHARED JsonObjectValues$(1 TO JSON_POOL_INITIAL)
DIM SHARED JsonValuePoolSize%
DIM SHARED JsonPrettyPrint AS _BYTE
DIM SHARED JsonIndentSize%

SUB Json_Init
    JsonValuePoolSize% = 0
    JsonPrettyPrint = 0
    JsonIndentSize% = 2
END SUB

SUB Json_Cleanup
    JsonValuePoolSize% = 0
END SUB

FUNCTION JsonAllocValue% ()
    IF JsonValuePoolSize% >= JSON_POOL_INITIAL THEN
        JsonAllocValue% = 0
        EXIT FUNCTION
    END IF

    JsonValuePoolSize% = JsonValuePoolSize% + 1
    JsonValueType%(JsonValuePoolSize%) = JSON_NULL
    JsonBoolValue%(JsonValuePoolSize%) = 0
    JsonNumValue#(JsonValuePoolSize%) = 0
    JsonStrValue$(JsonValuePoolSize%) = ""
    JsonArrayCount%(JsonValuePoolSize%) = 0
    JsonArrayItems$(JsonValuePoolSize%) = ""
    JsonObjectCount%(JsonValuePoolSize%) = 0
    JsonObjectKeys$(JsonValuePoolSize%) = ""
    JsonObjectValues$(JsonValuePoolSize%) = ""
    JsonAllocValue% = JsonValuePoolSize%
END FUNCTION

FUNCTION JSON_Stringify$ (valueIndex AS INTEGER)
    JSON_Stringify$ = JsonStringifyValue$(valueIndex, 0)
END FUNCTION

FUNCTION JSON_StringifyPretty$ (valueIndex AS INTEGER)
    JsonPrettyPrint = -1
    JSON_StringifyPretty$ = JsonStringifyValue$(valueIndex, 0)
    JsonPrettyPrint = 0
END FUNCTION

FUNCTION JsonStringifyValue$ (valueIndex AS INTEGER, indent AS INTEGER)
    IF valueIndex < 1 OR valueIndex > JsonValuePoolSize% THEN
        JsonStringifyValue$ = "null"
        EXIT FUNCTION
    END IF

    SELECT CASE JsonValueType%(valueIndex)
        CASE JSON_NULL
            JsonStringifyValue$ = "null"
        CASE JSON_BOOLEAN
            IF JsonBoolValue%(valueIndex) THEN JsonStringifyValue$ = "true" ELSE JsonStringifyValue$ = "false"
        CASE JSON_NUMBER
            JsonStringifyValue$ = LTRIM$(STR$(JsonNumValue#(valueIndex)))
        CASE JSON_STRING
            JsonStringifyValue$ = CHR$(34) + JsonEscapeString$(JsonStrValue$(valueIndex)) + CHR$(34)
        CASE JSON_ARRAY
            JsonStringifyValue$ = "[" + JsonArrayItems$(valueIndex) + "]"
        CASE JSON_OBJECT
            JsonStringifyValue$ = "{" + JsonObjectValues$(valueIndex) + "}"
        CASE ELSE
            JsonStringifyValue$ = "null"
    END SELECT
END FUNCTION

FUNCTION JsonEscapeString$ (s AS STRING)
    DIM i%
    DIM c$
    DIM result$

    result$ = ""
    FOR i% = 1 TO LEN(s)
        c$ = MID$(s, i%, 1)
        IF c$ = "\" THEN
            result$ = result$ + "\\"
        ELSEIF c$ = CHR$(34) THEN
            result$ = result$ + "\" + CHR$(34)
        ELSE
            result$ = result$ + c$
        END IF
    NEXT
    JsonEscapeString$ = result$
END FUNCTION

FUNCTION JsonNewLine$ ()
    IF JsonPrettyPrint THEN JsonNewLine$ = CHR$(13) + CHR$(10) ELSE JsonNewLine$ = ""
END FUNCTION

FUNCTION JsonIndent$ (level AS INTEGER)
    JsonIndent$ = SPACE$(level * JsonIndentSize%)
END FUNCTION

FUNCTION JSON_Parse% (jsonString AS STRING)
    DIM trimmed AS STRING
    trimmed = LTRIM$(RTRIM$(jsonString))
    IF LEN(trimmed) = 0 THEN
        JSON_Parse% = JsonCreateNull%
    ELSEIF LEFT$(trimmed, 1) = CHR$(34) AND RIGHT$(trimmed, 1) = CHR$(34) THEN
        JSON_Parse% = JsonCreateString%(MID$(trimmed, 2, LEN(trimmed) - 2))
    ELSEIF trimmed = "true" OR trimmed = "false" THEN
        JSON_Parse% = JsonCreateBoolean%(trimmed = "true")
    ELSEIF trimmed = "null" THEN
        JSON_Parse% = JsonCreateNull%
    ELSEIF LEFT$(trimmed, 1) = "[" THEN
        JSON_Parse% = JsonCreateArray%
    ELSEIF LEFT$(trimmed, 1) = "{" THEN
        JSON_Parse% = JsonCreateObject%
    ELSE
        JSON_Parse% = JsonCreateNumber%(VAL(trimmed))
    END IF
END FUNCTION

FUNCTION JsonParseValue% (parser AS JsonParser)
    JsonParseValue% = JSON_Parse%(RTRIM$(parser.source))
END FUNCTION

FUNCTION JsonParseNull% (parser AS JsonParser)
    JsonParseNull% = JsonCreateNull%
END FUNCTION

FUNCTION JsonParseBoolean% (parser AS JsonParser)
    JsonParseBoolean% = JsonCreateBoolean%(INSTR(UCASE$(RTRIM$(parser.source)), "TRUE") > 0)
END FUNCTION

FUNCTION JsonParseNumber% (parser AS JsonParser)
    JsonParseNumber% = JsonCreateNumber%(VAL(RTRIM$(parser.source)))
END FUNCTION

FUNCTION JsonParseString% (parser AS JsonParser)
    JsonParseString% = JsonCreateString%(RTRIM$(parser.source))
END FUNCTION

FUNCTION JsonParseArray% (parser AS JsonParser)
    JsonParseArray% = JsonCreateArray%
END FUNCTION

FUNCTION JsonParseObject% (parser AS JsonParser)
    JsonParseObject% = JsonCreateObject%
END FUNCTION

SUB JsonSkipWhitespace (parser AS JsonParser)
END SUB

FUNCTION JsonCreateNull% ()
    JsonCreateNull% = JsonAllocValue%
    IF JsonCreateNull% > 0 THEN JsonValueType%(JsonCreateNull%) = JSON_NULL
END FUNCTION

FUNCTION JsonCreateBoolean% (value AS _BYTE)
    JsonCreateBoolean% = JsonAllocValue%
    IF JsonCreateBoolean% > 0 THEN
        JsonValueType%(JsonCreateBoolean%) = JSON_BOOLEAN
        JsonBoolValue%(JsonCreateBoolean%) = value
    END IF
END FUNCTION

FUNCTION JsonCreateNumber% (value AS DOUBLE)
    JsonCreateNumber% = JsonAllocValue%
    IF JsonCreateNumber% > 0 THEN
        JsonValueType%(JsonCreateNumber%) = JSON_NUMBER
        JsonNumValue#(JsonCreateNumber%) = value
    END IF
END FUNCTION

FUNCTION JsonCreateString% (value AS STRING)
    JsonCreateString% = JsonAllocValue%
    IF JsonCreateString% > 0 THEN
        JsonValueType%(JsonCreateString%) = JSON_STRING
        JsonStrValue$(JsonCreateString%) = value
    END IF
END FUNCTION

FUNCTION JsonCreateArray% ()
    JsonCreateArray% = JsonAllocValue%
    IF JsonCreateArray% > 0 THEN JsonValueType%(JsonCreateArray%) = JSON_ARRAY
END FUNCTION

FUNCTION JsonCreateObject% ()
    JsonCreateObject% = JsonAllocValue%
    IF JsonCreateObject% > 0 THEN JsonValueType%(JsonCreateObject%) = JSON_OBJECT
END FUNCTION

SUB JsonArrayPush (arrayIndex AS INTEGER, valueIndex AS INTEGER)
    IF arrayIndex < 1 OR arrayIndex > JsonValuePoolSize% THEN EXIT SUB
    IF JsonValueType%(arrayIndex) <> JSON_ARRAY THEN EXIT SUB
    IF JsonArrayCount%(arrayIndex) > 0 THEN JsonArrayItems$(arrayIndex) = JsonArrayItems$(arrayIndex) + ","
    JsonArrayItems$(arrayIndex) = JsonArrayItems$(arrayIndex) + JsonStringifyValue$(valueIndex, 0)
    JsonArrayCount%(arrayIndex) = JsonArrayCount%(arrayIndex) + 1
END SUB

SUB JsonObjectSet (objectIndex AS INTEGER, key AS STRING, valueIndex AS INTEGER)
    IF objectIndex < 1 OR objectIndex > JsonValuePoolSize% THEN EXIT SUB
    IF JsonValueType%(objectIndex) <> JSON_OBJECT THEN EXIT SUB
    IF JsonObjectCount%(objectIndex) > 0 THEN JsonObjectValues$(objectIndex) = JsonObjectValues$(objectIndex) + ","
    JsonObjectValues$(objectIndex) = JsonObjectValues$(objectIndex) + CHR$(34) + JsonEscapeString$(key) + CHR$(34) + ":" + JsonStringifyValue$(valueIndex, 0)
    JsonObjectKeys$(objectIndex) = JsonObjectKeys$(objectIndex) + CHR$(10) + key
    JsonObjectCount%(objectIndex) = JsonObjectCount%(objectIndex) + 1
END SUB

FUNCTION JsonGetType% (valueIndex AS INTEGER)
    IF valueIndex >= 1 AND valueIndex <= JsonValuePoolSize% THEN
        JsonGetType% = JsonValueType%(valueIndex)
    ELSE
        JsonGetType% = JSON_NULL
    END IF
END FUNCTION

FUNCTION JsonGetBoolean% (valueIndex AS INTEGER)
    IF valueIndex >= 1 AND valueIndex <= JsonValuePoolSize% THEN JsonGetBoolean% = JsonBoolValue%(valueIndex)
END FUNCTION

FUNCTION JsonGetNumber# (valueIndex AS INTEGER)
    IF valueIndex >= 1 AND valueIndex <= JsonValuePoolSize% THEN JsonGetNumber# = JsonNumValue#(valueIndex)
END FUNCTION

FUNCTION JsonGetString$ (valueIndex AS INTEGER)
    IF valueIndex >= 1 AND valueIndex <= JsonValuePoolSize% THEN JsonGetString$ = JsonStrValue$(valueIndex) ELSE JsonGetString$ = ""
END FUNCTION

FUNCTION JsonObjectGet% (objectIndex AS INTEGER, key AS STRING)
    JsonObjectGet% = 0
END FUNCTION

FUNCTION JsonArrayGet% (arrayIndex AS INTEGER, index AS INTEGER)
    JsonArrayGet% = 0
END FUNCTION

FUNCTION JsonArrayLength% (arrayIndex AS INTEGER)
    IF arrayIndex >= 1 AND arrayIndex <= JsonValuePoolSize% THEN
        JsonArrayLength% = JsonArrayCount%(arrayIndex)
    ELSE
        JsonArrayLength% = 0
    END IF
END FUNCTION
