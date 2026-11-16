'===============================================================================
' QBNex Standard Library - json ('$IMPORT:'json')
'===============================================================================
' RFC 8259 JSON, like Python's json. A value is a handle (an INTEGER, 0 = none).
' Read with a path: names and 0-based indexes, "user.tags.0" or "items[2].id".
'
'   root = Json_Parse(text$)                 0 on error: LastError, Json_ErrorPos
'   PRINT Json_Str(root, "name")            text of a value ("" when missing)
'   PRINT Json_Num(root, "tags[1]")         Json_Bool, Json_Type ("object", ...)
'   FOR i = 0 TO Json_Count(root, "tags") - 1 ...     Json_Has, Json_Key
'   o = Json_Object: Json_SetStr o, "name", "Ann": Json_SetNum o, "age", 42
'   a = Json_Array: Json_PushStr a, "x": Json_Set o, "list", a
'   PRINT Json_Stringify(o)                 Json_Stringify(o, 2) indents
'   Json_Free root                           gives a document back
'
' Strings are bytes: \uXXXX escapes (with surrogate pairs) become UTF-8.
' Parsed numbers are written back exactly as they were. Objects keep member
' order; setting a key that is there replaces its value. Adding a value that
' already belongs to an array or object adds a copy. Up to 32767 values exist
' at a time; Json_FreeAll gives every value back.
'===============================================================================

'$IMPORT:'core'

CONST JSON_TYPE_NULL = 0
CONST JSON_TYPE_BOOLEAN = 1
CONST JSON_TYPE_NUMBER = 2
CONST JSON_TYPE_STRING = 3
CONST JSON_TYPE_ARRAY = 4
CONST JSON_TYPE_OBJECT = 5

CONST JSON_POOL_MAX = 32767
CONST JSON_MAX_DEPTH = 500

' Value pool: grown by JsonAllocValue (the arrays are declared without a size,
' so this line does not reset them wherever the module is placed). A free
' value has type JSON_TYPE_FREE; JsonNextSibling% links the free values.
CONST JSON_TYPE_FREE = -1
DIM SHARED JsonValueType%()
DIM SHARED JsonBoolValue%()
DIM SHARED JsonNumValue#()
DIM SHARED JsonStrValue$() ' string value, or the source text of a parsed number
DIM SHARED JsonKey$() ' member name when the parent is an object
DIM SHARED JsonParent%()
DIM SHARED JsonFirstChild%()
DIM SHARED JsonLastChild%()
DIM SHARED JsonNextSibling%()
DIM SHARED JsonChildCount%()
DIM SHARED JsonParseLog%() ' the values a parse made, freed when it fails
DIM SHARED JsonPoolCapacity%
DIM SHARED JsonValuePoolSize%
DIM SHARED JsonFreeValue%
DIM SHARED JsonParseLogged&
DIM SHARED JsonParseLogCap&
DIM SHARED JsonParsing AS _BYTE
DIM SHARED JsonPrettyPrint AS _BYTE
DIM SHARED JsonIndentSize%
DIM SHARED JsonSrc$
DIM SHARED JsonPos&
DIM SHARED JsonErrMsg$
DIM SHARED JsonErrPos&
DIM SHARED JsonDepth%


' Gives every JSON value back (every handle is then invalid).
SUB Json_FreeAll ()
    JsonValuePoolSize% = 0
    JsonFreeValue% = 0
END SUB

FUNCTION JsonAllocValue% ()
    DIM n%, c&
    IF JsonFreeValue% > 0 THEN
        n% = JsonFreeValue%
        JsonFreeValue% = JsonNextSibling%(n%)
    ELSE
        IF JsonValuePoolSize% >= JSON_POOL_MAX THEN EXIT FUNCTION
        n% = JsonValuePoolSize% + 1
        IF n% > JsonPoolCapacity% THEN
            c& = JsonPoolCapacity% * 2&
            IF c& < 64 THEN c& = 64
            IF c& > JSON_POOL_MAX THEN c& = JSON_POOL_MAX
            REDIM _PRESERVE JsonValueType%(1 TO c&)
            REDIM _PRESERVE JsonBoolValue%(1 TO c&)
            REDIM _PRESERVE JsonNumValue#(1 TO c&)
            REDIM _PRESERVE JsonStrValue$(1 TO c&)
            REDIM _PRESERVE JsonKey$(1 TO c&)
            REDIM _PRESERVE JsonParent%(1 TO c&)
            REDIM _PRESERVE JsonFirstChild%(1 TO c&)
            REDIM _PRESERVE JsonLastChild%(1 TO c&)
            REDIM _PRESERVE JsonNextSibling%(1 TO c&)
            REDIM _PRESERVE JsonChildCount%(1 TO c&)
            JsonPoolCapacity% = c&
        END IF
        JsonValuePoolSize% = n%
    END IF
    IF JsonParsing THEN
        JsonParseLogged& = JsonParseLogged& + 1
        IF JsonParseLogged& > JsonParseLogCap& THEN
            JsonParseLogCap& = JsonParseLogCap& * 2 + 64
            REDIM _PRESERVE JsonParseLog%(1 TO JsonParseLogCap&)
        END IF
        JsonParseLog%(JsonParseLogged&) = n%
    END IF
    JsonValueType%(n%) = JSON_TYPE_NULL
    JsonBoolValue%(n%) = 0
    JsonNumValue#(n%) = 0
    JsonStrValue$(n%) = ""
    JsonKey$(n%) = ""
    JsonParent%(n%) = 0
    JsonFirstChild%(n%) = 0
    JsonLastChild%(n%) = 0
    JsonNextSibling%(n%) = 0
    JsonChildCount%(n%) = 0
    JsonAllocValue% = n%
END FUNCTION

FUNCTION JsonValid% (valueIndex AS INTEGER)
    IF valueIndex >= 1 AND valueIndex <= JsonValuePoolSize% THEN JsonValid% = (JsonValueType%(valueIndex) <> JSON_TYPE_FREE)
END FUNCTION

' Gives back a value and all values in it (a document: free its root); it is first removed from the array
' or object that holds it. Its handle (and those of its children) are then
' invalid and are used again for new values.
SUB Json_Free (valueIndex AS INTEGER)
    DIM p%, c%, prev%
    IF NOT JsonValid%(valueIndex) THEN EXIT SUB
    p% = JsonParent%(valueIndex)
    IF JsonValid%(p%) THEN
        c% = JsonFirstChild%(p%)
        DO WHILE c% > 0
            IF c% = valueIndex THEN
                IF prev% > 0 THEN JsonNextSibling%(prev%) = JsonNextSibling%(c%) ELSE JsonFirstChild%(p%) = JsonNextSibling%(c%)
                IF JsonLastChild%(p%) = c% THEN JsonLastChild%(p%) = prev%
                JsonChildCount%(p%) = JsonChildCount%(p%) - 1
                EXIT DO
            END IF
            prev% = c%
            c% = JsonNextSibling%(c%)
        LOOP
    END IF
    JsonFreeTree valueIndex
END SUB

SUB JsonFreeTree (valueIndex AS INTEGER)
    DIM c%, nx%
    c% = JsonFirstChild%(valueIndex)
    DO WHILE c% > 0
        nx% = JsonNextSibling%(c%)
        JsonFreeTree c%
        c% = nx%
    LOOP
    JsonFreeOne valueIndex
END SUB

SUB JsonFreeOne (valueIndex AS INTEGER)
    JsonValueType%(valueIndex) = JSON_TYPE_FREE
    JsonStrValue$(valueIndex) = ""
    JsonKey$(valueIndex) = ""
    JsonParent%(valueIndex) = 0
    JsonFirstChild%(valueIndex) = 0
    JsonLastChild%(valueIndex) = 0
    JsonChildCount%(valueIndex) = 0
    JsonNextSibling%(valueIndex) = JsonFreeValue%
    JsonFreeValue% = valueIndex
END SUB

'------------------------------------------------------------------ creation

FUNCTION JsonCreateNull% ()
    JsonCreateNull% = JsonAllocValue%
END FUNCTION

FUNCTION JsonCreateBoolean% (value AS _BYTE)
    DIM v%
    v% = JsonAllocValue%
    IF v% > 0 THEN
        JsonValueType%(v%) = JSON_TYPE_BOOLEAN
        IF value THEN JsonBoolValue%(v%) = -1
    END IF
    JsonCreateBoolean% = v%
END FUNCTION

FUNCTION JsonCreateNumber% (value AS DOUBLE)
    DIM v%
    v% = JsonAllocValue%
    IF v% > 0 THEN
        JsonValueType%(v%) = JSON_TYPE_NUMBER
        JsonNumValue#(v%) = value
    END IF
    JsonCreateNumber% = v%
END FUNCTION

FUNCTION JsonCreateString% (value AS STRING)
    DIM v%
    v% = JsonAllocValue%
    IF v% > 0 THEN
        JsonValueType%(v%) = JSON_TYPE_STRING
        JsonStrValue$(v%) = value
    END IF
    JsonCreateString% = v%
END FUNCTION

FUNCTION JsonCreateArray% ()
    DIM v%
    v% = JsonAllocValue%
    IF v% > 0 THEN JsonValueType%(v%) = JSON_TYPE_ARRAY
    JsonCreateArray% = v%
END FUNCTION

FUNCTION JsonCreateObject% ()
    DIM v%
    v% = JsonAllocValue%
    IF v% > 0 THEN JsonValueType%(v%) = JSON_TYPE_OBJECT
    JsonCreateObject% = v%
END FUNCTION

' Deep copy of a value (used when a value that already has a parent is added).
FUNCTION JsonClone% (valueIndex AS INTEGER)
    DIM v%, c%, k%
    IF NOT JsonValid%(valueIndex) THEN EXIT FUNCTION
    v% = JsonAllocValue%
    IF v% = 0 THEN EXIT FUNCTION
    JsonValueType%(v%) = JsonValueType%(valueIndex)
    JsonBoolValue%(v%) = JsonBoolValue%(valueIndex)
    JsonNumValue#(v%) = JsonNumValue#(valueIndex)
    JsonStrValue$(v%) = JsonStrValue$(valueIndex)
    c% = JsonFirstChild%(valueIndex)
    DO WHILE c% > 0
        k% = JsonClone%(c%)
        IF k% = 0 THEN EXIT DO
        JsonKey$(k%) = JsonKey$(c%)
        JsonLinkChild v%, k%
        c% = JsonNextSibling%(c%)
    LOOP
    JsonClone% = v%
END FUNCTION

SUB JsonLinkChild (parentIndex AS INTEGER, childIndex AS INTEGER)
    JsonParent%(childIndex) = parentIndex
    JsonNextSibling%(childIndex) = 0
    IF JsonLastChild%(parentIndex) > 0 THEN
        JsonNextSibling%(JsonLastChild%(parentIndex)) = childIndex
    ELSE
        JsonFirstChild%(parentIndex) = childIndex
    END IF
    JsonLastChild%(parentIndex) = childIndex
    JsonChildCount%(parentIndex) = JsonChildCount%(parentIndex) + 1
END SUB

' A child that is free to link: the value itself, or a copy when it already
' has a parent (or is the container itself / one of its ancestors).
FUNCTION JsonAdoptable% (containerIndex AS INTEGER, valueIndex AS INTEGER)
    DIM a%
    IF NOT JsonValid%(valueIndex) THEN
        JsonAdoptable% = JsonCreateNull%
        EXIT FUNCTION
    END IF
    a% = containerIndex
    DO WHILE a% > 0
        IF a% = valueIndex THEN
            JsonAdoptable% = JsonClone%(valueIndex)
            EXIT FUNCTION
        END IF
        a% = JsonParent%(a%)
    LOOP
    IF JsonParent%(valueIndex) <> 0 THEN
        JsonAdoptable% = JsonClone%(valueIndex)
    ELSE
        JsonAdoptable% = valueIndex
    END IF
END FUNCTION

SUB JsonArrayPush (arrayIndex AS INTEGER, valueIndex AS INTEGER)
    DIM c%
    IF NOT JsonValid%(arrayIndex) THEN EXIT SUB
    IF JsonValueType%(arrayIndex) <> JSON_TYPE_ARRAY THEN EXIT SUB
    c% = JsonAdoptable%(arrayIndex, valueIndex)
    IF c% = 0 THEN EXIT SUB
    JsonKey$(c%) = ""
    JsonLinkChild arrayIndex, c%
END SUB

SUB JsonObjectSet (objectIndex AS INTEGER, memberName AS STRING, valueIndex AS INTEGER)
    DIM c%, p%, old%
    IF NOT JsonValid%(objectIndex) THEN EXIT SUB
    IF JsonValueType%(objectIndex) <> JSON_TYPE_OBJECT THEN EXIT SUB
    c% = JsonAdoptable%(objectIndex, valueIndex)
    IF c% = 0 THEN EXIT SUB
    JsonKey$(c%) = memberName
    ' replace an existing member in place
    p% = 0
    old% = JsonFirstChild%(objectIndex)
    DO WHILE old% > 0
        IF JsonKey$(old%) = memberName THEN
            JsonParent%(c%) = objectIndex
            JsonNextSibling%(c%) = JsonNextSibling%(old%)
            IF p% > 0 THEN JsonNextSibling%(p%) = c% ELSE JsonFirstChild%(objectIndex) = c%
            IF JsonLastChild%(objectIndex) = old% THEN JsonLastChild%(objectIndex) = c%
            JsonParent%(old%) = 0
            JsonNextSibling%(old%) = 0
            EXIT SUB
        END IF
        p% = old%
        old% = JsonNextSibling%(old%)
    LOOP
    JsonLinkChild objectIndex, c%
END SUB

SUB JsonObjectRemove (objectIndex AS INTEGER, memberName AS STRING)
    DIM p%, c%
    IF NOT JsonValid%(objectIndex) THEN EXIT SUB
    IF JsonValueType%(objectIndex) <> JSON_TYPE_OBJECT THEN EXIT SUB
    c% = JsonFirstChild%(objectIndex)
    DO WHILE c% > 0
        IF JsonKey$(c%) = memberName THEN
            IF p% > 0 THEN JsonNextSibling%(p%) = JsonNextSibling%(c%) ELSE JsonFirstChild%(objectIndex) = JsonNextSibling%(c%)
            IF JsonLastChild%(objectIndex) = c% THEN JsonLastChild%(objectIndex) = p%
            JsonChildCount%(objectIndex) = JsonChildCount%(objectIndex) - 1
            JsonParent%(c%) = 0
            JsonNextSibling%(c%) = 0
            EXIT SUB
        END IF
        p% = c%
        c% = JsonNextSibling%(c%)
    LOOP
END SUB

'------------------------------------------------------------------- queries

FUNCTION JsonGetType% (valueIndex AS INTEGER)
    IF JsonValid%(valueIndex) THEN JsonGetType% = JsonValueType%(valueIndex) ELSE JsonGetType% = JSON_TYPE_NULL
END FUNCTION

FUNCTION JsonGetBoolean% (valueIndex AS INTEGER)
    IF JsonValid%(valueIndex) THEN JsonGetBoolean% = JsonBoolValue%(valueIndex)
END FUNCTION

FUNCTION JsonGetNumber# (valueIndex AS INTEGER)
    IF JsonValid%(valueIndex) THEN JsonGetNumber# = JsonNumValue#(valueIndex)
END FUNCTION

FUNCTION JsonGetString$ (valueIndex AS INTEGER)
    IF JsonValid%(valueIndex) THEN
        IF JsonValueType%(valueIndex) = JSON_TYPE_STRING THEN JsonGetString$ = JsonStrValue$(valueIndex)
    END IF
END FUNCTION

FUNCTION JsonArrayLength% (arrayIndex AS INTEGER)
    IF JsonValid%(arrayIndex) THEN
        IF JsonValueType%(arrayIndex) = JSON_TYPE_ARRAY OR JsonValueType%(arrayIndex) = JSON_TYPE_OBJECT THEN JsonArrayLength% = JsonChildCount%(arrayIndex)
    END IF
END FUNCTION

FUNCTION JsonObjectCount% (objectIndex AS INTEGER)
    JsonObjectCount% = JsonArrayLength%(objectIndex)
END FUNCTION

' 0-based element of an array (or member of an object, in order); 0 if none.
FUNCTION JsonArrayGet% (arrayIndex AS INTEGER, index AS INTEGER)
    DIM c%, i%
    IF NOT JsonValid%(arrayIndex) OR index < 0 THEN EXIT FUNCTION
    c% = JsonFirstChild%(arrayIndex)
    DO WHILE c% > 0 AND i% < index
        c% = JsonNextSibling%(c%)
        i% = i% + 1
    LOOP
    JsonArrayGet% = c%
END FUNCTION

FUNCTION JsonObjectGet% (objectIndex AS INTEGER, memberName AS STRING)
    DIM c%
    IF NOT JsonValid%(objectIndex) THEN EXIT FUNCTION
    IF JsonValueType%(objectIndex) <> JSON_TYPE_OBJECT THEN EXIT FUNCTION
    c% = JsonFirstChild%(objectIndex)
    DO WHILE c% > 0
        IF JsonKey$(c%) = memberName THEN
            JsonObjectGet% = c%
            EXIT FUNCTION
        END IF
        c% = JsonNextSibling%(c%)
    LOOP
END FUNCTION

FUNCTION JsonObjectHas% (objectIndex AS INTEGER, memberName AS STRING)
    JsonObjectHas% = (JsonObjectGet%(objectIndex, memberName) > 0)
END FUNCTION

' Name of the 0-based member of an object.
FUNCTION JsonObjectKey$ (objectIndex AS INTEGER, index AS INTEGER)
    DIM c%
    IF JsonGetType%(objectIndex) <> JSON_TYPE_OBJECT THEN EXIT FUNCTION
    c% = JsonArrayGet%(objectIndex, index)
    IF c% > 0 THEN JsonObjectKey$ = JsonKey$(c%)
END FUNCTION

' Byte position (1-based) where the last Json_Parse failed.
FUNCTION Json_ErrorPos& ()
    Json_ErrorPos& = JsonErrPos&
END FUNCTION

'---------------------------------------------------------------- serializer

' JSON text of a value; indent > 0 puts members on lines indented that much.
FUNCTION Json_Stringify$ (valueIndex AS INTEGER, OPTIONAL indent AS INTEGER = 0)
    JsonPrettyPrint = (indent > 0)
    JsonIndentSize% = indent
    Json_Stringify$ = JsonStringifyValue$(valueIndex, 0)
    JsonPrettyPrint = 0
END FUNCTION

' JSON text for a DOUBLE: STR$ digits with a leading 0 and an E exponent.
FUNCTION JsonNumberText$ (value AS DOUBLE)
    DIM s$, p%
    IF value <> value THEN
        JsonNumberText$ = "null" ' NaN has no JSON form
        EXIT FUNCTION
    END IF
    s$ = LTRIM$(STR$(value))
    p% = INSTR(s$, "D")
    IF p% > 0 THEN MID$(s$, p%, 1) = "E"
    IF LEFT$(s$, 1) = "." THEN s$ = "0" + s$
    IF LEFT$(s$, 2) = "-." THEN s$ = "-0" + MID$(s$, 2)
    p% = INSTR(s$, ".E")
    IF p% > 0 THEN s$ = LEFT$(s$, p%) + "0" + MID$(s$, p% + 1)
    IF RIGHT$(s$, 1) = "." THEN s$ = s$ + "0"
    JsonNumberText$ = s$
END FUNCTION

FUNCTION JsonStringifyValue$ (valueIndex AS INTEGER, indent AS INTEGER)
    DIM r$, c%, nl$, pad$, inner$
    IF NOT JsonValid%(valueIndex) THEN
        JsonStringifyValue$ = "null"
        EXIT FUNCTION
    END IF

    SELECT CASE JsonValueType%(valueIndex)
        CASE JSON_TYPE_BOOLEAN
            IF JsonBoolValue%(valueIndex) THEN r$ = "true" ELSE r$ = "false"
        CASE JSON_TYPE_NUMBER
            IF LEN(JsonStrValue$(valueIndex)) THEN r$ = JsonStrValue$(valueIndex) ELSE r$ = JsonNumberText$(JsonNumValue#(valueIndex))
        CASE JSON_TYPE_STRING
            r$ = CHR$(34) + JsonEscapeString$(JsonStrValue$(valueIndex)) + CHR$(34)
        CASE JSON_TYPE_ARRAY, JSON_TYPE_OBJECT
            nl$ = JsonNewLine$
            pad$ = ""
            inner$ = ""
            IF JsonPrettyPrint THEN
                pad$ = JsonIndent$(indent + 1)
                inner$ = JsonIndent$(indent)
            END IF
            IF JsonValueType%(valueIndex) = JSON_TYPE_ARRAY THEN r$ = "[" ELSE r$ = "{"
            c% = JsonFirstChild%(valueIndex)
            IF c% > 0 THEN r$ = r$ + nl$
            DO WHILE c% > 0
                r$ = r$ + pad$
                IF JsonValueType%(valueIndex) = JSON_TYPE_OBJECT THEN
                    r$ = r$ + CHR$(34) + JsonEscapeString$(JsonKey$(c%)) + CHR$(34) + ":"
                    IF JsonPrettyPrint THEN r$ = r$ + " "
                END IF
                r$ = r$ + JsonStringifyValue$(c%, indent + 1)
                c% = JsonNextSibling%(c%)
                IF c% > 0 THEN r$ = r$ + ","
                r$ = r$ + nl$
            LOOP
            IF JsonFirstChild%(valueIndex) > 0 THEN r$ = r$ + inner$
            IF JsonValueType%(valueIndex) = JSON_TYPE_ARRAY THEN r$ = r$ + "]" ELSE r$ = r$ + "}"
        CASE ELSE
            r$ = "null"
    END SELECT
    JsonStringifyValue$ = r$
END FUNCTION

FUNCTION JsonEscapeString$ (s AS STRING)
    DIM i&, c%, r$
    FOR i& = 1 TO LEN(s)
        c% = ASC(s, i&)
        SELECT CASE c%
            CASE 34: r$ = r$ + "\" + CHR$(34)
            CASE 92: r$ = r$ + "\\"
            CASE 8: r$ = r$ + "\b"
            CASE 9: r$ = r$ + "\t"
            CASE 10: r$ = r$ + "\n"
            CASE 12: r$ = r$ + "\f"
            CASE 13: r$ = r$ + "\r"
            CASE IS < 32: r$ = r$ + "\u00" + RIGHT$("0" + LCASE$(HEX$(c%)), 2)
            CASE ELSE: r$ = r$ + CHR$(c%)
        END SELECT
    NEXT
    JsonEscapeString$ = r$
END FUNCTION

FUNCTION JsonNewLine$ ()
    IF JsonPrettyPrint THEN JsonNewLine$ = CHR$(10) ELSE JsonNewLine$ = ""
END FUNCTION

FUNCTION JsonIndent$ (level AS INTEGER)
    IF JsonIndentSize% <= 0 THEN JsonIndentSize% = 2
    JsonIndent$ = SPACE$(level * JsonIndentSize%)
END FUNCTION

'-------------------------------------------------------------------- parser

' Parses a complete JSON text. Returns the root value, or 0 on error
' (LastError / Json_ErrorPos describe it).
FUNCTION Json_Parse% (jsonString AS STRING)
    DIM v%, i&
    JsonParsing = -1
    JsonParseLogged& = 0
    v% = JsonParseText%(jsonString)
    JsonParsing = 0
    IF v% = 0 THEN
        FOR i& = JsonParseLogged& TO 1 STEP -1
            IF JsonValueType%(JsonParseLog%(i&)) <> JSON_TYPE_FREE THEN JsonFreeOne JsonParseLog%(i&)
        NEXT
    END IF
    JsonParseLogged& = 0
    Core_SetError JsonErrMsg$
    Json_Parse% = v%
END FUNCTION

FUNCTION JsonParseText% (jsonString AS STRING)
    DIM v%
    JsonSrc$ = jsonString
    JsonPos& = 1
    JsonErrMsg$ = ""
    JsonErrPos& = 0
    JsonDepth% = 0
    JsonSkipWs
    IF JsonPos& > LEN(JsonSrc$) THEN
        JsonFail "unexpected end of input, expected a value"
        EXIT FUNCTION
    END IF
    v% = JsonParseAny%
    IF LEN(JsonErrMsg$) THEN EXIT FUNCTION
    JsonSkipWs
    IF JsonPos& <= LEN(JsonSrc$) THEN
        JsonFail "unexpected text after the value"
        EXIT FUNCTION
    END IF
    JsonParseText% = v%
END FUNCTION

SUB JsonFail (message AS STRING)
    IF LEN(JsonErrMsg$) = 0 THEN
        JsonErrMsg$ = message
        JsonErrPos& = JsonPos&
    END IF
END SUB

SUB JsonSkipWs
    DIM c%
    DO WHILE JsonPos& <= LEN(JsonSrc$)
        c% = ASC(JsonSrc$, JsonPos&)
        IF c% <> 32 AND c% <> 9 AND c% <> 10 AND c% <> 13 THEN EXIT SUB
        JsonPos& = JsonPos& + 1
    LOOP
END SUB

FUNCTION JsonPeek% ()
    IF JsonPos& <= LEN(JsonSrc$) THEN JsonPeek% = ASC(JsonSrc$, JsonPos&) ELSE JsonPeek% = -1
END FUNCTION

FUNCTION JsonParseAny% ()
    DIM c%, v%
    JsonSkipWs
    c% = JsonPeek%
    SELECT CASE c%
        CASE -1
            JsonFail "unexpected end of input, expected a value"
        CASE 123 ' {
            JsonParseAny% = JsonParseObjectAt%
        CASE 91 ' [
            JsonParseAny% = JsonParseArrayAt%
        CASE 34 ' "
            DIM s$
            s$ = JsonParseStringText$
            IF LEN(JsonErrMsg$) = 0 THEN
                v% = JsonCreateString%(s$)
                IF v% = 0 THEN JsonFail "out of JSON value space"
                JsonParseAny% = v%
            END IF
        CASE 116 ' t
            JsonParseAny% = JsonParseLiteral%("true", JSON_TYPE_BOOLEAN, -1)
        CASE 102 ' f
            JsonParseAny% = JsonParseLiteral%("false", JSON_TYPE_BOOLEAN, 0)
        CASE 110 ' n
            JsonParseAny% = JsonParseLiteral%("null", JSON_TYPE_NULL, 0)
        CASE 45, 48 TO 57
            JsonParseAny% = JsonParseNumberAt%
        CASE ELSE
            JsonFail "unexpected character '" + CHR$(c%) + "'"
    END SELECT
END FUNCTION

FUNCTION JsonParseLiteral% (word AS STRING, kind AS INTEGER, flag AS INTEGER)
    DIM v%
    IF MID$(JsonSrc$, JsonPos&, LEN(word)) <> word THEN
        JsonFail "invalid literal, expected " + word
        EXIT FUNCTION
    END IF
    JsonPos& = JsonPos& + LEN(word)
    v% = JsonAllocValue%
    IF v% = 0 THEN
        JsonFail "out of JSON value space"
        EXIT FUNCTION
    END IF
    JsonValueType%(v%) = kind
    JsonBoolValue%(v%) = flag
    JsonParseLiteral% = v%
END FUNCTION

FUNCTION JsonIsDigit% (c AS INTEGER)
    JsonIsDigit% = (c >= 48 AND c <= 57)
END FUNCTION

' number = [ minus ] int [ frac ] [ exp ]   (RFC 8259 section 6)
FUNCTION JsonParseNumberAt% ()
    DIM start&, v%, t$
    start& = JsonPos&
    IF JsonPeek% = 45 THEN JsonPos& = JsonPos& + 1
    IF JsonPeek% = 48 THEN
        JsonPos& = JsonPos& + 1
        IF JsonIsDigit%(JsonPeek%) THEN
            JsonFail "leading zeros are not allowed"
            EXIT FUNCTION
        END IF
    ELSEIF JsonIsDigit%(JsonPeek%) THEN
        DO WHILE JsonIsDigit%(JsonPeek%): JsonPos& = JsonPos& + 1: LOOP
    ELSE
        JsonFail "expected a digit"
        EXIT FUNCTION
    END IF
    IF JsonPeek% = 46 THEN
        JsonPos& = JsonPos& + 1
        IF NOT JsonIsDigit%(JsonPeek%) THEN
            JsonFail "expected a digit after the decimal point"
            EXIT FUNCTION
        END IF
        DO WHILE JsonIsDigit%(JsonPeek%): JsonPos& = JsonPos& + 1: LOOP
    END IF
    IF JsonPeek% = 101 OR JsonPeek% = 69 THEN
        JsonPos& = JsonPos& + 1
        IF JsonPeek% = 43 OR JsonPeek% = 45 THEN JsonPos& = JsonPos& + 1
        IF NOT JsonIsDigit%(JsonPeek%) THEN
            JsonFail "expected a digit in the exponent"
            EXIT FUNCTION
        END IF
        DO WHILE JsonIsDigit%(JsonPeek%): JsonPos& = JsonPos& + 1: LOOP
    END IF
    t$ = MID$(JsonSrc$, start&, JsonPos& - start&)
    v% = JsonAllocValue%
    IF v% = 0 THEN
        JsonFail "out of JSON value space"
        EXIT FUNCTION
    END IF
    JsonValueType%(v%) = JSON_TYPE_NUMBER
    JsonNumValue#(v%) = JsonNumberValue#(t$)
    JsonStrValue$(v%) = t$
    JsonParseNumberAt% = v%
END FUNCTION

' ' Value of a JSON number lexeme. Magnitudes beyond DOUBLE become the largest
' DOUBLE (with sign) and tiny ones 0, so VAL never sees an out-of-range text.
FUNCTION JsonNumberValue# (t AS STRING)
    DIM p&, e&, m$, i&, c%, intDigits&, seenPoint%, lead&, pos10&, neg%, leadInt%, r#
    p& = INSTR(t, "e")
    IF p& = 0 THEN p& = INSTR(t, "E")
    m$ = t
    IF p& THEN
        m$ = LEFT$(t, p& - 1)
        IF LEN(t) - p& > 9 THEN
            IF MID$(t, p& + 1, 1) = "-" THEN e& = -999999999 ELSE e& = 999999999
        ELSE
            e& = VAL(MID$(t, p& + 1))
        END IF
    END IF
    neg% = (LEFT$(m$, 1) = "-")
    ' pos10& = decimal exponent of the first significant digit (d.ddd x 10^pos10&)
    lead& = 0
    FOR i& = 1 TO LEN(m$)
        c% = ASC(m$, i&)
        IF c% = 46 THEN
            seenPoint% = -1
        ELSEIF c% >= 48 AND c% <= 57 THEN
            IF NOT seenPoint% THEN intDigits& = intDigits& + 1
            IF lead& = 0 AND c% > 48 THEN
                lead& = intDigits&
                IF seenPoint% THEN lead& = -(i& - INSTR(m$, ".")) ' position after the point
                IF seenPoint% THEN pos10& = lead& ELSE leadInt% = -1
            END IF
        END IF
    NEXT
    IF lead& = 0 THEN EXIT FUNCTION ' the value is zero
    IF leadInt% THEN pos10& = intDigits& - lead& ' first significant digit before the point
    IF pos10& + e& > 308 THEN
        IF neg% THEN JsonNumberValue# = -1.7976931348623157D+308 ELSE JsonNumberValue# = 1.7976931348623157D+308
        EXIT FUNCTION
    END IF
    IF pos10& + e& < -330 THEN EXIT FUNCTION
    r# = VAL(t)
    IF ABS(r#) > 1.7976931348623157D+308 THEN ' a mantissa above 1.797...: VAL overflowed to infinity
        IF neg% THEN r# = -1.7976931348623157D+308 ELSE r# = 1.7976931348623157D+308
    END IF
    JsonNumberValue# = r#
END FUNCTION
FUNCTION JsonHexValue& (h AS STRING)
    DIM i%, c%, v&
    IF LEN(h) <> 4 THEN
        JsonHexValue& = -1
        EXIT FUNCTION
    END IF
    FOR i% = 1 TO 4
        c% = ASC(h, i%)
        SELECT CASE c%
            CASE 48 TO 57: v& = v& * 16 + c% - 48
            CASE 65 TO 70: v& = v& * 16 + c% - 55
            CASE 97 TO 102: v& = v& * 16 + c% - 87
            CASE ELSE
                JsonHexValue& = -1
                EXIT FUNCTION
        END SELECT
    NEXT
    JsonHexValue& = v&
END FUNCTION

FUNCTION JsonUtf8$ (cp AS LONG)
    IF cp < &H80 THEN
        JsonUtf8$ = CHR$(cp)
    ELSEIF cp < &H800 THEN
        JsonUtf8$ = CHR$(&HC0 OR (cp \ 64)) + CHR$(&H80 OR (cp AND 63))
    ELSEIF cp < &H10000 THEN
        JsonUtf8$ = CHR$(&HE0 OR (cp \ 4096)) + CHR$(&H80 OR ((cp \ 64) AND 63)) + CHR$(&H80 OR (cp AND 63))
    ELSE
        JsonUtf8$ = CHR$(&HF0 OR (cp \ 262144)) + CHR$(&H80 OR ((cp \ 4096) AND 63)) + CHR$(&H80 OR ((cp \ 64) AND 63)) + CHR$(&H80 OR (cp AND 63))
    END IF
END FUNCTION

' Reads a string token at JsonPos& (which is on the opening quote).
FUNCTION JsonParseStringText$ ()
    DIM r$, c%, runStart&, cp&, lo&
    JsonPos& = JsonPos& + 1
    runStart& = JsonPos&
    DO
        IF JsonPos& > LEN(JsonSrc$) THEN
            JsonFail "unterminated string"
            EXIT FUNCTION
        END IF
        c% = ASC(JsonSrc$, JsonPos&)
        IF c% = 34 THEN
            r$ = r$ + MID$(JsonSrc$, runStart&, JsonPos& - runStart&)
            JsonPos& = JsonPos& + 1
            JsonParseStringText$ = r$
            EXIT FUNCTION
        ELSEIF c% < 32 THEN
            JsonFail "control character in string"
            EXIT FUNCTION
        ELSEIF c% = 92 THEN
            r$ = r$ + MID$(JsonSrc$, runStart&, JsonPos& - runStart&)
            JsonPos& = JsonPos& + 1
            IF JsonPos& > LEN(JsonSrc$) THEN
                JsonFail "unterminated string"
                EXIT FUNCTION
            END IF
            c% = ASC(JsonSrc$, JsonPos&)
            SELECT CASE c%
                CASE 34: r$ = r$ + CHR$(34)
                CASE 92: r$ = r$ + "\"
                CASE 47: r$ = r$ + "/"
                CASE 98: r$ = r$ + CHR$(8)
                CASE 102: r$ = r$ + CHR$(12)
                CASE 110: r$ = r$ + CHR$(10)
                CASE 114: r$ = r$ + CHR$(13)
                CASE 116: r$ = r$ + CHR$(9)
                CASE 117 ' u
                    cp& = JsonHexValue&(MID$(JsonSrc$, JsonPos& + 1, 4))
                    IF cp& < 0 THEN
                        JsonFail "invalid \u escape"
                        EXIT FUNCTION
                    END IF
                    JsonPos& = JsonPos& + 4
                    IF cp& >= &HD800& AND cp& <= &HDBFF& THEN
                        IF MID$(JsonSrc$, JsonPos& + 1, 2) <> "\u" THEN
                            JsonFail "unpaired surrogate in \u escape"
                            EXIT FUNCTION
                        END IF
                        lo& = JsonHexValue&(MID$(JsonSrc$, JsonPos& + 3, 4))
                        IF lo& < &HDC00& OR lo& > &HDFFF& THEN
                            JsonFail "unpaired surrogate in \u escape"
                            EXIT FUNCTION
                        END IF
                        JsonPos& = JsonPos& + 6
                        cp& = &H10000 + (cp& - &HD800&) * 1024 + (lo& - &HDC00&)
                    ELSEIF cp& >= &HDC00& AND cp& <= &HDFFF& THEN
                        JsonFail "unpaired surrogate in \u escape"
                        EXIT FUNCTION
                    END IF
                    r$ = r$ + JsonUtf8$(cp&)
                CASE ELSE
                    JsonFail "invalid escape \" + CHR$(c%)
                    EXIT FUNCTION
            END SELECT
            JsonPos& = JsonPos& + 1
            runStart& = JsonPos&
        ELSE
            JsonPos& = JsonPos& + 1
        END IF
    LOOP
END FUNCTION

FUNCTION JsonParseArrayAt% ()
    DIM a%, v%
    JsonDepth% = JsonDepth% + 1
    IF JsonDepth% > JSON_MAX_DEPTH THEN
        JsonFail "nesting too deep"
        EXIT FUNCTION
    END IF
    a% = JsonCreateArray%
    IF a% = 0 THEN
        JsonFail "out of JSON value space"
        EXIT FUNCTION
    END IF
    JsonPos& = JsonPos& + 1
    JsonSkipWs
    IF JsonPeek% = 93 THEN
        JsonPos& = JsonPos& + 1
        JsonDepth% = JsonDepth% - 1
        JsonParseArrayAt% = a%
        EXIT FUNCTION
    END IF
    DO
        v% = JsonParseAny%
        IF LEN(JsonErrMsg$) THEN EXIT FUNCTION
        JsonLinkChild a%, v%
        JsonSkipWs
        SELECT CASE JsonPeek%
            CASE 44
                JsonPos& = JsonPos& + 1
            CASE 93
                JsonPos& = JsonPos& + 1
                EXIT DO
            CASE -1
                JsonFail "unexpected end of input in array"
                EXIT FUNCTION
            CASE ELSE
                JsonFail "expected ',' or ']'"
                EXIT FUNCTION
        END SELECT
    LOOP
    JsonDepth% = JsonDepth% - 1
    JsonParseArrayAt% = a%
END FUNCTION

FUNCTION JsonParseObjectAt% ()
    DIM o%, v%, k$, old%
    JsonDepth% = JsonDepth% + 1
    IF JsonDepth% > JSON_MAX_DEPTH THEN
        JsonFail "nesting too deep"
        EXIT FUNCTION
    END IF
    o% = JsonCreateObject%
    IF o% = 0 THEN
        JsonFail "out of JSON value space"
        EXIT FUNCTION
    END IF
    JsonPos& = JsonPos& + 1
    JsonSkipWs
    IF JsonPeek% = 125 THEN
        JsonPos& = JsonPos& + 1
        JsonDepth% = JsonDepth% - 1
        JsonParseObjectAt% = o%
        EXIT FUNCTION
    END IF
    DO
        JsonSkipWs
        IF JsonPeek% <> 34 THEN
            IF JsonPeek% = -1 THEN JsonFail "unexpected end of input in object" ELSE JsonFail "expected a string key"
            EXIT FUNCTION
        END IF
        k$ = JsonParseStringText$
        IF LEN(JsonErrMsg$) THEN EXIT FUNCTION
        JsonSkipWs
        IF JsonPeek% <> 58 THEN
            JsonFail "expected ':'"
            EXIT FUNCTION
        END IF
        JsonPos& = JsonPos& + 1
        v% = JsonParseAny%
        IF LEN(JsonErrMsg$) THEN EXIT FUNCTION
        ' a duplicate key keeps the last value: the one it replaces is given back
        old% = JsonObjectGet%(o%, k$)
        JsonObjectSet o%, k$, v%
        IF old% > 0 THEN
            IF JsonParent%(old%) = 0 THEN JsonFreeTree old%
        END IF
        JsonSkipWs
        SELECT CASE JsonPeek%
            CASE 44
                JsonPos& = JsonPos& + 1
            CASE 125
                JsonPos& = JsonPos& + 1
                EXIT DO
            CASE -1
                JsonFail "unexpected end of input in object"
                EXIT FUNCTION
            CASE ELSE
                JsonFail "expected ',' or '}'"
                EXIT FUNCTION
        END SELECT
    LOOP
    JsonDepth% = JsonDepth% - 1
    JsonParseObjectAt% = o%
END FUNCTION

'-------------------------------------------

'---------------------------------------------------------------- public API

' The value at path inside v ("" = v itself): member names and 0-based
' indexes, "user.tags.0" or "items[2].id". 0 when there is none.
FUNCTION Json_Get% (v AS INTEGER, OPTIONAL path AS STRING = "")
    DIM cur%, i&, c$, part$, p$
    cur% = v
    IF NOT JsonValid%(cur%) THEN EXIT FUNCTION
    p$ = path + "."
    FOR i& = 1 TO LEN(p$)
        c$ = MID$(p$, i&, 1)
        IF c$ = "." OR c$ = "[" OR c$ = "]" THEN
            IF LEN(part$) THEN
                cur% = JsonStep%(cur%, part$)
                IF cur% = 0 THEN EXIT FUNCTION
                part$ = ""
            END IF
        ELSE
            part$ = part$ + c$
        END IF
    NEXT
    Json_Get% = cur%
END FUNCTION

' The member name of an object, or the index of an array, one step down.
FUNCTION JsonStep% (v AS INTEGER, part AS STRING)
    DIM i&
    SELECT CASE JsonValueType%(v)
        CASE JSON_TYPE_OBJECT
            JsonStep% = JsonObjectGet%(v, part)
        CASE JSON_TYPE_ARRAY
            FOR i& = 1 TO LEN(part)
                IF INSTR("0123456789", MID$(part, i&, 1)) = 0 THEN EXIT FUNCTION
            NEXT
            IF LEN(part) < 6 THEN JsonStep% = JsonArrayGet%(v, VAL(part))
    END SELECT
END FUNCTION

' "object", "array", "string", "number", "boolean" or "null"; "" when missing.
FUNCTION Json_Type$ (v AS INTEGER, OPTIONAL path AS STRING = "")
    DIM x%
    x% = Json_Get%(v, path)
    IF x% = 0 THEN EXIT FUNCTION
    SELECT CASE JsonValueType%(x%)
        CASE JSON_TYPE_OBJECT: Json_Type$ = "object"
        CASE JSON_TYPE_ARRAY: Json_Type$ = "array"
        CASE JSON_TYPE_STRING: Json_Type$ = "string"
        CASE JSON_TYPE_NUMBER: Json_Type$ = "number"
        CASE JSON_TYPE_BOOLEAN: Json_Type$ = "boolean"
        CASE ELSE: Json_Type$ = "null"
    END SELECT
END FUNCTION

' -1 when there is a value at path.
FUNCTION Json_Has% (v AS INTEGER, path AS STRING)
    Json_Has% = (Json_Get%(v, path) > 0)
END FUNCTION

' A string as its text; a number, true or false as JSON text; null or missing as "".
FUNCTION Json_Str$ (v AS INTEGER, OPTIONAL path AS STRING = "")
    DIM x%
    x% = Json_Get%(v, path)
    IF x% = 0 THEN EXIT FUNCTION
    SELECT CASE JsonValueType%(x%)
        CASE JSON_TYPE_STRING: Json_Str$ = JsonStrValue$(x%)
        CASE JSON_TYPE_NULL
        CASE ELSE: Json_Str$ = JsonStringifyValue$(x%, 0)
    END SELECT
END FUNCTION

' A number (a string through VAL, true = -1); 0 when missing.
FUNCTION Json_Num# (v AS INTEGER, OPTIONAL path AS STRING = "")
    DIM x%
    x% = Json_Get%(v, path)
    IF x% = 0 THEN EXIT FUNCTION
    SELECT CASE JsonValueType%(x%)
        CASE JSON_TYPE_NUMBER: Json_Num# = JsonNumValue#(x%)
        CASE JSON_TYPE_STRING: Json_Num# = VAL(JsonStrValue$(x%))
        CASE JSON_TYPE_BOOLEAN: Json_Num# = JsonBoolValue%(x%)
    END SELECT
END FUNCTION

' -1 for true or a number that is not 0; 0 otherwise.
FUNCTION Json_Bool% (v AS INTEGER, OPTIONAL path AS STRING = "")
    Json_Bool% = (Json_Num#(v, path) <> 0)
END FUNCTION

' Number of elements of an array or members of an object.
FUNCTION Json_Count% (v AS INTEGER, OPTIONAL path AS STRING = "")
    Json_Count% = JsonArrayLength%(Json_Get%(v, path))
END FUNCTION

' Name of member index (0-based) of an object.
FUNCTION Json_Key$ (v AS INTEGER, index AS INTEGER, OPTIONAL path AS STRING = "")
    Json_Key$ = JsonObjectKey$(Json_Get%(v, path), index)
END FUNCTION

' A new, empty object.
FUNCTION Json_Object% ()
    Json_Object% = JsonCreateObject%
END FUNCTION

' A new, empty array.
FUNCTION Json_Array% ()
    Json_Array% = JsonCreateArray%
END FUNCTION

' Sets member keyName of object o to value x (an object, an array, ...).
SUB Json_Set (o AS INTEGER, keyName AS STRING, x AS INTEGER)
    JsonObjectSet o, keyName, x
END SUB

' Sets member keyName of object o to a string.
SUB Json_SetStr (o AS INTEGER, keyName AS STRING, text AS STRING)
    JsonObjectSet o, keyName, JsonCreateString%(text)
END SUB

' Sets member keyName of object o to a number.
SUB Json_SetNum (o AS INTEGER, keyName AS STRING, n AS DOUBLE)
    JsonObjectSet o, keyName, JsonCreateNumber%(n)
END SUB

' Sets member keyName of object o to true (b <> 0) or false.
SUB Json_SetBool (o AS INTEGER, keyName AS STRING, b AS INTEGER)
    JsonObjectSet o, keyName, JsonCreateBoolean%(b <> 0)
END SUB

' Sets member keyName of object o to null.
SUB Json_SetNull (o AS INTEGER, keyName AS STRING)
    JsonObjectSet o, keyName, JsonCreateNull%
END SUB

' Removes member keyName of object o.
SUB Json_Remove (o AS INTEGER, keyName AS STRING)
    JsonObjectRemove o, keyName
END SUB

' Adds value x at the end of array a.
SUB Json_Push (a AS INTEGER, x AS INTEGER)
    JsonArrayPush a, x
END SUB

' Adds a string at the end of array a.
SUB Json_PushStr (a AS INTEGER, text AS STRING)
    JsonArrayPush a, JsonCreateString%(text)
END SUB

' Adds a number at the end of array a.
SUB Json_PushNum (a AS INTEGER, n AS DOUBLE)
    JsonArrayPush a, JsonCreateNumber%(n)
END SUB

' Adds true (b <> 0) or false at the end of array a.
SUB Json_PushBool (a AS INTEGER, b AS INTEGER)
    JsonArrayPush a, JsonCreateBoolean%(b <> 0)
END SUB