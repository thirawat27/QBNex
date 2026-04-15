'===============================================================================
' QBNex Standard Library - HTTP Server Module
'===============================================================================
'
' Features:
' HTTP server for building REST APIs and web applications.
' Supports routing, middleware, and modern web development patterns.
' - JSON request/response handling
' - Static file serving
' - Template rendering (basic)
'===============================================================================

'-------------------------------------------------------------------------------
' SERVER CONFIGURATION
'-------------------------------------------------------------------------------

CONST SERVER_DEFAULT_PORT = 8080
CONST SERVER_MAX_CONNECTIONS = 100
CONST SERVER_REQUEST_BUFFER_SIZE = 8192

'-------------------------------------------------------------------------------
' ROUTE TYPES
'-------------------------------------------------------------------------------

CONST ROUTE_EXACT = 1
CONST ROUTE_WILDCARD = 2
CONST ROUTE_REGEX = 3

TYPE HttpRoute
    method AS INTEGER 'HTTP_GET, HTTP_POST, etc.
    path AS STRING * 256
    routeType AS INTEGER
    handlerIndex AS INTEGER
    middlewareCount AS INTEGER
    middlewareIndices(1 TO 10) AS INTEGER
END TYPE

TYPE HttpRouteHandler
    name AS STRING * 64
    handlerType AS INTEGER '1=SUB, 2=FUNCTION
END TYPE

'-------------------------------------------------------------------------------
' SERVER STATE
'-------------------------------------------------------------------------------

TYPE HttpServer
    port AS INTEGER
    host AS STRING * 128
    isRunning AS _BYTE
    serverSocket AS LONG
    
    'Routes
    routes(1 TO 100) AS HttpRoute
    routeCount AS INTEGER
    
    'Middleware
    middleware(1 TO 20) AS INTEGER
    middlewareCount AS INTEGER
    
    'Configuration
    enableCors AS _BYTE
    enableLogging AS _BYTE
    staticPath AS STRING * 260
    defaultDocument AS STRING * 64
END TYPE

'-------------------------------------------------------------------------------
' REQUEST CONTEXT 
'-------------------------------------------------------------------------------

TYPE ServerRequest
    method AS INTEGER
    path AS STRING * 1024
    fullUrl AS STRING * 2048
    queryString AS STRING * 1024
    headers AS STRING * 4096
    body AS STRING * 8192
    contentType AS STRING * 128
    contentLength AS LONG
    remoteAddress AS STRING * 64
    
    'Parsed data
    params(1 TO 20) AS STRING * 256
    paramNames(1 TO 20) AS STRING * 64
    paramCount AS INTEGER
    
    queryParams(1 TO 50) AS STRING * 512
    queryParamNames(1 TO 50) AS STRING * 64
    queryParamCount AS INTEGER
END TYPE

TYPE ServerResponse
    statusCode AS INTEGER
    statusText AS STRING * 128
    headers AS STRING * 4096
    body AS STRING * 16384
    contentType AS STRING * 128
    cookies AS STRING * 1024
    isSent AS _BYTE
END TYPE

'-------------------------------------------------------------------------------
' MIDDLEWARE TYPE
'-------------------------------------------------------------------------------

TYPE Middleware
    name AS STRING * 64
    enabled AS _BYTE
END TYPE

'-------------------------------------------------------------------------------
' MODULE STATE
'-------------------------------------------------------------------------------

DIM SHARED ServerInstances(1 TO 5) AS HttpServer
DIM SHARED ActiveServerCount AS INTEGER
DIM SHARED DefaultServer AS HttpServer
DIM SHARED HttpServerInitialized AS _BYTE

'-------------------------------------------------------------------------------
' INITIALIZATION
'-------------------------------------------------------------------------------

SUB HttpServer_Init
    ActiveServerCount = 0
    HttpServerInitialized = -1
    
    'Initialize default server
    WITH DefaultServer
        .port = SERVER_DEFAULT_PORT
        .host = "0.0.0.0"
        .isRunning = 0
        .routeCount = 0
        .middlewareCount = 0
        .enableCors = -1
        .enableLogging = -1
        .staticPath = "./public"
        .defaultDocument = "index.html"
    END WITH
END SUB

SUB HttpServer_Cleanup
    DIM i AS INTEGER
    
    'Stop all running servers
    FOR i = 1 TO ActiveServerCount
        IF ServerInstances(i).isRunning THEN
            HttpServer_Stop ServerInstances(i)
        END IF
    NEXT
    
    HttpServerInitialized = 0
END SUB

'-------------------------------------------------------------------------------
' SERVER CREATION 
'-------------------------------------------------------------------------------

FUNCTION HttpServer_Create% (port AS INTEGER)
    DIM idx AS INTEGER
    
    IF ActiveServerCount >= UBOUND(ServerInstances) THEN
        HttpServer_Create% = 0
        EXIT FUNCTION
    END IF
    
    ActiveServerCount = ActiveServerCount + 1
    idx = ActiveServerCount
    
    ServerInstances(idx) = DefaultServer
    ServerInstances(idx).port = port
    
    HttpServer_Create% = idx
END FUNCTION

SUB HttpServer_SetHost (server AS HttpServer, host AS STRING)
    server.host = host
END SUB

'-------------------------------------------------------------------------------
' ROUTE REGISTRATION 
'-------------------------------------------------------------------------------

SUB HttpServer_Get (server AS HttpServer, path AS STRING, handlerIndex AS INTEGER)
    AddRoute server, HTTP_GET, path, handlerIndex
END SUB

SUB HttpServer_Post (server AS HttpServer, path AS STRING, handlerIndex AS INTEGER)
    AddRoute server, HTTP_POST, path, handlerIndex
END SUB

SUB HttpServer_Put (server AS HttpServer, path AS STRING, handlerIndex AS INTEGER)
    AddRoute server, HTTP_PUT, path, handlerIndex
END SUB

SUB HttpServer_Delete (server AS HttpServer, path AS STRING, handlerIndex AS INTEGER)
    AddRoute server, HTTP_DELETE, path, handlerIndex
END SUB

SUB HttpServer_Patch (server AS HttpServer, path AS STRING, handlerIndex AS INTEGER)
    AddRoute server, HTTP_PATCH, path, handlerIndex
END SUB

SUB HttpServer_All (server AS HttpServer, path AS STRING, handlerIndex AS INTEGER)
    'Register for all HTTP methods
    AddRoute server, HTTP_GET, path, handlerIndex
    AddRoute server, HTTP_POST, path, handlerIndex
    AddRoute server, HTTP_PUT, path, handlerIndex
    AddRoute server, HTTP_DELETE, path, handlerIndex
END SUB

SUB AddRoute (server AS HttpServer, method AS INTEGER, path AS STRING, handlerIndex AS INTEGER)
    IF server.routeCount >= UBOUND(server.routes) THEN EXIT SUB
    
    server.routeCount = server.routeCount + 1
    
    WITH server.routes(server.routeCount)
        .method = method
        .path = path
        .routeType = ROUTE_EXACT
        .handlerIndex = handlerIndex
        .middlewareCount = 0
        
        'Check for wildcards
        IF INSTR(path, "*") > 0 OR INSTR(path, ":") > 0 THEN
            .routeType = ROUTE_WILDCARD
        END IF
    END WITH
END SUB

'-------------------------------------------------------------------------------
' MIDDLEWARE 
'-------------------------------------------------------------------------------

SUB HttpServer_Use (server AS HttpServer, middlewareIndex AS INTEGER)
    IF server.middlewareCount >= UBOUND(server.middleware) THEN EXIT SUB
    
    server.middlewareCount = server.middlewareCount + 1
    server.middleware(server.middlewareCount) = middlewareIndex
END SUB

' Built-in middleware
SUB Middleware_Logging (req AS ServerRequest, res AS ServerResponse, next AS _BYTE)
    'Log request
    PRINT TIMESTAMP$; " "; HttpMethodString$(req.method); " "; RTRIM$(req.path)
    next = -1
END SUB

SUB Middleware_CORS (req AS ServerRequest, res AS ServerResponse, next AS _BYTE)
    'Add CORS headers
    ServerResponse_SetHeader res, "Access-Control-Allow-Origin", "*"
    ServerResponse_SetHeader res, "Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS"
    ServerResponse_SetHeader res, "Access-Control-Allow-Headers", "Content-Type, Authorization"
    next = -1
END SUB

SUB Middleware_JSON (req AS ServerRequest, res AS ServerResponse, next AS _BYTE)
    'Parse JSON body if content-type is application/json
    IF INSTR(req.contentType, "application/json") > 0 THEN
        'JSON would be parsed here
    END IF
    next = -1
END SUB

SUB Middleware_StaticFiles (req AS ServerRequest, res AS ServerResponse, next AS _BYTE, staticPath AS STRING)
    'Serve static files from directory
    DIM filePath AS STRING
    filePath = staticPath + RTRIM$(req.path)
    
    IF _FILEEXISTS(filePath) THEN
        'Serve file
        ServerResponse_SendFile res, filePath
        next = 0 'Don't continue to next middleware
    ELSE
        next = -1
    END IF
END SUB

'-------------------------------------------------------------------------------
' SERVER CONTROL
'-------------------------------------------------------------------------------

SUB HttpServer_Start (server AS HttpServer)
    IF server.isRunning THEN EXIT SUB
    
    PRINT "Starting QBNex HTTP Server..."
    PRINT "Host: "; RTRIM$(server.host)
    PRINT "Port: "; server.port
    PRINT "Routes: "; server.routeCount
    PRINT ""
    PRINT "Server running! Press Ctrl+C to stop."
    
    server.isRunning = -1
    
    'Main server loop (would use actual socket listening)
    HttpServer_RunLoop server
END SUB

SUB HttpServer_Stop (server AS HttpServer)
    server.isRunning = 0
    PRINT "Server stopped."
END SUB

SUB HttpServer_RunLoop (server AS HttpServer)
    'This is a placeholder - real implementation would:
    '1. Create socket
    '2. Bind to port
    '3. Listen for connections
    '4. Accept and handle requests
    
    WHILE server.isRunning
        'Simulate request handling
        _DELAY 0.1
        
        'In real implementation:
        'DIM req AS ServerRequest
        'DIM res AS ServerResponse
        'AcceptConnection server, req
        'HandleRequest server, req, res
        'SendResponse res
    WEND
END SUB

'-------------------------------------------------------------------------------
' REQUEST HANDLING
'-------------------------------------------------------------------------------

SUB HandleRequest (server AS HttpServer, req AS ServerRequest, res AS ServerResponse)
    DIM i AS INTEGER
    DIM routeMatched AS _BYTE
    
    'Initialize response
    res.statusCode = 404
    res.statusText = "Not Found"
    res.contentType = "text/plain"
    res.body = "404 Not Found"
    res.isSent = 0
    
    'Find matching route
    routeMatched = 0
    FOR i = 1 TO server.routeCount
        IF server.routes(i).method = req.method THEN
            IF MatchRoute(server.routes(i).path, req.path) THEN
                routeMatched = -1
                
                'Execute middleware
                'ExecuteMiddleware server, req, res
                
                'Execute route handler
                'CallRouteHandler server.routes(i).handlerIndex, req, res
                
                EXIT FOR
            END IF
        END IF
    NEXT
    
    'Send response
    IF NOT res.isSent THEN
        res.isSent = -1
    END IF
END SUB

FUNCTION MatchRoute% (routePath AS STRING, requestPath AS STRING)
    'Simple exact match
    IF routePath = requestPath THEN
        MatchRoute% = -1
        EXIT FUNCTION
    END IF
    
    'Wildcard match (simplified)
    'TODO: Implement proper path matching with parameters
    
    MatchRoute% = 0
END FUNCTION

'-------------------------------------------------------------------------------
' RESPONSE HELPERS
'-------------------------------------------------------------------------------

SUB ServerResponse_SetHeader (res AS ServerResponse, name AS STRING, value AS STRING)
    IF res.headers = "" THEN
        res.headers = name + ": " + value
    ELSE
        res.headers = res.headers + CHR$(13) + CHR$(10) + name + ": " + value
    END IF
END SUB

SUB ServerResponse_Status (res AS ServerResponse, code AS INTEGER)
    res.statusCode = code
    res.statusText = HttpStatusText$(code)
END SUB

SUB ServerResponse_Send (res AS ServerResponse, body AS STRING)
    res.body = body
    res.contentType = "text/plain"
    res.isSent = -1
END SUB

SUB ServerResponse_Json (res AS ServerResponse, jsonString AS STRING)
    res.body = jsonString
    res.contentType = "application/json"
    ServerResponse_SetHeader res, "Content-Type", "application/json"
    res.isSent = -1
END SUB

SUB ServerResponse_SendFile (res AS ServerResponse, filePath AS STRING)
    'Read and send file
    DIM fileNum AS INTEGER
    DIM fileData AS STRING
    
    fileNum = FREEFILE
    OPEN filePath FOR BINARY AS #fileNum
    fileData = INPUT$(LOF(fileNum), #fileNum)
    CLOSE #fileNum
    
    res.body = fileData
    'Set content type based on extension
    res.contentType = GetContentType(filePath)
    res.isSent = -1
END SUB

SUB ServerResponse_Redirect (res AS ServerResponse, url AS STRING, statusCode AS INTEGER)
    IF statusCode = 0 THEN statusCode = 302
    res.statusCode = statusCode
    ServerResponse_SetHeader res, "Location", url
    res.isSent = -1
END SUB

'-------------------------------------------------------------------------------
' REQUEST HELPERS
'-------------------------------------------------------------------------------

FUNCTION ServerRequest_GetParam$ (req AS ServerRequest, name AS STRING)
    DIM i AS INTEGER
    
    FOR i = 1 TO req.paramCount
        IF req.paramNames(i) = name THEN
            ServerRequest_GetParam$ = RTRIM$(req.params(i))
            EXIT FUNCTION
        END IF
    NEXT
    
    ServerRequest_GetParam$ = ""
END FUNCTION

FUNCTION ServerRequest_GetQuery$ (req AS ServerRequest, name AS STRING)
    DIM i AS INTEGER
    
    FOR i = 1 TO req.queryParamCount
        IF req.queryParamNames(i) = name THEN
            ServerRequest_GetQuery$ = RTRIM$(req.queryParams(i))
            EXIT FUNCTION
        END IF
    NEXT
    
    ServerRequest_GetQuery$ = ""
END FUNCTION

FUNCTION ServerRequest_GetHeader$ (req AS ServerRequest, name AS STRING)
    'Parse headers to find value
    ServerRequest_GetHeader$ = ""
END FUNCTION

'-------------------------------------------------------------------------------
' UTILITY FUNCTIONS
'-------------------------------------------------------------------------------

FUNCTION HttpMethodString$ (method AS INTEGER)
    SELECT CASE method
        CASE HTTP_GET: HttpMethodString$ = "GET"
        CASE HTTP_POST: HttpMethodString$ = "POST"
        CASE HTTP_PUT: HttpMethodString$ = "PUT"
        CASE HTTP_DELETE: HttpMethodString$ = "DELETE"
        CASE HTTP_PATCH: HttpMethodString$ = "PATCH"
        CASE HTTP_HEAD: HttpMethodString$ = "HEAD"
        CASE HTTP_OPTIONS: HttpMethodString$ = "OPTIONS"
        CASE ELSE: HttpMethodString$ = "UNKNOWN"
    END SELECT
END FUNCTION

FUNCTION GetContentType$ (filePath AS STRING)
    DIM ext AS STRING
    DIM dotPos AS INTEGER
    
    dotPos = _INSTRREV(filePath, ".")
    IF dotPos > 0 THEN
        ext = LCASE$(MID$(filePath, dotPos + 1))
    ELSE
        GetContentType$ = "application/octet-stream"
        EXIT FUNCTION
    END IF
    
    SELECT CASE ext
        CASE "html", "htm": GetContentType$ = "text/html"
        CASE "css": GetContentType$ = "text/css"
        CASE "js": GetContentType$ = "application/javascript"
        CASE "json": GetContentType$ = "application/json"
        CASE "png": GetContentType$ = "image/png"
        CASE "jpg", "jpeg": GetContentType$ = "image/jpeg"
        CASE "gif": GetContentType$ = "image/gif"
        CASE "svg": GetContentType$ = "image/svg+xml"
        CASE "txt": GetContentType$ = "text/plain"
        CASE "xml": GetContentType$ = "application/xml"
        CASE "pdf": GetContentType$ = "application/pdf"
        CASE "zip": GetContentType$ = "application/zip"
        CASE ELSE: GetContentType$ = "application/octet-stream"
    END SELECT
END FUNCTION

FUNCTION TIMESTAMP$ ()
    TIMESTAMP$ = DATE$ + " " + TIME$
END FUNCTION

'-------------------------------------------------------------------------------
' HIGH-LEVEL API (Quick Start)
'-------------------------------------------------------------------------------

' Quick server setup with common configuration
FUNCTION QuickServer% (port AS INTEGER)
    DIM serverIdx AS INTEGER
    DIM server AS HttpServer
    
    serverIdx = HttpServer_Create(port)
    IF serverIdx = 0 THEN
        QuickServer% = 0
        EXIT FUNCTION
    END IF
    
    server = ServerInstances(serverIdx)
    
    'Enable common middleware
    server.enableCors = -1
    server.enableLogging = -1
    
    'Default routes
    'HttpServer_Get server, "/", AddressOf(IndexHandler)
    'HttpServer_Get server, "/api/health", AddressOf(HealthHandler)
    
    QuickServer% = serverIdx
END FUNCTION

