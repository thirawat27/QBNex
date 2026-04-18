'===============================================================================
' QBNex Standard Library - URL Compatibility Module
'===============================================================================
' Stage0-compatible URL parser/manipulator. Preserves the public API with a
' simplified query-string representation during bootstrap/self-host builds.
'===============================================================================

TYPE UrlParts
    href AS STRING * 2048
    protocol AS STRING * 16
    username AS STRING * 128
    password AS STRING * 128
    hostname AS STRING * 256
    port AS INTEGER
    pathname AS STRING * 1024
    search AS STRING * 2048
    hash AS STRING * 512
    host AS STRING * 268
    origin AS STRING * 2048
    queryCount AS INTEGER
    queryData AS STRING * 4096
END TYPE

SUB Url_Init
END SUB

SUB UrlParse (urlString AS STRING, parts AS UrlParts)
    DIM pos%
    DIM temp AS STRING
    DIM auth AS STRING
    DIM colonPos%

    parts.href = urlString
    parts.protocol = ""
    parts.username = ""
    parts.password = ""
    parts.hostname = ""
    parts.port = 0
    parts.pathname = "/"
    parts.search = ""
    parts.hash = ""
    parts.host = ""
    parts.origin = ""
    parts.queryCount = 0
    parts.queryData = ""

    pos% = INSTR(urlString, "://")
    IF pos% > 0 THEN
        parts.protocol = LEFT$(urlString, pos% + 2)
        temp = MID$(urlString, pos% + 3)
    ELSE
        temp = urlString
    END IF

    pos% = INSTR(temp, "#")
    IF pos% > 0 THEN
        parts.hash = MID$(temp, pos%)
        temp = LEFT$(temp, pos% - 1)
    END IF

    pos% = INSTR(temp, "?")
    IF pos% > 0 THEN
        parts.search = MID$(temp, pos%)
        UrlParseQueryString MID$(temp, pos% + 1), parts
        temp = LEFT$(temp, pos% - 1)
    END IF

    pos% = INSTR(temp, "@")
    IF pos% > 0 THEN
        auth = LEFT$(temp, pos% - 1)
        temp = MID$(temp, pos% + 1)
        colonPos% = INSTR(auth, ":")
        IF colonPos% > 0 THEN
            parts.username = LEFT$(auth, colonPos% - 1)
            parts.password = MID$(auth, colonPos% + 1)
        ELSE
            parts.username = auth
        END IF
    END IF

    pos% = INSTR(temp, "/")
    IF pos% > 0 THEN
        parts.pathname = MID$(temp, pos%)
        temp = LEFT$(temp, pos% - 1)
    END IF

    colonPos% = INSTR(temp, ":")
    IF colonPos% > 0 THEN
        parts.hostname = LEFT$(temp, colonPos% - 1)
        parts.port = VAL(MID$(temp, colonPos% + 1))
    ELSE
        parts.hostname = temp
    END IF

    IF parts.port > 0 THEN
        parts.host = parts.hostname + ":" + LTRIM$(STR$(parts.port))
    ELSE
        parts.host = parts.hostname
    END IF

    IF LEN(parts.protocol) > 0 THEN
        parts.origin = parts.protocol + "//" + parts.host
    END IF
END SUB

SUB UrlParseQueryString (queryString AS STRING, parts AS UrlParts)
    DIM item AS STRING
    DIM rest AS STRING
    DIM pos%

    parts.queryData = ""
    parts.queryCount = 0
    rest = queryString
    DO WHILE LEN(rest) > 0
        pos% = INSTR(rest, "&")
        IF pos% > 0 THEN
            item = LEFT$(rest, pos% - 1)
            rest = MID$(rest, pos% + 1)
        ELSE
            item = rest
            rest = ""
        END IF
        IF LEN(item) > 0 THEN
            IF LEN(parts.queryData) > 0 THEN parts.queryData = parts.queryData + CHR$(10)
            parts.queryData = parts.queryData + item
            parts.queryCount = parts.queryCount + 1
        END IF
    LOOP
END SUB

FUNCTION UrlEncode$ (str AS STRING)
    UrlEncode$ = UrlEncodeComponent$(str)
END FUNCTION

FUNCTION UrlEncodeComponent$ (str AS STRING)
    DIM i%
    DIM c$
    DIM result$

    result$ = ""
    FOR i% = 1 TO LEN(str)
        c$ = MID$(str, i%, 1)
        IF (c$ >= "A" AND c$ <= "Z") OR (c$ >= "a" AND c$ <= "z") OR (c$ >= "0" AND c$ <= "9") OR c$ = "-" OR c$ = "_" OR c$ = "." OR c$ = "~" THEN
            result$ = result$ + c$
        ELSEIF c$ = " " THEN
            result$ = result$ + "%20"
        ELSE
            result$ = result$ + "%" + RIGHT$("0" + HEX$(ASC(c$)), 2)
        END IF
    NEXT
    UrlEncodeComponent$ = result$
END FUNCTION

FUNCTION UrlDecode$ (str AS STRING)
    DIM i%
    DIM result$
    DIM hexPart$

    result$ = ""
    i% = 1
    DO WHILE i% <= LEN(str)
        IF MID$(str, i%, 1) = "%" AND i% + 2 <= LEN(str) THEN
            hexPart$ = MID$(str, i% + 1, 2)
            result$ = result$ + CHR$(VAL("&H" + hexPart$))
            i% = i% + 3
        ELSE
            result$ = result$ + MID$(str, i%, 1)
            i% = i% + 1
        END IF
    LOOP
    UrlDecode$ = result$
END FUNCTION

FUNCTION UrlBuildQueryString$ (parts AS UrlParts)
    UrlBuildQueryString$ = parts.queryData
    IF LEN(UrlBuildQueryString$) > 0 THEN UrlBuildQueryString$ = REPLACE_LINEBREAKS$(UrlBuildQueryString$, "&")
END FUNCTION

SUB UrlSetQueryParam (parts AS UrlParts, key AS STRING, value AS STRING)
    DIM encoded$

    encoded$ = UrlEncodeComponent$(key) + "=" + UrlEncodeComponent$(value)
    IF LEN(parts.queryData) > 0 THEN parts.queryData = parts.queryData + CHR$(10)
    parts.queryData = parts.queryData + encoded$
    parts.queryCount = parts.queryCount + 1
    parts.search = "?" + REPLACE_LINEBREAKS$(parts.queryData, "&")
END SUB

FUNCTION UrlGetQueryParam$ (parts AS UrlParts, key AS STRING)
    DIM lines$
    DIM item$
    DIM pos%
    DIM eq%

    lines$ = parts.queryData
    DO WHILE LEN(lines$) > 0
        pos% = INSTR(lines$, CHR$(10))
        IF pos% > 0 THEN
            item$ = LEFT$(lines$, pos% - 1)
            lines$ = MID$(lines$, pos% + 1)
        ELSE
            item$ = lines$
            lines$ = ""
        END IF
        eq% = INSTR(item$, "=")
        IF eq% > 0 THEN
            IF UrlDecode$(LEFT$(item$, eq% - 1)) = key THEN
                UrlGetQueryParam$ = UrlDecode$(MID$(item$, eq% + 1))
                EXIT FUNCTION
            END IF
        END IF
    LOOP
    UrlGetQueryParam$ = ""
END FUNCTION

FUNCTION UrlFormat$ (parts AS UrlParts)
    DIM result$
    result$ = ""
    IF LEN(parts.protocol) > 0 THEN result$ = result$ + parts.protocol + "//"
    IF LEN(parts.username) > 0 THEN
        result$ = result$ + parts.username
        IF LEN(parts.password) > 0 THEN result$ = result$ + ":" + parts.password
        result$ = result$ + "@"
    END IF
    result$ = result$ + parts.hostname
    IF parts.port > 0 THEN result$ = result$ + ":" + LTRIM$(STR$(parts.port))
    result$ = result$ + parts.pathname + parts.search + parts.hash
    UrlFormat$ = result$
END FUNCTION

SUB UrlResolve (baseUrl AS STRING, relativeUrl AS STRING, result AS UrlParts)
    IF UrlIsAbsolute%(relativeUrl) THEN
        UrlParse relativeUrl, result
    ELSE
        UrlParse UrlJoin$(baseUrl, relativeUrl), result
    END IF
END SUB

FUNCTION UrlJoin$ (basePath AS STRING, path AS STRING)
    IF LEN(basePath) = 0 THEN
        UrlJoin$ = path
    ELSEIF RIGHT$(basePath, 1) = "/" THEN
        UrlJoin$ = basePath + path
    ELSE
        UrlJoin$ = basePath + "/" + path
    END IF
END FUNCTION

FUNCTION UrlDirname$ (path AS STRING)
    DIM pos%
    pos% = _INSTRREV(path, "/")
    IF pos% > 0 THEN UrlDirname$ = LEFT$(path, pos% - 1) ELSE UrlDirname$ = ""
END FUNCTION

FUNCTION UrlBasename$ (path AS STRING, ext AS STRING)
    DIM pos%
    DIM name$
    pos% = _INSTRREV(path, "/")
    IF pos% > 0 THEN name$ = MID$(path, pos% + 1) ELSE name$ = path
    IF LEN(ext) > 0 AND RIGHT$(name$, LEN(ext)) = ext THEN name$ = LEFT$(name$, LEN(name$) - LEN(ext))
    UrlBasename$ = name$
END FUNCTION

FUNCTION UrlExtname$ (path AS STRING)
    DIM pos%
    pos% = _INSTRREV(path, ".")
    IF pos% > 0 THEN UrlExtname$ = MID$(path, pos%) ELSE UrlExtname$ = ""
END FUNCTION

FUNCTION UrlFileToPath$ (fileUrl AS STRING)
    IF LEFT$(LCASE$(fileUrl), 8) = "file:///" THEN
        UrlFileToPath$ = MID$(fileUrl, 9)
    ELSE
        UrlFileToPath$ = fileUrl
    END IF
END FUNCTION

FUNCTION UrlPathToFile$ (filePath AS STRING)
    UrlPathToFile$ = "file:///" + filePath
END FUNCTION

FUNCTION _INSTRREV (s AS STRING, substr AS STRING)
    DIM i%
    FOR i% = LEN(s) - LEN(substr) + 1 TO 1 STEP -1
        IF MID$(s, i%, LEN(substr)) = substr THEN
            _INSTRREV = i%
            EXIT FUNCTION
        END IF
    NEXT
    _INSTRREV = 0
END FUNCTION

FUNCTION UrlIsAbsolute% (url AS STRING)
    IF INSTR(url, "://") > 0 OR LEFT$(url, 1) = "/" THEN UrlIsAbsolute% = -1 ELSE UrlIsAbsolute% = 0
END FUNCTION

FUNCTION UrlNormalizePath$ (path AS STRING)
    UrlNormalizePath$ = path
END FUNCTION

FUNCTION REPLACE_LINEBREAKS$ (s AS STRING, replacement AS STRING)
    DIM result$
    DIM i%
    result$ = ""
    FOR i% = 1 TO LEN(s)
        IF MID$(s, i%, 1) = CHR$(10) THEN
            result$ = result$ + replacement
        ELSEIF MID$(s, i%, 1) <> CHR$(13) THEN
            result$ = result$ + MID$(s, i%, 1)
        END IF
    NEXT
    REPLACE_LINEBREAKS$ = result$
END FUNCTION
