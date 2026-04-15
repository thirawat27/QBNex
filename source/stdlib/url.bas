'===============================================================================
' QBNex Standard Library - URL Module
'===============================================================================
' URL parsing and manipulation for QBNex.
' Provides comprehensive URL handling and query string management.
'
' Features:
' - URL parsing into components
' - URL encoding/decoding
' - Query string parsing
' - URL building and manipulation
'===============================================================================

'-------------------------------------------------------------------------------
' URL PARTS TYPE
'-------------------------------------------------------------------------------

TYPE UrlParts
    href AS STRING * 2048        'Full URL
    protocol AS STRING * 16      'http:, https:, ftp:, etc.
    username AS STRING * 128     'Username for auth
    password AS STRING * 128     'Password for auth
    hostname AS STRING * 256     'Domain or IP
    port AS INTEGER              'Port number
    pathname AS STRING * 1024    'Path after hostname
    search AS STRING * 2048      'Query string with ?
    hash AS STRING * 512         'Fragment with #
    
    'Derived properties
    host AS STRING * 268         'hostname:port
    origin AS STRING * 2048      'protocol//hostname:port
    
    'Parsed query parameters
    queryCount AS INTEGER
    queryKeys(1 TO 100) AS STRING * 64
    queryValues(1 TO 100) AS STRING * 512
END TYPE

'-------------------------------------------------------------------------------
' INITIALIZATION
'-------------------------------------------------------------------------------

SUB Url_Init
    'Nothing to initialize for this module
END SUB

'-------------------------------------------------------------------------------
' URL PARSING (Parse URL into components)
'-------------------------------------------------------------------------------

SUB UrlParse (urlString AS STRING, parts AS UrlParts)
    DIM pos AS INTEGER
    DIM temp AS STRING
    
    parts.href = urlString
    
    'Parse protocol
    pos = INSTR(urlString, "://")
    IF pos > 0 THEN
        parts.protocol = LEFT$(urlString, pos + 2)
        temp = MID$(urlString, pos + 4)
    ELSE
        'No protocol - assume relative URL
        parts.protocol = ""
        temp = urlString
    END IF
    
    'Parse hash/fragment
    pos = INSTR(temp, "#")
    IF pos > 0 THEN
        parts.hash = MID$(temp, pos)
        temp = LEFT$(temp, pos - 1)
    ELSE
        parts.hash = ""
    END IF
    
    'Parse search/query string
    pos = INSTR(temp, "?")
    IF pos > 0 THEN
        parts.search = MID$(temp, pos)
        temp = LEFT$(temp, pos - 1)
        
        'Parse query parameters
        UrlParseQueryString parts.search, parts
    ELSE
        parts.search = ""
        parts.queryCount = 0
    END IF
    
    'Parse username:password@hostname:port/path
    pos = INSTR(temp, "@")
    IF pos > 0 THEN
        DIM auth AS STRING
        auth = LEFT$(temp, pos - 1)
        temp = MID$(temp, pos + 1)
        
        DIM colonPos AS INTEGER
        colonPos = INSTR(auth, ":")
        IF colonPos > 0 THEN
            parts.username = LEFT$(auth, colonPos - 1)
            parts.password = MID$(auth, colonPos + 1)
        ELSE
            parts.username = auth
            parts.password = ""
        END IF
    ELSE
        parts.username = ""
        parts.password = ""
    END IF
    
    'Now temp should be hostname:port/path
    'Find where path starts
    pos = INSTR(temp, "/")
    IF pos > 0 THEN
        parts.pathname = MID$(temp, pos)
        temp = LEFT$(temp, pos - 1)
    ELSE
        parts.pathname = "/"
    END IF
    
    'Parse hostname:port
    pos = INSTR(temp, ":")
    IF pos > 0 THEN
        parts.hostname = LEFT$(temp, pos - 1)
        parts.port = VAL(MID$(temp, pos + 1))
    ELSE
        parts.hostname = temp
        'Default ports
        SELECT CASE parts.protocol
            CASE "http:": parts.port = 80
            CASE "https:": parts.port = 443
            CASE "ftp:": parts.port = 21
            CASE ELSE: parts.port = 0
        END SELECT
    END IF
    
    'Build derived properties
    IF parts.port > 0 AND parts.port <> 80 AND parts.port <> 443 THEN
        parts.host = RTRIM$(parts.hostname) + ":" + LTRIM$(STR$(parts.port))
    ELSE
        parts.host = RTRIM$(parts.hostname)
    END IF
    
    IF parts.protocol <> "" THEN
        parts.origin = RTRIM$(parts.protocol) + "//" + RTRIM$(parts.host)
    ELSE
        parts.origin = ""
    END IF
END SUB

'-------------------------------------------------------------------------------
' QUERY STRING PARSING
'-------------------------------------------------------------------------------

SUB UrlParseQueryString (queryString AS STRING, parts AS UrlParts)
    DIM temp AS STRING
    DIM pos AS INTEGER
    DIM ampPos AS INTEGER
    DIM eqPos AS INTEGER
    DIM key AS STRING
    DIM value AS STRING
    
    parts.queryCount = 0
    
    'Remove leading ? if present
    temp = queryString
    IF LEFT$(temp, 1) = "?" THEN
        temp = MID$(temp, 2)
    END IF
    
    DO
        ampPos = INSTR(temp, "&")
        IF ampPos > 0 THEN
            key = LEFT$(temp, ampPos - 1)
            temp = MID$(temp, ampPos + 1)
        ELSE
            key = temp
            temp = ""
        END IF
        
        eqPos = INSTR(key, "=")
        IF eqPos > 0 THEN
            value = UrlDecode$(MID$(key, eqPos + 1))
            key = UrlDecode$(LEFT$(key, eqPos - 1))
        ELSE
            value = ""
            key = UrlDecode$(key)
        END IF
        
        IF key <> "" AND parts.queryCount < UBOUND(parts.queryKeys) THEN
            parts.queryCount = parts.queryCount + 1
            parts.queryKeys(parts.queryCount) = key
            parts.queryValues(parts.queryCount) = value
        END IF
    LOOP UNTIL temp = ""
END SUB

'-------------------------------------------------------------------------------
' URL ENCODING/DECODING
'-------------------------------------------------------------------------------

FUNCTION UrlEncode$ (str AS STRING)
    DIM result AS STRING
    DIM i AS INTEGER
    DIM ch AS STRING * 1
    
    result = ""
    
    FOR i = 1 TO LEN(str)
        ch = MID$(str, i, 1)
        SELECT CASE ch
            CASE " ": result = result + "+"
            CASE "0" TO "9", "A" TO "Z", "a" TO "z", "-", "_", ".", "~"
                result = result + ch
            CASE ELSE
                result = result + "%" + RIGHT$("0" + HEX$(ASC(ch)), 2)
        END SELECT
    NEXT
    
    UrlEncode$ = result
END FUNCTION

FUNCTION UrlEncodeComponent$ (str AS STRING)
    ' Component-level URL encoding
    DIM result AS STRING
    DIM i AS INTEGER
    DIM ch AS STRING * 1
    
    result = ""
    
    FOR i = 1 TO LEN(str)
        ch = MID$(str, i, 1)
        SELECT CASE ch
            CASE "0" TO "9", "A" TO "Z", "a" TO "z", "-", "_", ".", "!", "~", "*", "'", "(", ")"
                result = result + ch
            CASE ELSE
                result = result + "%" + RIGHT$("0" + HEX$(ASC(ch)), 2)
        END SELECT
    NEXT
    
    UrlEncodeComponent$ = result
END FUNCTION

FUNCTION UrlDecode$ (str AS STRING)
    DIM result AS STRING
    DIM i AS INTEGER
    DIM ch AS STRING * 1
    DIM hexCode AS STRING
    
    result = ""
    i = 1
    
    DO WHILE i <= LEN(str)
        ch = MID$(str, i, 1)
        
        IF ch = "+" THEN
            result = result + " "
            i = i + 1
        ELSEIF ch = "%" AND i + 2 <= LEN(str) THEN
            hexCode = MID$(str, i + 1, 2)
            result = result + CHR$(VAL("&H" + hexCode))
            i = i + 3
        ELSE
            result = result + ch
            i = i + 1
        END IF
    LOOP
    
    UrlDecode$ = result
END FUNCTION

'-------------------------------------------------------------------------------
' QUERY PARAMETER BUILDERS
'-------------------------------------------------------------------------------

FUNCTION UrlBuildQueryString$ (parts AS UrlParts)
    DIM result AS STRING
    DIM i AS INTEGER
    
    result = ""
    
    FOR i = 1 TO parts.queryCount
        IF i > 1 THEN result = result + "&"
        result = result + UrlEncode$(RTRIM$(parts.queryKeys(i)))
        result = result + "="
        result = result + UrlEncode$(RTRIM$(parts.queryValues(i)))
    NEXT
    
    IF result <> "" THEN
        result = "?" + result
    END IF
    
    UrlBuildQueryString$ = result
END FUNCTION

SUB UrlSetQueryParam (parts AS UrlParts, key AS STRING, value AS STRING)
    DIM i AS INTEGER
    DIM found AS _BYTE
    
    found = 0
    FOR i = 1 TO parts.queryCount
        IF RTRIM$(parts.queryKeys(i)) = key THEN
            parts.queryValues(i) = value
            found = -1
            EXIT FOR
        END IF
    NEXT
    
    IF NOT found AND parts.queryCount < UBOUND(parts.queryKeys) THEN
        parts.queryCount = parts.queryCount + 1
        parts.queryKeys(parts.queryCount) = key
        parts.queryValues(parts.queryCount) = value
    END IF
    
    'Update search property
    parts.search = UrlBuildQueryString(parts)
END SUB

FUNCTION UrlGetQueryParam$ (parts AS UrlParts, key AS STRING)
    DIM i AS INTEGER
    
    FOR i = 1 TO parts.queryCount
        IF RTRIM$(parts.queryKeys(i)) = key THEN
            UrlGetQueryParam$ = RTRIM$(parts.queryValues(i))
            EXIT FUNCTION
        END IF
    NEXT
    
    UrlGetQueryParam$ = ""
END FUNCTION

'-------------------------------------------------------------------------------
' URL BUILDING
'-------------------------------------------------------------------------------

FUNCTION UrlFormat$ (parts AS UrlParts)
    DIM result AS STRING
    
    result = ""
    
    'Protocol
    IF parts.protocol <> "" THEN
        result = RTRIM$(parts.protocol) + "//"
    END IF
    
    'Auth
    IF parts.username <> "" THEN
        result = result + RTRIM$(parts.username)
        IF parts.password <> "" THEN
            result = result + ":" + RTRIM$(parts.password)
        END IF
        result = result + "@"
    END IF
    
    'Host
    result = result + RTRIM$(parts.host)
    
    'Path
    IF parts.pathname <> "" THEN
        result = result + RTRIM$(parts.pathname)
    ELSE
        result = result + "/"
    END IF
    
    'Query string
    IF parts.search <> "" THEN
        result = result + RTRIM$(parts.search)
    END IF
    
    'Hash
    IF parts.hash <> "" THEN
        result = result + RTRIM$(parts.hash)
    END IF
    
    UrlFormat$ = result
END FUNCTION

SUB UrlResolve (baseUrl AS STRING, relativeUrl AS STRING, result AS UrlParts)
    DIM baseParts AS UrlParts
    
    UrlParse baseUrl, baseParts
    
    IF INSTR(relativeUrl, "://") > 0 THEN
        'Absolute URL
        UrlParse relativeUrl, result
    ELSEIF LEFT$(relativeUrl, 2) = "//" THEN
        'Protocol-relative
        result = baseParts
        result.protocol = ""
        pos = INSTR(relativeUrl, "/", 3)
        IF pos > 0 THEN
            result.host = MID$(relativeUrl, 3, pos - 3)
            result.pathname = MID$(relativeUrl, pos)
        ELSE
            result.host = MID$(relativeUrl, 3)
            result.pathname = "/"
        END IF
    ELSEIF LEFT$(relativeUrl, 1) = "/" THEN
        'Root-relative
        result = baseParts
        result.pathname = relativeUrl
    ELSE
        'Path-relative
        result = baseParts
        
        'Resolve relative path
        DIM basePath AS STRING
        basePath = RTRIM$(baseParts.pathname)
        
        'Remove filename from base path
        DIM lastSlash AS INTEGER
        lastSlash = _INSTRREV(basePath, "/")
        IF lastSlash > 0 THEN
            basePath = LEFT$(basePath, lastSlash)
        END IF
        
        result.pathname = basePath + relativeUrl
    END IF
    
    'Re-parse to update derived properties
    UrlParse UrlFormat$(result), result
END SUB

'-------------------------------------------------------------------------------
' PATH MANIPULATION
'-------------------------------------------------------------------------------

FUNCTION UrlJoin$ (basePath AS STRING, path AS STRING)
    DIM result AS STRING
    
    'Ensure base ends with /
    IF RIGHT$(basePath, 1) <> "/" THEN
        result = basePath + "/"
    ELSE
        result = basePath
    END IF
    
    'Remove leading / from path
    IF LEFT$(path, 1) = "/" THEN
        result = result + MID$(path, 2)
    ELSE
        result = result + path
    END IF
    
    UrlJoin$ = result
END FUNCTION

FUNCTION UrlDirname$ (path AS STRING)
    DIM lastSlash AS INTEGER
    
    lastSlash = _INSTRREV(path, "/")
    
    IF lastSlash > 1 THEN
        UrlDirname$ = LEFT$(path, lastSlash - 1)
    ELSEIF lastSlash = 1 THEN
        UrlDirname$ = "/"
    ELSE
        UrlDirname$ = "."
    END IF
END FUNCTION

FUNCTION UrlBasename$ (path AS STRING, ext AS STRING)
    DIM lastSlash AS INTEGER
    DIM result AS STRING
    
    lastSlash = _INSTRREV(path, "/")
    
    IF lastSlash > 0 THEN
        result = MID$(path, lastSlash + 1)
    ELSE
        result = path
    END IF
    
    'Remove extension if specified
    IF ext <> "" THEN
        IF RIGHT$(result, LEN(ext)) = ext THEN
            result = LEFT$(result, LEN(result) - LEN(ext))
        END IF
    END IF
    
    UrlBasename$ = result
END FUNCTION

FUNCTION UrlExtname$ (path AS STRING)
    DIM lastDot AS INTEGER
    DIM lastSlash AS INTEGER
    
    lastDot = _INSTRREV(path, ".")
    lastSlash = _INSTRREV(path, "/")
    
    IF lastDot > lastSlash THEN
        UrlExtname$ = MID$(path, lastDot)
    ELSE
        UrlExtname$ = ""
    END IF
END FUNCTION

'-------------------------------------------------------------------------------
' FILE URL HELPERS
'-------------------------------------------------------------------------------

FUNCTION UrlFileToPath$ (fileUrl AS STRING)
    DIM result AS STRING
    
    IF LEFT$(fileUrl, 8) = "file:///" THEN
        result = MID$(fileUrl, 8)
        result = UrlDecode$(result)
    ELSE
        result = fileUrl
    END IF
    
    UrlFileToPath$ = result
END FUNCTION

FUNCTION UrlPathToFile$ (filePath AS STRING)
    UrlPathToFile$ = "file:///" + UrlEncodeComponent$(filePath)
END FUNCTION

'-------------------------------------------------------------------------------
' UTILITY FUNCTIONS
'-------------------------------------------------------------------------------

FUNCTION _INSTRREV (s AS STRING, substr AS STRING)
    DIM i AS INTEGER
    
    FOR i = LEN(s) TO 1 STEP -1
        IF MID$(s, i, LEN(substr)) = substr THEN
            _INSTRREV = i
            EXIT FUNCTION
        END IF
    NEXT
    
    _INSTRREV = 0
END FUNCTION

' Check for absolute URL format
FUNCTION UrlIsAbsolute% (url AS STRING)
    UrlIsAbsolute% = (INSTR(url, "://") > 0)
END FUNCTION

' Normalize path (resolve .. and .)
FUNCTION UrlNormalizePath$ (path AS STRING)
    DIM parts(1 TO 50) AS STRING * 128
    DIM partCount AS INTEGER
    DIM result AS STRING
    DIM temp AS STRING
    DIM i AS INTEGER
    
    'Split path
    temp = path
    partCount = 0
    
    DO
        DIM slashPos AS INTEGER
        slashPos = INSTR(temp, "/")
        
        IF slashPos > 0 THEN
            IF slashPos > 1 OR partCount = 0 THEN
                partCount = partCount + 1
                parts(partCount) = LEFT$(temp, slashPos - 1)
            END IF
            temp = MID$(temp, slashPos + 1)
        ELSE
            IF temp <> "" THEN
                partCount = partCount + 1
                parts(partCount) = temp
            END IF
            EXIT DO
        END IF
    LOOP
    
    'Process . and ..
    DIM outputParts(1 TO 50) AS STRING * 128
    DIM outputCount AS INTEGER
    outputCount = 0
    
    FOR i = 1 TO partCount
        IF parts(i) = "." THEN
            'Skip
        ELSEIF parts(i) = ".." THEN
            IF outputCount > 0 THEN
                outputCount = outputCount - 1
            END IF
        ELSE
            outputCount = outputCount + 1
            outputParts(outputCount) = parts(i)
        END IF
    NEXT
    
    'Rebuild path
    result = ""
    FOR i = 1 TO outputCount
        result = result + "/" + RTRIM$(outputParts(i))
    NEXT
    
    IF result = "" THEN result = "/"
    
    UrlNormalizePath$ = result
END FUNCTION

