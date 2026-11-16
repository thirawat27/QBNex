'===============================================================================
' QBNex Standard Library - url ('$IMPORT:'url')
'===============================================================================
' URLs, like JavaScript's URL object and Python's urllib.parse (RFC 3986).
'
'   DIM u AS Url
'   u.Parse "https://user@Example.com:8080/a/b.txt?q=1&s=a+b#top"
'   PRINT u.hostname, u.port, u.pathname    example.com  8080  /a/b.txt
'   PRINT u.Query("s")                     a b
'   u.SetQuery "q", "two words": PRINT u.Format
'   PRINT Url_Encode("a b/c"), Url_Decode("a%20b")
'   PRINT Url_Resolve("http://a/b/c/d;p?q", "../g")   http://a/b/g
'
' The fields are variable-length strings (port is a LONG). A port that is
' not a number from 0 to 65535 is an error: port stays 0 and LastError
' says why. Query parameters are kept encoded, one "key=value" per line in
' queryData; Url_Query decodes them ("+" is a space).
'===============================================================================

'$IMPORT:'core'

TYPE Url
    href AS STRING
    protocol AS STRING
    username AS STRING
    password AS STRING
    hostname AS STRING
    port AS LONG
    pathname AS STRING
    search AS STRING
    hash AS STRING
    host AS STRING
    origin AS STRING
    queryCount AS INTEGER
    queryData AS STRING
END TYPE
' The port of a URL: digits only, 0 to 65535 (an empty port means none, as in
' "http://h:/"). Returns -1 when portText is not a valid port.
FUNCTION UrlPortValue& (portText AS STRING)
    DIM i&, c%, v&
    IF LEN(portText) > 5 THEN UrlPortValue& = -1: EXIT FUNCTION
    FOR i& = 1 TO LEN(portText)
        c% = ASC(portText, i&)
        IF c% < 48 OR c% > 57 THEN UrlPortValue& = -1: EXIT FUNCTION
        v& = v& * 10 + c% - 48
    NEXT
    IF v& > 65535 THEN v& = -1
    UrlPortValue& = v&
END FUNCTION

' Fills u from the text of a URL.
SUB Url_Parse (urlParseParts AS Url, urlParseText AS STRING)
    DIM rest AS STRING, auth AS STRING, hostPort AS STRING
    DIM p&, c&, portValue&

    Core_SetError ""
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
            portValue& = UrlPortValue&(MID$(hostPort, c& + 1))
            IF portValue& < 0 THEN
                Core_SetError "invalid port: " + MID$(hostPort, c& + 1)
            ELSE
                urlParseParts.port = portValue&
            END IF
        ELSE
            urlParseParts.hostname = LCASE$(hostPort)
        END IF
        IF rest = "" THEN rest = "/"
    END IF
    urlParseParts.pathname = rest

    IF urlParseParts.port > 0 THEN
        urlParseParts.host = urlParseParts.hostname + ":" + LTRIM$(STR$(urlParseParts.port))
    ELSE
        urlParseParts.host = urlParseParts.hostname
    END IF
    IF LEN(urlParseParts.protocol) > 0 AND LEN(urlParseParts.host) > 0 THEN
        urlParseParts.origin = urlParseParts.protocol + "//" + urlParseParts.host
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

SUB UrlParseQueryString (urlQueryText AS STRING, urlQueryParts AS Url)
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

' Percent-encodes everything except the RFC 3986 unreserved characters
' (JS encodeURIComponent, Python quote with safe="").
FUNCTION Url_Encode$ (urlEncodeComponentText AS STRING)
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
    Url_Encode$ = r$
END FUNCTION

' Decodes %XX escapes; a "%" not followed by two hex digits is kept as is.
FUNCTION Url_Decode$ (urlDecodeText AS STRING)
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
    Url_Decode$ = r$
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
    UrlDecodeQuery$ = Url_Decode$(r$)
END FUNCTION

' Sets query parameter urlSetKey (replaces it, or adds it at the end).
SUB Url_SetQuery (urlSetParts AS Url, urlSetKey AS STRING, urlSetValue AS STRING)
    DIM lines$, item$, qd$, encoded$
    DIM p&, eq&, found%, n%
    encoded$ = Url_Encode$(urlSetKey) + "=" + Url_Encode$(urlSetValue)
    lines$ = urlSetParts.queryData
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
    urlSetParts.search = "?" + UrlLines$(qd$, "&")
    urlSetParts.href = Url_Format$(urlSetParts)
END SUB

' The decoded value of query parameter urlGetKey ("" when it is not there).
FUNCTION Url_Query$ (urlGetParts AS Url, urlGetKey AS STRING)
    DIM lines$, item$
    DIM p&, eq&
    lines$ = urlGetParts.queryData
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
            Url_Query$ = UrlDecodeQuery$(MID$(item$, eq& + 1))
            EXIT FUNCTION
        END IF
    LOOP
END FUNCTION

' The URL text of u's fields.
FUNCTION Url_Format$ (urlFormatParts AS Url)
    DIM r$
    IF LEN(urlFormatParts.protocol) > 0 THEN r$ = urlFormatParts.protocol
    IF LEN(urlFormatParts.hostname) > 0 THEN
        r$ = r$ + "//"
        IF LEN(urlFormatParts.username) > 0 THEN
            r$ = r$ + urlFormatParts.username
            IF LEN(urlFormatParts.password) > 0 THEN r$ = r$ + ":" + urlFormatParts.password
            r$ = r$ + "@"
        END IF
        r$ = r$ + urlFormatParts.hostname
        IF urlFormatParts.port > 0 THEN r$ = r$ + ":" + LTRIM$(STR$(urlFormatParts.port))
    END IF
    Url_Format$ = r$ + urlFormatParts.pathname + urlFormatParts.search + urlFormatParts.hash
END FUNCTION

' A reference resolved against a base URL (RFC 3986 section 5.2; Python urljoin).
FUNCTION Url_Resolve$ (urlResolveBase AS STRING, urlResolveRelative AS STRING)
    DIM b AS Url, res AS Url
    DIM r$, path$, query$, frag$, p&, basePath$, authority$
    r$ = urlResolveRelative
    IF UrlSchemeEnd&(r$) > 0 THEN
        Url_Parse res, r$
        res.pathname = UrlNormalizePath$(res.pathname)
        Url_Resolve$ = Url_Format$(res)
        EXIT FUNCTION
    END IF
    Url_Parse b, urlResolveBase
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
    IF LEN(b.hostname) > 0 THEN
        authority$ = "//"
        IF LEN(b.username) > 0 THEN
            authority$ = authority$ + b.username
            IF LEN(b.password) > 0 THEN authority$ = authority$ + ":" + b.password
            authority$ = authority$ + "@"
        END IF
        authority$ = authority$ + b.host
    END IF
    IF LEFT$(r$, 2) = "//" THEN
        Url_Parse res, b.protocol + urlResolveRelative
        res.pathname = UrlNormalizePath$(res.pathname)
        Url_Resolve$ = Url_Format$(res)
        EXIT FUNCTION
    END IF
    basePath$ = b.pathname
    IF r$ = "" THEN
        path$ = basePath$
        IF query$ = "" THEN query$ = b.search
    ELSEIF LEFT$(r$, 1) = "/" THEN
        path$ = UrlNormalizePath$(r$)
    ELSEIF LEN(authority$) > 0 AND basePath$ = "" THEN
        path$ = UrlNormalizePath$("/" + r$)
    ELSE
        p& = UrlInstrRev%(basePath$, "/")
        path$ = UrlNormalizePath$(LEFT$(basePath$, p&) + r$)
    END IF
    Url_Resolve$ = b.protocol + authority$ + path$ + query$ + frag$
END FUNCTION

' The file path of a file:/// URL (other text is returned as it is).
FUNCTION Url_ToPath$ (urlFileUrl AS STRING)
    DIM t$
    t$ = urlFileUrl
    IF LEFT$(LCASE$(t$), 8) = "file:///" THEN
        t$ = MID$(t$, 9)
        ' file:///C:/x -> C:/x, file:///usr/x -> /usr/x
        IF NOT (LEN(t$) >= 2 AND MID$(t$, 2, 1) = ":") THEN t$ = "/" + t$
        Url_ToPath$ = Url_Decode$(t$)
    ELSE
        Url_ToPath$ = urlFileUrl
    END IF
END FUNCTION

' The file:/// URL of a file path.
FUNCTION Url_FromPath$ (urlFilePath AS STRING)
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
    Url_FromPath$ = "file:///" + r$
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

' Lines of text joined with replaceLineReplacement.
FUNCTION UrlLines$ (replaceLineText AS STRING, replaceLineReplacement AS STRING)
    DIM r$, i&, c$
    FOR i& = 1 TO LEN(replaceLineText)
        c$ = MID$(replaceLineText, i&, 1)
        IF c$ = CHR$(10) THEN
            r$ = r$ + replaceLineReplacement
        ELSEIF c$ <> CHR$(13) THEN
            r$ = r$ + c$
        END IF
    NEXT
    UrlLines$ = r$
END FUNCTION
