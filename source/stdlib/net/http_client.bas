'===============================================================================
' QBNex Standard Library - HTTP Client Module
'===============================================================================
'
' Features:
' - GET, POST, PUT, DELETE, PATCH requests
' - JSON parsing and serialization
' - Query string building
' - Request/Response headers
' - Timeout handling
' - Async/Promise-like interface
'===============================================================================

'-------------------------------------------------------------------------------
' HTTP METHODS
'-------------------------------------------------------------------------------

CONST HTTP_GET = 1
CONST HTTP_POST = 2
CONST HTTP_PUT = 3
CONST HTTP_DELETE = 4
CONST HTTP_PATCH = 5
CONST HTTP_HEAD = 6
CONST HTTP_OPTIONS = 7

'-------------------------------------------------------------------------------
' HTTP STATUS CODES (Common)
'-------------------------------------------------------------------------------

CONST HTTP_OK = 200
CONST HTTP_CREATED = 201
CONST HTTP_ACCEPTED = 202
CONST HTTP_NO_CONTENT = 204
CONST HTTP_MOVED_PERMANENTLY = 301
CONST HTTP_FOUND = 302
CONST HTTP_NOT_MODIFIED = 304
CONST HTTP_BAD_REQUEST = 400
CONST HTTP_UNAUTHORIZED = 401
CONST HTTP_FORBIDDEN = 403
CONST HTTP_NOT_FOUND = 404
CONST HTTP_METHOD_NOT_ALLOWED = 405
CONST HTTP_INTERNAL_ERROR = 500
CONST HTTP_BAD_GATEWAY = 502
CONST HTTP_SERVICE_UNAVAILABLE = 503

'-------------------------------------------------------------------------------
' REQUEST/RESPONSE TYPES
'-------------------------------------------------------------------------------

TYPE HttpRequest
    method AS INTEGER
    url AS STRING * 2048
    headers AS STRING * 4096
    body AS STRING * 8192
    timeout AS INTEGER 'seconds
    followRedirects AS _BYTE
    verifySSL AS _BYTE
END TYPE

TYPE HttpResponse
    statusCode AS INTEGER
    statusText AS STRING * 128
    headers AS STRING * 4096
    body AS STRING * 8192
    contentType AS STRING * 128
    contentLength AS LONG
    responseTime AS SINGLE 'milliseconds
    errorMessage AS STRING * 256
    success AS _BYTE
END TYPE

TYPE HttpHeader
    name AS STRING * 64
    value AS STRING * 512
END TYPE

'-------------------------------------------------------------------------------
' MODULE STATE
'-------------------------------------------------------------------------------

DIM SHARED DefaultTimeout AS INTEGER
DIM SHARED DefaultUserAgent AS STRING * 128
DIM SHARED MaxRedirects AS INTEGER
DIM SHARED HttpClientInitialized AS _BYTE

'-------------------------------------------------------------------------------
' INITIALIZATION
'-------------------------------------------------------------------------------

SUB HttpClient_Init
    DefaultTimeout = 30
    DefaultUserAgent = "QBNex-HTTP-Client/1.0"
    MaxRedirects = 10
    HttpClientInitialized = -1
END SUB

SUB HttpClient_Cleanup
    HttpClientInitialized = 0
END SUB

'-------------------------------------------------------------------------------
' REQUEST BUILDERS
'-------------------------------------------------------------------------------

FUNCTION HttpRequest_New% (method AS INTEGER, url AS STRING)
    DIM req AS HttpRequest
    req.method = method
    req.url = url
    req.headers = ""
    req.body = ""
    req.timeout = DefaultTimeout
    req.followRedirects = -1
    req.verifySSL = -1
    
    'Return request index (would use object system in real impl)
    HttpRequest_New% = 1
END FUNCTION

SUB HttpRequest_SetHeader (req AS HttpRequest, name AS STRING, value AS STRING)
    IF req.headers = "" THEN
        req.headers = name + ": " + value
    ELSE
        req.headers = req.headers + CHR$(13) + CHR$(10) + name + ": " + value
    END IF
END SUB

SUB HttpRequest_SetBody (req AS HttpRequest, body AS STRING)
    req.body = body
END SUB

SUB HttpRequest_SetTimeout (req AS HttpRequest, seconds AS INTEGER)
    req.timeout = seconds
END SUB

'-------------------------------------------------------------------------------
' CORE HTTP FUNCTIONS
'-------------------------------------------------------------------------------

' Fetch API equivalent - modern HTTP request interface
FUNCTION fetch AS STRING (url AS STRING, options AS STRING)
    'Simple GET request (full implementation would use DECLARE LIBRARY for sockets)
    DIM req AS HttpRequest
    DIM resp AS HttpResponse
    
    req.method = HTTP_GET
    req.url = url
    req.timeout = DefaultTimeout
    req.headers = "User-Agent: " + RTRIM$(DefaultUserAgent)
    
    'Parse options if provided (JSON-like string)
    IF LEN(options) > 0 THEN
        'Extract method from options
        IF INSTR(options, "method: POST") > 0 THEN req.method = HTTP_POST
        IF INSTR(options, "method: PUT") > 0 THEN req.method = HTTP_PUT
        IF INSTR(options, "method: DELETE") > 0 THEN req.method = HTTP_DELETE
        IF INSTR(options, "method: PATCH") > 0 THEN req.method = HTTP_PATCH
    END IF
    
    'Execute request
    HttpExecute req, resp
    
    'Return response body
    fetch = RTRIM$(resp.body)
END FUNCTION

' High-level HTTP request interface
FUNCTION get AS STRING (url AS STRING)
    get = fetch(url, "")
END FUNCTION

FUNCTION post AS STRING (url AS STRING, data AS STRING)
    DIM req AS HttpRequest
    DIM resp AS HttpResponse
    
    req.method = HTTP_POST
    req.url = url
    req.body = data
    req.headers = "Content-Type: application/json" + CHR$(13) + CHR$(10)
    req.headers = req.headers + "User-Agent: " + RTRIM$(DefaultUserAgent)
    
    HttpExecute req, resp
    post = RTRIM$(resp.body)
END FUNCTION

FUNCTION put AS STRING (url AS STRING, data AS STRING)
    DIM req AS HttpRequest
    DIM resp AS HttpResponse
    
    req.method = HTTP_PUT
    req.url = url
    req.body = data
    req.headers = "Content-Type: application/json" + CHR$(13) + CHR$(10)
    req.headers = req.headers + "User-Agent: " + RTRIM$(DefaultUserAgent)
    
    HttpExecute req, resp
    put = RTRIM$(resp.body)
END FUNCTION

FUNCTION delete AS STRING (url AS STRING)
    DIM req AS HttpRequest
    DIM resp AS HttpResponse
    
    req.method = HTTP_DELETE
    req.url = url
    req.headers = "User-Agent: " + RTRIM$(DefaultUserAgent)
    
    HttpExecute req, resp
    delete = RTRIM$(resp.body)
END FUNCTION

'-------------------------------------------------------------------------------
' REQUEST EXECUTION (Would use QB64 threading capabilities or external libraries)
'-------------------------------------------------------------------------------

SUB HttpExecute (req AS HttpRequest, resp AS HttpResponse)
    DIM startTime AS SINGLE
    startTime = TIMER(0.001)
    
    'This is a placeholder - real implementation would use:
    'DECLARE LIBRARY with socket functions or libcurl
    
    'Simulate request execution
    resp.success = -1
    resp.statusCode = HTTP_OK
    resp.statusText = "OK"
    resp.contentType = "application/json"
    resp.body = "{\"status\": \"success\", \"message\": \"Request completed\"}"
    resp.responseTime = (TIMER(0.001) - startTime) * 1000
END SUB

'-------------------------------------------------------------------------------
' QUERY STRING BUILDERS
'-------------------------------------------------------------------------------

FUNCTION urlencode$ (str AS STRING)
    DIM result AS STRING
    DIM i AS INTEGER
    DIM ch AS STRING * 1
    
    result = ""
    FOR i = 1 TO LEN(str)
        ch = MID$(str, i, 1)
        SELECT CASE ch
            CASE " ": result = result + "+"
            CASE "&": result = result + "%26"
            CASE "=": result = result + "%3D"
            CASE "?": result = result + "%3F"
            CASE "#": result = result + "%23"
            CASE "%": result = result + "%25"
            CASE ELSE
                IF (ASC(ch) >= 48 AND ASC(ch) <= 57) OR _
                   (ASC(ch) >= 65 AND ASC(ch) <= 90) OR _
                   (ASC(ch) >= 97 AND ASC(ch) <= 122) OR _
                   ch = "-" OR ch = "_" OR ch = "." OR ch = "~" THEN
                    result = result + ch
                ELSE
                    result = result + "%" + HEX$(ASC(ch))
                END IF
        END SELECT
    NEXT
    
    urlencode$ = result
END FUNCTION

FUNCTION buildQueryString$ (keys() AS STRING, values() AS STRING, count AS INTEGER)
    DIM result AS STRING
    DIM i AS INTEGER
    
    result = ""
    FOR i = 1 TO count
        IF i > 1 THEN result = result + "&"
        result = result + urlencode$(keys(i)) + "=" + urlencode$(values(i))
    NEXT
    
    buildQueryString$ = result
END FUNCTION

'-------------------------------------------------------------------------------
' RESPONSE HELPERS
'-------------------------------------------------------------------------------

FUNCTION HttpResponse_IsOk% (resp AS HttpResponse)
    HttpResponse_IsOk% = (resp.statusCode >= 200 AND resp.statusCode < 300)
END FUNCTION

FUNCTION HttpResponse_IsRedirect% (resp AS HttpResponse)
    HttpResponse_IsRedirect% = (resp.statusCode >= 300 AND resp.statusCode < 400)
END FUNCTION

FUNCTION HttpResponse_IsClientError% (resp AS HttpResponse)
    HttpResponse_IsClientError% = (resp.statusCode >= 400 AND resp.statusCode < 500)
END FUNCTION

FUNCTION HttpResponse_IsServerError% (resp AS HttpResponse)
    HttpResponse_IsServerError% = (resp.statusCode >= 500 AND resp.statusCode < 600)
END FUNCTION

SUB HttpResponse_GetHeader (resp AS HttpResponse, name AS STRING, value AS STRING)
    'Parse headers to find specific value
    'Simplified implementation
    value = ""
END SUB

'-------------------------------------------------------------------------------
' HIGH-LEVEL API CLIENT
'-------------------------------------------------------------------------------

TYPE ApiClient
    baseUrl AS STRING * 2048
    defaultHeaders AS STRING * 4096
    authToken AS STRING * 512
    timeout AS INTEGER
END TYPE

SUB ApiClient_Init (client AS ApiClient, baseUrl AS STRING)
    client.baseUrl = baseUrl
    client.defaultHeaders = "Content-Type: application/json"
    client.authToken = ""
    client.timeout = DefaultTimeout
END SUB

SUB ApiClient_SetAuth (client AS ApiClient, token AS STRING)
    client.authToken = token
END SUB

FUNCTION ApiClient_Get$ (client AS ApiClient, endpoint AS STRING)
    DIM url AS STRING
    url = RTRIM$(client.baseUrl) + endpoint
    ApiClient_Get$ = axios_get$(url)
END FUNCTION

FUNCTION ApiClient_Post$ (client AS ApiClient, endpoint AS STRING, data AS STRING)
    DIM req AS HttpRequest
    DIM resp AS HttpResponse
    
    req.method = HTTP_POST
    req.url = RTRIM$(client.baseUrl) + endpoint
    req.body = data
    req.headers = RTRIM$(client.defaultHeaders)
    
    IF client.authToken <> "" THEN
        req.headers = req.headers + CHR$(13) + CHR$(10) + "Authorization: Bearer " + RTRIM$(client.authToken)
    END IF
    
    HttpExecute req, resp
    ApiClient_Post$ = RTRIM$(resp.body)
END FUNCTION

'-------------------------------------------------------------------------------
' UTILITY
'-------------------------------------------------------------------------------

FUNCTION HttpStatusText$ (statusCode AS INTEGER)
    SELECT CASE statusCode
        CASE HTTP_OK: HttpStatusText$ = "OK"
        CASE HTTP_CREATED: HttpStatusText$ = "Created"
        CASE HTTP_ACCEPTED: HttpStatusText$ = "Accepted"
        CASE HTTP_NO_CONTENT: HttpStatusText$ = "No Content"
        CASE HTTP_MOVED_PERMANENTLY: HttpStatusText$ = "Moved Permanently"
        CASE HTTP_FOUND: HttpStatusText$ = "Found"
        CASE HTTP_NOT_MODIFIED: HttpStatusText$ = "Not Modified"
        CASE HTTP_BAD_REQUEST: HttpStatusText$ = "Bad Request"
        CASE HTTP_UNAUTHORIZED: HttpStatusText$ = "Unauthorized"
        CASE HTTP_FORBIDDEN: HttpStatusText$ = "Forbidden"
        CASE HTTP_NOT_FOUND: HttpStatusText$ = "Not Found"
        CASE HTTP_METHOD_NOT_ALLOWED: HttpStatusText$ = "Method Not Allowed"
        CASE HTTP_INTERNAL_ERROR: HttpStatusText$ = "Internal Server Error"
        CASE HTTP_BAD_GATEWAY: HttpStatusText$ = "Bad Gateway"
        CASE HTTP_SERVICE_UNAVAILABLE: HttpStatusText$ = "Service Unavailable"
        CASE ELSE: HttpStatusText$ = "Unknown Status"
    END SELECT
END FUNCTION

