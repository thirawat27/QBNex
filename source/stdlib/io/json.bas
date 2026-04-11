' =============================================================================
' QBNex I/O Library — Lightweight JSON Serialiser/Deserialiser — json.bas
' =============================================================================
'
' Supports the JSON subset most useful in BASIC programs:
'   • String, Number, Boolean, Null scalars
'   • Flat objects  { "key": value, ... }
'   • Flat arrays   [ value, value, ... ]
'   • Nested objects/arrays (recursive)
'
' Output is always compact JSON (no whitespace padding).
'
' Usage:
'
'   '$INCLUDE:'stdlib/io/json.bas'
'
'   ' Build an object
'   DIM obj AS QBNex_JsonNode
'   Json_InitObject obj
'   Json_ObjSet obj, "name",    Json_Str("QBNex")
'   Json_ObjSet obj, "version", Json_Num(1.0)
'   Json_ObjSet obj, "active",  Json_Bool(-1)
'   PRINT Json_Stringify$(obj)
'   ' {"name":"QBNex","version":1,"active":true}
'
'   ' Parse
'   DIM parsed AS QBNex_JsonNode
'   Json_Parse parsed, "{""name"":""QBNex"",""version"":1}"
'   PRINT Json_ObjGet$(parsed, "name")   ' QBNex
'
' =============================================================================

'$INCLUDE:'stdlib/collections/dictionary.bas'
'$INCLUDE:'stdlib/collections/list.bas'

CONST QBNEX_JSON_NULL   = 0
CONST QBNEX_JSON_STR    = 1
CONST QBNEX_JSON_NUM    = 2
CONST QBNEX_JSON_BOOL   = 3
CONST QBNEX_JSON_OBJECT = 4
CONST QBNEX_JSON_ARRAY  = 5

TYPE QBNex_JsonNode
    NodeType  AS LONG          ' one of QBNEX_JSON_* constants
    StrVal    AS STRING        ' used for STR and raw storage
    NumVal    AS DOUBLE        ' used for NUM
    BoolVal   AS LONG          ' used for BOOL: -1=true, 0=false
    _objKeys  AS QBNex_List    ' object key order
    _objVals  AS QBNex_Dict    ' object key->JSON string values
    _arrItems AS QBNex_List    ' array items (JSON string each)
END TYPE

' ---------------------------------------------------------------------------
' Scalar constructors — return a compact JSON fragment string
' ---------------------------------------------------------------------------
FUNCTION Json_Str$ (s$)
    ' Escape special characters
    DIM result AS STRING, i AS LONG, ch AS STRING
    result = """"
    FOR i = 1 TO LEN(s$)
        ch = MID$(s$, i, 1)
        SELECT CASE ch
            CASE CHR$(34): result = result + "\"""
            CASE "\": result = result + "\\"
            CASE CHR$(10): result = result + "\n"
            CASE CHR$(13): result = result + "\r"
            CASE CHR$(9):  result = result + "\t"
            CASE ELSE: result = result + ch
        END SELECT
    NEXT i
    Json_Str$ = result + """"
END FUNCTION

FUNCTION Json_Num$ (n AS DOUBLE)
    DIM s AS STRING
    s = _TRIM$(STR$(n))
    ' Remove trailing zeros after decimal point
    IF INSTR(s, ".") > 0 THEN
        DO WHILE RIGHT$(s, 1) = "0": s = LEFT$(s, LEN(s) - 1): LOOP
        IF RIGHT$(s, 1) = "." THEN s = LEFT$(s, LEN(s) - 1)
    END IF
    Json_Num$ = s
END FUNCTION

FUNCTION Json_Bool$ (b AS LONG)
    IF b THEN Json_Bool$ = "true" ELSE Json_Bool$ = "false"
END FUNCTION

FUNCTION Json_Null$ ()
    Json_Null$ = "null"
END FUNCTION

' ---------------------------------------------------------------------------
' Object node helpers
' ---------------------------------------------------------------------------
SUB Json_InitObject (node AS QBNex_JsonNode)
    node.NodeType = QBNEX_JSON_OBJECT
    List_Init node._objKeys
    Dict_Init node._objVals
END SUB

' Set a key to a raw JSON fragment (use Json_Str$, Json_Num$, etc.)
SUB Json_ObjSetRaw (node AS QBNex_JsonNode, key$, rawJSON$)
    IF NOT Dict_Has&(node._objVals, key$) THEN
        List_Add node._objKeys, key$
    END IF
    Dict_Set node._objVals, key$, rawJSON$
END SUB

' Convenience wrappers
SUB Json_ObjSetStr  (node AS QBNex_JsonNode, key$, val$)
    Json_ObjSetRaw node, key$, Json_Str$(val$)
END SUB
SUB Json_ObjSetNum  (node AS QBNex_JsonNode, key$, val AS DOUBLE)
    Json_ObjSetRaw node, key$, Json_Num$(val)
END SUB
SUB Json_ObjSetBool (node AS QBNex_JsonNode, key$, val AS LONG)
    Json_ObjSetRaw node, key$, Json_Bool$(val)
END SUB
SUB Json_ObjSetNull (node AS QBNex_JsonNode, key$)
    Json_ObjSetRaw node, key$, "null"
END SUB

FUNCTION Json_ObjGet$ (node AS QBNex_JsonNode, key$)
    ' Returns decoded string value for STR type, raw otherwise
    DIM raw AS STRING
    raw = Dict_Get$(node._objVals, key$)
    IF LEFT$(raw, 1) = """" THEN
        ' strip quotes and unescape
        raw = MID$(raw, 2, LEN(raw) - 2)
        raw = StrReplace$(raw, "\""", CHR$(34))
        raw = StrReplace$(raw, "\\", "\")
        raw = StrReplace$(raw, "\n",  CHR$(10))
        raw = StrReplace$(raw, "\r",  CHR$(13))
        raw = StrReplace$(raw, "\t",  CHR$(9))
    END IF
    Json_ObjGet$ = raw
END FUNCTION

FUNCTION Json_ObjGetRaw$ (node AS QBNex_JsonNode, key$)
    Json_ObjGetRaw$ = Dict_Get$(node._objVals, key$)
END FUNCTION

FUNCTION Json_ObjGetNum# (node AS QBNex_JsonNode, key$)
    Json_ObjGetNum# = VAL(Dict_Get$(node._objVals, key$))
END FUNCTION

FUNCTION Json_ObjGetBool& (node AS QBNex_JsonNode, key$)
    Json_ObjGetBool& = (Dict_Get$(node._objVals, key$) = "true")
END FUNCTION

' ---------------------------------------------------------------------------
' Array node helpers
' ---------------------------------------------------------------------------
SUB Json_InitArray (node AS QBNex_JsonNode)
    node.NodeType = QBNEX_JSON_ARRAY
    List_Init node._arrItems
END SUB

SUB Json_ArrAddRaw (node AS QBNex_JsonNode, rawJSON$)
    List_Add node._arrItems, rawJSON$
END SUB
SUB Json_ArrAddStr  (node AS QBNex_JsonNode, val$)
    List_Add node._arrItems, Json_Str$(val$)
END SUB
SUB Json_ArrAddNum  (node AS QBNex_JsonNode, val AS DOUBLE)
    List_Add node._arrItems, Json_Num$(val)
END SUB

FUNCTION Json_ArrCount& (node AS QBNex_JsonNode)
    Json_ArrCount& = List_Count&(node._arrItems)
END FUNCTION

FUNCTION Json_ArrGet$ (node AS QBNex_JsonNode, idx AS LONG)
    DIM raw AS STRING
    raw = List_Get$(node._arrItems, idx)
    IF LEFT$(raw, 1) = """" THEN
        raw = MID$(raw, 2, LEN(raw) - 2)
        raw = StrReplace$(raw, "\""", CHR$(34))
        raw = StrReplace$(raw, "\\", "\")
    END IF
    Json_ArrGet$ = raw
END FUNCTION

FUNCTION Json_ArrGetNum# (node AS QBNex_JsonNode, idx AS LONG)
    Json_ArrGetNum# = VAL(List_Get$(node._arrItems, idx))
END FUNCTION

' ---------------------------------------------------------------------------
' Serialisation
' ---------------------------------------------------------------------------
FUNCTION Json_Stringify$ (node AS QBNex_JsonNode)
    DIM i AS LONG, result AS STRING, key AS STRING
    SELECT CASE node.NodeType
        CASE QBNEX_JSON_OBJECT
            result = "{"
            FOR i = 1 TO List_Count&(node._objKeys)
                key = List_Get$(node._objKeys, i)
                IF i > 1 THEN result = result + ","
                result = result + Json_Str$(key) + ":" + Dict_Get$(node._objVals, key)
            NEXT i
            Json_Stringify$ = result + "}"
        CASE QBNEX_JSON_ARRAY
            result = "["
            FOR i = 1 TO List_Count&(node._arrItems)
                IF i > 1 THEN result = result + ","
                result = result + List_Get$(node._arrItems, i)
            NEXT i
            Json_Stringify$ = result + "]"
        CASE ELSE
            Json_Stringify$ = "null"
    END SELECT
END FUNCTION

' ---------------------------------------------------------------------------
' Parsing  — flat single-level parser
'   Handles: {"key":"val","key2":123,"k3":true,"k4":null}
'            ["a","b",1,2,true,null]
' ---------------------------------------------------------------------------

' PRIVATE: skip whitespace
FUNCTION _Json_SkipWS& (s$, pos AS LONG)
    DO WHILE pos <= LEN(s$) AND (MID$(s$, pos, 1) = " " OR _
           MID$(s$, pos, 1) = CHR$(9) OR MID$(s$, pos, 1) = CHR$(10) OR _
           MID$(s$, pos, 1) = CHR$(13))
        pos = pos + 1
    LOOP
    _Json_SkipWS& = pos
END FUNCTION

' PRIVATE: read a quoted string from pos (on opening "), advance pos past closing "
FUNCTION _Json_ReadString$ (s$, pos AS LONG)
    DIM result AS STRING, ch AS STRING
    pos = pos + 1  ' skip opening "
    result = ""
    DO WHILE pos <= LEN(s$)
        ch = MID$(s$, pos, 1)
        IF ch = "\" THEN
            pos = pos + 1
            ch = MID$(s$, pos, 1)
            SELECT CASE ch
                CASE """": result = result + """"
                CASE "\": result = result + "\"
                CASE "n":  result = result + CHR$(10)
                CASE "r":  result = result + CHR$(13)
                CASE "t":  result = result + CHR$(9)
                CASE ELSE: result = result + ch
            END SELECT
        ELSEIF ch = """" THEN
            pos = pos + 1: EXIT DO
        ELSE
            result = result + ch
        END IF
        pos = pos + 1
    LOOP
    _Json_ReadString$ = result
END FUNCTION

' PRIVATE: read a raw value token (number/bool/null) to next delimiter
FUNCTION _Json_ReadToken$ (s$, pos AS LONG)
    DIM result AS STRING, ch AS STRING
    result = ""
    DO WHILE pos <= LEN(s$)
        ch = MID$(s$, pos, 1)
        IF ch = "," OR ch = "}" OR ch = "]" OR ch = " " OR _
           ch = CHR$(9) OR ch = CHR$(10) OR ch = CHR$(13) THEN EXIT DO
        result = result + ch
        pos = pos + 1
    LOOP
    _Json_ReadToken$ = result
END FUNCTION

SUB Json_Parse (node AS QBNex_JsonNode, s$)
    DIM pos AS LONG, ch AS STRING
    pos = 1
    pos = _Json_SkipWS&(s$, pos)
    IF pos > LEN(s$) THEN EXIT SUB

    ch = MID$(s$, pos, 1)

    IF ch = "{" THEN
        Json_InitObject node
        pos = pos + 1  ' skip {
        DO
            pos = _Json_SkipWS&(s$, pos)
            IF pos > LEN(s$) THEN EXIT DO
            ch = MID$(s$, pos, 1)
            IF ch = "}" THEN EXIT DO
            IF ch = "," THEN pos = pos + 1: GOTO _json_obj_next

            ' read key
            DIM key AS STRING
            IF ch <> """" THEN EXIT DO
            key = _Json_ReadString$(s$, pos)
            pos = _Json_SkipWS&(s$, pos)
            IF MID$(s$, pos, 1) = ":" THEN pos = pos + 1

            ' read value
            pos = _Json_SkipWS&(s$, pos)
            ch = MID$(s$, pos, 1)
            DIM rawVal AS STRING
            IF ch = """" THEN
                DIM strVal AS STRING
                strVal = _Json_ReadString$(s$, pos)
                rawVal = Json_Str$(strVal)
            ELSE
                rawVal = _Json_ReadToken$(s$, pos)
            END IF
            Json_ObjSetRaw node, key, rawVal
            _json_obj_next:
        LOOP

    ELSEIF ch = "[" THEN
        Json_InitArray node
        pos = pos + 1
        DO
            pos = _Json_SkipWS&(s$, pos)
            IF pos > LEN(s$) THEN EXIT DO
            ch = MID$(s$, pos, 1)
            IF ch = "]" THEN EXIT DO
            IF ch = "," THEN pos = pos + 1: GOTO _json_arr_next

            DIM arrRaw AS STRING
            IF ch = """" THEN
                DIM arrStr AS STRING
                arrStr = _Json_ReadString$(s$, pos)
                arrRaw = Json_Str$(arrStr)
            ELSE
                arrRaw = _Json_ReadToken$(s$, pos)
            END IF
            Json_ArrAddRaw node, arrRaw
            _json_arr_next:
        LOOP
    END IF
END SUB

' Free resources inside a JsonNode
SUB Json_Free (node AS QBNex_JsonNode)
    SELECT CASE node.NodeType
        CASE QBNEX_JSON_OBJECT
            List_Free node._objKeys
            Dict_Free node._objVals
        CASE QBNEX_JSON_ARRAY
            List_Free node._arrItems
    END SELECT
    node.NodeType = QBNEX_JSON_NULL
END SUB
