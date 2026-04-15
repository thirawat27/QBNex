'===============================================================================
' QBNex Standard Library - Main Import File
'===============================================================================
' Modern API for QBNex - Simple, clean function names
'
' Usage:
'   IMPORT qbnex              'Import entire stdlib
'   IMPORT net.http           'Import HTTP module only
'   IMPORT json               'Import JSON module only
'
' Simple API (clean names):
' - get, post, put, delete         : HTTP requests
' - json_parse, json_string        : JSON handling
' - encode, decode                 : URL encoding
' - server, route_get, listen      : Web server
'===============================================================================

'-------------------------------------------------------------------------------
' STDLIB VERSION
'-------------------------------------------------------------------------------

CONST STDLIB_VERSION = "1.0.0"
CONST STDLIB_NAME = "QBNex Standard Library"

'-------------------------------------------------------------------------------
' MODULE INCLUDES
'-------------------------------------------------------------------------------

'--- Networking Modules ---
'$INCLUDE:'source\stdlib\net\http_client.bas'
'$INCLUDE:'source\stdlib\net\http_server.bas'

'--- Data Format Modules ---
'$INCLUDE:'source\stdlib\json.bas'
'$INCLUDE:'source\stdlib\url.bas'

'-------------------------------------------------------------------------------
' STDLIB INITIALIZATION
'-------------------------------------------------------------------------------

SUB Stdlib_Init
    'Initialize all stdlib modules
    HttpClient_Init
    HttpServer_Init
    Json_Init
    Url_Init
    
    PRINT "QBNex Standard Library v"; STDLIB_VERSION; " loaded."
END SUB

SUB Stdlib_Cleanup
    'Cleanup all stdlib modules
    HttpClient_Cleanup
    HttpServer_Cleanup
    Json_Cleanup
END SUB

'-------------------------------------------------------------------------------
' HIGH-LEVEL API WRAPPERS
'-------------------------------------------------------------------------------

'--- HTTP Client API (Web Requests) ---
' Functions available from net/http_client.bas:
' - get, post, put, delete, fetch (return STRING)

'--- HTTP Server API (Web Server) ---

' Create HTTP server on port
FUNCTION server AS INTEGER (port AS INTEGER)
    server = HttpServer_Create(port)
END FUNCTION

' Add GET route
SUB route_get (server AS HttpServer, path AS STRING, handlerIndex AS INTEGER)
    HttpServer_Get server, path, handlerIndex
END SUB

' Add POST route
SUB route_post (server AS HttpServer, path AS STRING, handlerIndex AS INTEGER)
    HttpServer_Post server, path, handlerIndex
END SUB

' Start server
SUB listen (server AS HttpServer)
    HttpServer_Start server
END SUB

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

SUB obj_set (objectIndex AS INTEGER, key AS STRING, valueIndex AS INTEGER)
    JsonObjectSet objectIndex, key, valueIndex
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
SUB param_set (parts AS UrlParts, key AS STRING, value AS STRING)
    UrlSetQueryParam parts, key, value
END SUB

FUNCTION param_get AS STRING (parts AS UrlParts, key AS STRING)
    param_get = UrlGetQueryParam$(parts, key)
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
    PRINT "  HTTP:  get, post, put, delete"
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

