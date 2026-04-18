'===============================================================================
' QBNex Standard Library - Main Import File
'===============================================================================
' Modern API for QBNex - Simple, clean function names
'
' Usage:
'   IMPORT qbnex              'Import entire stdlib
'   IMPORT json               'Import JSON module only
'
' Simple API (clean names):
' - json_parse, json_string        : JSON handling
' - encode, decode                 : URL encoding
'===============================================================================

'-------------------------------------------------------------------------------
' STDLIB VERSION
'-------------------------------------------------------------------------------

CONST STDLIB_VERSION = "1.0.0"
CONST STDLIB_NAME = "QBNex Standard Library"

'-------------------------------------------------------------------------------
' MODULE INCLUDES
'-------------------------------------------------------------------------------

' Core data modules
$INCLUDE:'json.bas'
$INCLUDE:'url.bas'

' Collections
$INCLUDE:'collections\list.bas'
$INCLUDE:'collections\stack.bas'
$INCLUDE:'collections\queue.bas'
$INCLUDE:'collections\set.bas'
$INCLUDE:'collections\dictionary.bas'

' Strings
$INCLUDE:'strings\strbuilder.bas'
$INCLUDE:'strings\text.bas'

' I/O
$INCLUDE:'io\csv.bas'
$INCLUDE:'io\json.bas'
$INCLUDE:'io\path.bas'

' System
$INCLUDE:'sys\args.bas'
$INCLUDE:'sys\datetime.bas'
$INCLUDE:'sys\env.bas'

' Math and error handling
$INCLUDE:'math\numeric.bas'
$INCLUDE:'error\result.bas'

' OOP runtime
$INCLUDE:'oop\interface.bas'

'-------------------------------------------------------------------------------
' STDLIB INITIALIZATION
'-------------------------------------------------------------------------------

SUB Stdlib_Init
    'Initialize all stdlib modules
    Json_Init
    Url_Init
    
    PRINT "QBNex Standard Library v"; STDLIB_VERSION; " loaded."
END SUB

SUB Stdlib_Cleanup
    'Cleanup all stdlib modules
    Json_Cleanup
END SUB

'-------------------------------------------------------------------------------
' HIGH-LEVEL API WRAPPERS
'-------------------------------------------------------------------------------

'--- JSON API (Data Serialization) ---

' Parse JSON string (returns INTEGER index)
FUNCTION json_parse AS INTEGER (jsonString AS STRING)
    json_parse = JSON_Parse(jsonString)
END FUNCTION

' Convert to JSON string
FUNCTION json_string AS STRING (valueIndex AS INTEGER)
    json_string = JSON_Stringify(valueIndex)
END FUNCTION

' Pretty print JSON
FUNCTION json_pretty AS STRING (valueIndex AS INTEGER)
    json_pretty = JSON_StringifyPretty(valueIndex)
END FUNCTION

' Create JSON values
FUNCTION json_null AS INTEGER ()
    json_null = JsonCreateNull%
END FUNCTION

FUNCTION json_bool AS INTEGER (value AS _BYTE)
    json_bool = JsonCreateBoolean(value)
END FUNCTION

FUNCTION json_num AS INTEGER (value AS DOUBLE)
    json_num = JsonCreateNumber(value)
END FUNCTION

FUNCTION json_str AS INTEGER (value AS STRING)
    json_str = JsonCreateString(value)
END FUNCTION

FUNCTION json_array AS INTEGER ()
    json_array = JsonCreateArray%
END FUNCTION

FUNCTION json_obj AS INTEGER ()
    json_obj = JsonCreateObject%
END FUNCTION

' Add to array / object
SUB array_add (arrayIndex AS INTEGER, valueIndex AS INTEGER)
    JsonArrayPush arrayIndex, valueIndex
END SUB

SUB obj_set (objectIndex AS INTEGER, KEY AS STRING, valueIndex AS INTEGER)
    JsonObjectSet objectIndex, KEY, valueIndex
END SUB

' Get values
FUNCTION json_get_str AS STRING (valueIndex AS INTEGER)
    json_get_str = JsonGetString$(valueIndex)
END FUNCTION

FUNCTION json_get_num AS DOUBLE (valueIndex AS INTEGER)
    json_get_num = JsonGetNumber(valueIndex)
END FUNCTION

'--- URL API (URL Manipulation) ---

' Parse URL
SUB url_parse (urlString AS STRING, parts AS UrlParts)
    UrlParse urlString, parts
END SUB

' Build URL from parts
FUNCTION url_build AS STRING (parts AS UrlParts)
    url_build = UrlFormat$(parts)
END FUNCTION

' Encode / Decode
FUNCTION encode AS STRING (str AS STRING)
    encode = UrlEncode$(str)
END FUNCTION

FUNCTION decode AS STRING (str AS STRING)
    decode = UrlDecode$(str)
END FUNCTION

' Query parameters
SUB param_set (parts AS UrlParts, KEY AS STRING, value AS STRING)
    UrlSetQueryParam parts, KEY, value
END SUB

FUNCTION param_get AS STRING (parts AS UrlParts, KEY AS STRING)
    param_get = UrlGetQueryParam$(parts, KEY)
END FUNCTION

' Path helpers
FUNCTION path_join AS STRING (basePath AS STRING, path AS STRING)
    path_join = UrlJoin$(basePath, path)
END FUNCTION

FUNCTION path_dir AS STRING (path AS STRING)
    path_dir = UrlDirname$(path)
END FUNCTION

FUNCTION path_file AS STRING (path AS STRING, ext AS STRING)
    path_file = UrlBasename$(path, ext)
END FUNCTION

'-------------------------------------------------------------------------------
' MODULE INFORMATION
'-------------------------------------------------------------------------------

SUB Stdlib_PrintInfo
    PRINT "========================================"
    PRINT STDLIB_NAME
    PRINT "Version: "; STDLIB_VERSION
    PRINT ""
    PRINT "Simple API (clean syntax):"
    PRINT "  JSON:  json_parse, json_string"
    PRINT "  URL:   encode, decode"
    PRINT ""
    PRINT "Usage:"
    PRINT "  IMPORT qbnex         'Import entire stdlib"
    PRINT '  result = get("http://api.example.com")'
    PRINT "========================================"
END SUB

FUNCTION Stdlib_GetVersion AS STRING ()
    Stdlib_GetVersion = STDLIB_VERSION
END FUNCTION

FUNCTION Stdlib_GetName AS STRING ()
    Stdlib_GetName = STDLIB_NAME
END FUNCTION

' Backward-compatible aliases used by earlier stdlib entrypoints.
FUNCTION QBNex_StdLib_Version$ ()
    QBNex_StdLib_Version$ = Stdlib_GetVersion()
END FUNCTION

FUNCTION QBNex_StdLib_Info$ ()
    DIM text AS STRING

    text = STDLIB_NAME + " v" + STDLIB_VERSION + CHR$(13) + CHR$(10)
    text = text + "Loaded via IMPORT qbnex"
    QBNex_StdLib_Info$ = text
END FUNCTION

SUB QBNex_StdLib_PrintInfo ()
    Stdlib_PrintInfo
END SUB
