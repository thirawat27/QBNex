'===============================================================================
' QBNex Standard Library - URL ('$IMPORT:'url')
'===============================================================================
' Parses, formats and resolves URLs (RFC 3986 reference resolution and dot
' segment removal; field names follow the WHATWG URL object).
'
'   DIM u AS UrlParts
'   UrlParse "https://user:pw@example.com:8080/a/b.txt?q=1#top", u
'   u.protocol = "https:"   u.hostname = "example.com"   u.port = 8080
'   u.pathname = "/a/b.txt" u.search = "?q=1"            u.hash = "#top"
'
' UrlParts fields are fixed-length strings: read them with RTRIM$ (the
' functions here do that themselves). Query parameters are kept encoded, one
' "key=value" per line in queryData; UrlGetQueryParam$ decodes them ("+" is a
' space), UrlSetQueryParam replaces an existing key or appends a new one.
'===============================================================================

TYPE UrlParts
    href AS STRING * 2048
    protocol AS STRING * 16
    username AS STRING * 128
    password AS STRING * 128
    hostname AS STRING * 256
    port AS LONG
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

SUB UrlParse (urlParseText AS STRING, urlParseParts AS UrlParts)
    DIM rest AS STRING, auth AS STRING, hostPort AS STRING
    DIM p&, c&

    urlParseParts.href = urlParseText
    urlParseParts.protocol = ""
    urlParseParts.username = ""
    urlParseParts.password = ""
    urlParseParts.hostname = ""
    urlParseParts.port = 0
    urlParseParts.pathname = ""
    urlParseParts.search = ""
    urlParseParts.hash = ""
    urlParseParts.host = ""
    urlParseParts.origin = ""
    urlParseParts.queryCount = 0
    urlParseParts.queryData = ""

    rest = urlParseText
    p& = INSTR(rest, "#")
    IF p& > 0 THEN
        urlParseParts.hash = MID$(rest, p&)
        rest = LEFT$(rest, p& - 1)
    END IF
    p& = INSTR(rest, "?")
    IF p& > 0 THEN
        urlParseParts.search = MID$(rest, p&)
        UrlParseQueryString MID$(rest, p& + 1), urlParseParts
        rest = LEFT$(rest, p& - 1)
    END IF

    p& = UrlSchemeEnd&(rest)
    IF p& > 0 THEN
        urlParseParts.protocol = LCASE$(LEFT$(rest, p&))
        rest = MID$(rest, p& + 1)
    END IF

    IF LEFT$(rest, 2) = "//" THEN
        rest = MID$(rest, 3)
        p& = INSTR(rest, "/")
        IF p& > 0 THEN
            hostPort = LEFT$(rest, p& - 1)
            rest = MID$(rest, p&)
        ELSE
            hostPort = rest
            rest = ""
        END IF
        p& = UrlInstrRev%(hostPort, "@")
        IF p& > 0 THEN
            auth = LEFT$(hostPort, p& - 1)
            hostPort = MID$(hostPort, p& + 1)
            c& = INSTR(auth, ":")
            IF c& > 0 THEN
                urlParseParts.username = LEFT$(auth, c& - 1)
                urlParseParts.password = MID$(auth, c& + 1)
            ELSE
                urlParseParts.username = auth
            END IF
        END IF
        c& = UrlInstrRev%(hostPort, ":")
        IF c& > 0 AND INSTR(MID$(hostPort, c& + 1), "]") = 0 THEN
            urlParseParts.hostname = LCASE$(LEFT$(hostPort, c& - 1))
            urlParseParts.port = VAL(MID$(hostPort, c& + 1))
        ELSE
            urlParseParts.hostname = LCASE$(hostPort)
        END IF
        IF rest = "" THEN rest = "/"
    END IF
    urlParseParts.pathname = rest

    IF urlParseParts.port > 0 THEN
        urlParseParts.host = RTRIM$(urlParseParts.hostname) + ":" + LTRIM$(STR$(urlParseParts.port))
    ELSE
        urlParseParts.host = RTRIM$(urlParseParts.hostname)
    END IF
    IF LEN(RTRIM$(urlParseParts.protocol)) > 0 AND LEN(RTRIM$(urlParseParts.host)) > 0 THEN
        urlParseParts.origin = RTRIM$(urlParseParts.protocol) + "//" + RTRIM$(urlParseParts.host)
    END IF
END SUB

' Length of "scheme:" at the start of a reference (0 when there is none).
FUNCTION UrlSchemeEnd& (t AS STRING)
    DIM i&, c%
    FOR i& = 1 TO LEN(t)
        c% = ASC(t, i&)
        SELECT CASE c%
            CASE 65 TO 90, 97 TO 122
            CASE 48 TO 57, 43, 45, 46
                IF i& = 1 THEN EXIT FUNCTION
            CASE 58
                IF i& > 1 THEN UrlSchemeEnd& = i&
                EXIT FUNCTION
            CASE ELSE
                EXIT FUNCTION
        END SELECT
    NEXT
END FUNCTION

SUB UrlParseQueryString (urlQueryText AS STRING, urlQueryParts AS UrlParts)
    DIM item AS STRING, rest AS STRING, qd AS STRING
    DIM p&, n%
    rest = urlQueryText
    DO WHILE LEN(rest) > 0
        p& = INSTR(rest, "&")
        IF p& > 0 THEN
            item = LEFT$(rest, p& - 1)
            rest = MID$(rest, p& + 1)
        ELSE
            item = rest
            rest = ""
        END IF
        IF LEN(item) > 0 THEN
            IF n% > 0 THEN qd = qd + CHR$(10)
            qd = qd + item
            n% = n% + 1
        END IF
    LOOP
    urlQueryParts.queryData = qd
    urlQueryParts.queryCount = n%
END SUB

FUNCTION UrlEncode$ (urlEncodeText AS STRING)
    UrlEncode$ = UrlEncodeComponent$(urlEncodeText)
END FUNCTION

' Percent-encodes everything except the RFC 3986 unreserved characters.
FUNCTION UrlEncodeComponent$ (urlEncodeComponentText AS STRING)
    DIM i&, c%, r$
    FOR i& = 1 TO LEN(urlEncodeComponentText)
        c% = ASC(urlEncodeComponentText, i&)
        SELECT CASE c%
            CASE 65 TO 90, 97 TO 122, 48 TO 57, 45, 95, 46, 126
                r$ = r$ + CHR$(c%)
            CASE ELSE
                r$ = r$ + "%" + RIGHT$("0" + HEX$(c%), 2)
        END SELECT
    NEXT
    UrlEncodeComponent$ = r$
END FUNCTION

' Decodes %XX escapes; a "%" not followed by two hex digits is kept as is.
FUNCTION UrlDecode$ (urlDecodeText AS STRING)
    DIM i&, r$, h$
    i& = 1
    DO WHILE i& <= LEN(urlDecodeText)
        h$ = MID$(urlDecodeText, i& + 1, 2)
        IF MID$(urlDecodeText, i&, 1) = "%" AND UrlIsHex2%(h$) THEN
            r$ = r$ + CHR$(VAL("&H" + h$))
            i& = i& + 3
        ELSE
            r$ = r$ + MID$(urlDecodeText, i&, 1)
            i& = i& + 1
        END IF
    LOOP
    UrlDecode$ = r$
END FUNCTION

FUNCTION UrlIsHex2% (h AS STRING)
    DIM i%, c%
    IF LEN(h) <> 2 THEN EXIT FUNCTION
    FOR i% = 1 TO 2
        c% = ASC(UCASE$(h), i%)
        IF NOT ((c% >= 48 AND c% <= 57) OR (c% >= 65 AND c% <= 70)) THEN EXIT FUNCTION
    NEXT
    UrlIsHex2% = -1
END FUNCTION

' Query-string value decoding: "+" is a space, then %XX escapes.
FUNCTION UrlDecodeQuery$ (t AS STRING)
    DIM i&, r$
    r$ = t
    FOR i& = 1 TO LEN(r$)
        IF ASC(r$, i&) = 43 THEN MID$(r$, i&, 1) = " "
    NEXT
    UrlDecodeQuery$ = UrlDecode$(r$)
END FUNCTION

FUNCTION UrlBuildQueryString$ (urlBuildParts AS UrlParts)
    UrlBuildQueryString$ = REPLACE_LINEBREAKS$(RTRIM$(urlBuildParts.queryData), "&")
END FUNCTION

SUB UrlSetQueryParam (urlSetParts AS UrlParts, urlSetKey AS STRING, urlSetValue AS STRING)
    DIM lines$, item$, qd$, encoded$
    DIM p&, eq&, found%, n%
    encoded$ = UrlEncodeComponent$(urlSetKey) + "=" + UrlEncodeComponent$(urlSetValue)
    lines$ = RTRIM$(urlSetParts.queryData)
    DO WHILE LEN(lines$) > 0
        p& = INSTR(lines$, CHR$(10))
        IF p& > 0 THEN
            item$ = LEFT$(lines$, p& - 1)
            lines$ = MID$(lines$, p& + 1)
        ELSE
            item$ = lines$
            lines$ = ""
        END IF
        eq& = INSTR(item$, "=")
        IF eq& = 0 THEN eq& = LEN(item$) + 1
        IF UrlDecodeQuery$(LEFT$(item$, eq& - 1)) = urlSetKey THEN
            IF found% THEN item$ = "" ELSE item$ = encoded$
            found% = -1
        END IF
        IF LEN(item$) THEN
            IF n% > 0 THEN qd$ = qd$ + CHR$(10)
            qd$ = qd$ + item$
            n% = n% + 1
        END IF
    LOOP
    IF NOT found% THEN
        IF n% > 0 THEN qd$ = qd$ + CHR$(10)
        qd$ = qd$ + encoded$
        n% = n% + 1
    END IF
    urlSetParts.queryData = qd$
    urlSetParts.queryCount = n%
    urlSetParts.search = "?" + REPLACE_LINEBREAKS$(qd$, "&")
    urlSetParts.href = UrlFormat$(urlSetParts)
END SUB

FUNCTION UrlGetQueryParam$ (urlGetParts AS UrlParts, urlGetKey AS STRING)
    DIM lines$, item$
    DIM p&, eq&
    lines$ = RTRIM$(urlGetParts.queryData)
    DO WHILE LEN(lines$) > 0
        p& = INSTR(lines$, CHR$(10))
        IF p& > 0 THEN
            item$ = LEFT$(lines$, p& - 1)
            lines$ = MID$(lines$, p& + 1)
        ELSE
            item$ = lines$
            lines$ = ""
        END IF
        eq& = INSTR(item$, "=")
        IF eq& = 0 THEN eq& = LEN(item$) + 1
        IF UrlDecodeQuery$(LEFT$(item$, eq& - 1)) = urlGetKey THEN
            UrlGetQueryParam$ = UrlDecodeQuery$(MID$(item$, eq& + 1))
            EXIT FUNCTION
        END IF
    LOOP
END FUNCTION

FUNCTION UrlFormat$ (urlFormatParts AS UrlParts)
    DIM r$
    IF LEN(RTRIM$(urlFormatParts.protocol)) > 0 THEN r$ = RTRIM$(urlFormatParts.protocol)
    IF LEN(RTRIM$(urlFormatParts.hostname)) > 0 THEN
        r$ = r$ + "//"
        IF LEN(RTRIM$(urlFormatParts.username)) > 0 THEN
            r$ = r$ + RTRIM$(urlFormatParts.username)
            IF LEN(RTRIM$(urlFormatParts.password)) > 0 THEN r$ = r$ + ":" + RTRIM$(urlFormatParts.password)
            r$ = r$ + "@"
        END IF
        r$ = r$ + RTRIM$(urlFormatParts.hostname)
        IF urlFormatParts.port > 0 THEN r$ = r$ + ":" + LTRIM$(STR$(urlFormatParts.port))
    END IF
    UrlFormat$ = r$ + RTRIM$(urlFormatParts.pathname) + RTRIM$(urlFormatParts.search) + RTRIM$(urlFormatParts.hash)
END FUNCTION

' RFC 3986 section 5.2: resolves a reference against a base URL.
SUB UrlResolve (urlResolveBase AS STRING, urlResolveRelative AS STRING, urlResolveResult AS UrlParts)
    DIM b AS UrlParts
    DIM r$, path$, query$, frag$, p&, basePath$, authority$
    r$ = urlResolveRelative
    IF UrlSchemeEnd&(r$) > 0 THEN
        UrlParse r$, urlResolveResult
        urlResolveResult.pathname = UrlNormalizePath$(RTRIM$(urlResolveResult.pathname))
        urlResolveResult.href = UrlFormat$(urlResolveResult)
        EXIT SUB
    END IF
    UrlParse urlResolveBase, b
    p& = INSTR(r$, "#")
    IF p& > 0 THEN
        frag$ = MID$(r$, p&)
        r$ = LEFT$(r$, p& - 1)
    END IF
    p& = INSTR(r$, "?")
    IF p& > 0 THEN
        query$ = MID$(r$, p&)
        r$ = LEFT$(r$, p& - 1)
    END IF
    authority$ = ""
    IF LEN(RTRIM$(b.hostname)) > 0 THEN
        authority$ = "//"
        IF LEN(RTRIM$(b.username)) > 0 THEN
            authority$ = authority$ + RTRIM$(b.username)
            IF LEN(RTRIM$(b.password)) > 0 THEN authority$ = authority$ + ":" + RTRIM$(b.password)
            authority$ = authority$ + "@"
        END IF
        authority$ = authority$ + RTRIM$(b.host)
    END IF
    IF LEFT$(r$, 2) = "//" THEN
        UrlParse RTRIM$(b.protocol) + urlResolveRelative, urlResolveResult
        urlResolveResult.pathname = UrlNormalizePath$(RTRIM$(urlResolveResult.pathname))
        urlResolveResult.href = UrlFormat$(urlResolveResult)
        EXIT SUB
    END IF
    basePath$ = RTRIM$(b.pathname)
    IF r$ = "" THEN
        path$ = basePath$
        IF query$ = "" THEN query$ = RTRIM$(b.search)
    ELSEIF LEFT$(r$, 1) = "/" THEN
        path$ = UrlNormalizePath$(r$)
    ELSE
        IF LEN(authority$) > 0 AND basePath$ = "" THEN
            path$ = "/" + r$
        ELSE
            p& = UrlInstrRev%(basePath$, "/")
            path$ = LEFT$(basePath$, p&) + r$
        END IF
        path$ = UrlNormalizePath$(path$)
    END IF
    UrlParse RTRIM$(b.protocol) + authority$ + path$ + query$ + frag$, urlResolveResult
END SUB

FUNCTION UrlJoin$ (urlJoinBase AS STRING, urlJoinPath AS STRING)
    IF LEN(urlJoinBase) = 0 THEN
        UrlJoin$ = urlJoinPath
    ELSEIF RIGHT$(urlJoinBase, 1) = "/" THEN
        UrlJoin$ = urlJoinBase + urlJoinPath
    ELSE
        UrlJoin$ = urlJoinBase + "/" + urlJoinPath
    END IF
END FUNCTION

FUNCTION UrlDirname$ (urlDirPath AS STRING)
    DIM t$, p&
    t$ = RTRIM$(urlDirPath)
    p& = UrlInstrRev%(t$, "/")
    IF p& = 1 THEN
        UrlDirname$ = "/"
    ELSEIF p& > 0 THEN
        UrlDirname$ = LEFT$(t$, p& - 1)
    END IF
END FUNCTION

FUNCTION UrlBasename$ (urlBasePath AS STRING, urlBaseExt AS STRING)
    DIM leaf$, p&
    leaf$ = RTRIM$(urlBasePath)
    p& = UrlInstrRev%(leaf$, "/")
    IF p& > 0 THEN leaf$ = MID$(leaf$, p& + 1)
    IF LEN(urlBaseExt) > 0 AND LEN(leaf$) > LEN(urlBaseExt) THEN
        IF RIGHT$(leaf$, LEN(urlBaseExt)) = urlBaseExt THEN leaf$ = LEFT$(leaf$, LEN(leaf$) - LEN(urlBaseExt))
    END IF
    UrlBasename$ = leaf$
END FUNCTION

FUNCTION UrlExtname$ (urlExtPath AS STRING)
    DIM leaf$, p&
    leaf$ = UrlBasename$(urlExtPath, "")
    p& = UrlInstrRev%(leaf$, ".")
    IF p& > 1 THEN UrlExtname$ = MID$(leaf$, p&)
END FUNCTION

FUNCTION UrlFileToPath$ (urlFileUrl AS STRING)
    DIM t$
    t$ = urlFileUrl
    IF LEFT$(LCASE$(t$), 8) = "file:///" THEN
        t$ = MID$(t$, 9)
        ' file:///C:/x -> C:/x, file:///usr/x -> /usr/x
        IF NOT (LEN(t$) >= 2 AND MID$(t$, 2, 1) = ":") THEN t$ = "/" + t$
        UrlFileToPath$ = UrlDecode$(t$)
    ELSE
        UrlFileToPath$ = urlFileUrl
    END IF
END FUNCTION

FUNCTION UrlPathToFile$ (urlFilePath AS STRING)
    DIM t$, i&, c%, r$
    t$ = urlFilePath
    FOR i& = 1 TO LEN(t$)
        IF ASC(t$, i&) = 92 THEN MID$(t$, i&, 1) = "/"
    NEXT
    IF LEFT$(t$, 1) = "/" THEN t$ = MID$(t$, 2)
    FOR i& = 1 TO LEN(t$)
        c% = ASC(t$, i&)
        SELECT CASE c%
            CASE 32, 35, 37, 63: r$ = r$ + "%" + RIGHT$("0" + HEX$(c%), 2)
            CASE ELSE: r$ = r$ + CHR$(c%)
        END SELECT
    NEXT
    UrlPathToFile$ = "file:///" + r$
END FUNCTION

FUNCTION UrlInstrRev% (urlInstrText AS STRING, urlInstrNeedle AS STRING)
    DIM i&
    IF LEN(urlInstrNeedle) = 0 THEN EXIT FUNCTION
    FOR i& = LEN(urlInstrText) - LEN(urlInstrNeedle) + 1 TO 1 STEP -1
        IF MID$(urlInstrText, i&, LEN(urlInstrNeedle)) = urlInstrNeedle THEN
            UrlInstrRev% = i&
            EXIT FUNCTION
        END IF
    NEXT
END FUNCTION

FUNCTION UrlIsAbsolute% (urlAbsoluteText AS STRING)
    IF UrlSchemeEnd&(urlAbsoluteText) > 0 OR LEFT$(urlAbsoluteText, 1) = "/" THEN UrlIsAbsolute% = -1
END FUNCTION

' RFC 3986 section 5.2.4 (remove_dot_segments).
FUNCTION UrlNormalizePath$ (urlNormalizeValue AS STRING)
    DIM inp$, out$, p&
    inp$ = urlNormalizeValue
    DO WHILE LEN(inp$) > 0
        IF LEFT$(inp$, 3) = "../" THEN
            inp$ = MID$(inp$, 4)
        ELSEIF LEFT$(inp$, 2) = "./" THEN
            inp$ = MID$(inp$, 3)
        ELSEIF LEFT$(inp$, 3) = "/./" THEN
            inp$ = MID$(inp$, 3)
        ELSEIF inp$ = "/." THEN
            inp$ = "/"
        ELSEIF LEFT$(inp$, 4) = "/../" OR inp$ = "/.." THEN
            IF inp$ = "/.." THEN inp$ = "/" ELSE inp$ = MID$(inp$, 4)
            p& = UrlInstrRev%(out$, "/")
            IF p& > 0 THEN out$ = LEFT$(out$, p& - 1) ELSE out$ = ""
        ELSEIF inp$ = "." OR inp$ = ".." THEN
            inp$ = ""
        ELSE
            p& = INSTR(2, inp$, "/")
            IF p& = 0 THEN p& = LEN(inp$) + 1
            out$ = out$ + LEFT$(inp$, p& - 1)
            inp$ = MID$(inp$, p&)
        END IF
    LOOP
    UrlNormalizePath$ = out$
END FUNCTION

FUNCTION REPLACE_LINEBREAKS$ (replaceLineText AS STRING, replaceLineReplacement AS STRING)
    DIM r$, i&, c$
    FOR i& = 1 TO LEN(replaceLineText)
        c$ = MID$(replaceLineText, i&, 1)
        IF c$ = CHR$(10) THEN
            r$ = r$ + replaceLineReplacement
        ELSEIF c$ <> CHR$(13) THEN
            r$ = r$ + c$
        END IF
    NEXT
    REPLACE_LINEBREAKS$ = r$
END FUNCTION
