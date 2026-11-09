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

SUB UrlParse (urlParseText AS STRING, urlParseParts AS UrlParts)
    DIM urlParsePos%
    DIM urlParseTemp AS STRING
    DIM urlParseAuth AS STRING
    DIM urlParseColonPos%

    urlParseParts.href = urlParseText
    urlParseParts.protocol = ""
    urlParseParts.username = ""
    urlParseParts.password = ""
    urlParseParts.hostname = ""
    urlParseParts.port = 0
    urlParseParts.pathname = "/"
    urlParseParts.search = ""
    urlParseParts.hash = ""
    urlParseParts.host = ""
    urlParseParts.origin = ""
    urlParseParts.queryCount = 0
    urlParseParts.queryData = ""

    urlParsePos% = INSTR(urlParseText, "://")
    IF urlParsePos% > 0 THEN
        urlParseParts.protocol = LEFT$(urlParseText, urlParsePos% + 2)
        urlParseTemp = MID$(urlParseText, urlParsePos% + 3)
    ELSE
        urlParseTemp = urlParseText
    END IF

    urlParsePos% = INSTR(urlParseTemp, "#")
    IF urlParsePos% > 0 THEN
        urlParseParts.hash = MID$(urlParseTemp, urlParsePos%)
        urlParseTemp = LEFT$(urlParseTemp, urlParsePos% - 1)
    END IF

    urlParsePos% = INSTR(urlParseTemp, "?")
    IF urlParsePos% > 0 THEN
        urlParseParts.search = MID$(urlParseTemp, urlParsePos%)
        UrlParseQueryString MID$(urlParseTemp, urlParsePos% + 1), urlParseParts
        urlParseTemp = LEFT$(urlParseTemp, urlParsePos% - 1)
    END IF

    urlParsePos% = INSTR(urlParseTemp, "@")
    IF urlParsePos% > 0 THEN
        urlParseAuth = LEFT$(urlParseTemp, urlParsePos% - 1)
        urlParseTemp = MID$(urlParseTemp, urlParsePos% + 1)
        urlParseColonPos% = INSTR(urlParseAuth, ":")
        IF urlParseColonPos% > 0 THEN
            urlParseParts.username = LEFT$(urlParseAuth, urlParseColonPos% - 1)
            urlParseParts.password = MID$(urlParseAuth, urlParseColonPos% + 1)
        ELSE
            urlParseParts.username = urlParseAuth
        END IF
    END IF

    urlParsePos% = INSTR(urlParseTemp, "/")
    IF urlParsePos% > 0 THEN
        urlParseParts.pathname = MID$(urlParseTemp, urlParsePos%)
        urlParseTemp = LEFT$(urlParseTemp, urlParsePos% - 1)
    END IF

    urlParseColonPos% = INSTR(urlParseTemp, ":")
    IF urlParseColonPos% > 0 THEN
        urlParseParts.hostname = LEFT$(urlParseTemp, urlParseColonPos% - 1)
        urlParseParts.port = VAL(MID$(urlParseTemp, urlParseColonPos% + 1))
    ELSE
        urlParseParts.hostname = urlParseTemp
    END IF

    IF urlParseParts.port > 0 THEN
        urlParseParts.host = urlParseParts.hostname + ":" + LTRIM$(STR$(urlParseParts.port))
    ELSE
        urlParseParts.host = urlParseParts.hostname
    END IF

    IF LEN(urlParseParts.protocol) > 0 THEN
        urlParseParts.origin = urlParseParts.protocol + "//" + urlParseParts.host
    END IF
END SUB

SUB UrlParseQueryString (urlQueryText AS STRING, urlQueryParts AS UrlParts)
    DIM urlQueryItem AS STRING
    DIM urlQueryRest AS STRING
    DIM urlQueryPos%

    urlQueryParts.queryData = ""
    urlQueryParts.queryCount = 0
    urlQueryRest = urlQueryText
    DO WHILE LEN(urlQueryRest) > 0
        urlQueryPos% = INSTR(urlQueryRest, "&")
        IF urlQueryPos% > 0 THEN
            urlQueryItem = LEFT$(urlQueryRest, urlQueryPos% - 1)
            urlQueryRest = MID$(urlQueryRest, urlQueryPos% + 1)
        ELSE
            urlQueryItem = urlQueryRest
            urlQueryRest = ""
        END IF
        IF LEN(urlQueryItem) > 0 THEN
            IF LEN(urlQueryParts.queryData) > 0 THEN
                urlQueryParts.queryData = urlQueryParts.queryData + CHR$(10)
            END IF
            urlQueryParts.queryData = urlQueryParts.queryData + urlQueryItem
            urlQueryParts.queryCount = urlQueryParts.queryCount + 1
        END IF
    LOOP
END SUB

FUNCTION UrlEncode$ (urlEncodeText AS STRING)
    UrlEncode$ = UrlEncodeComponent$(urlEncodeText)
END FUNCTION

FUNCTION UrlEncodeComponent$ (urlEncodeComponentText AS STRING)
    DIM urlEncodeIndex%
    DIM urlEncodeChar$
    DIM urlEncodeResult$

    urlEncodeResult$ = ""
    FOR urlEncodeIndex% = 1 TO LEN(urlEncodeComponentText)
        urlEncodeChar$ = MID$(urlEncodeComponentText, urlEncodeIndex%, 1)
        IF (urlEncodeChar$ >= "A" AND urlEncodeChar$ <= "Z") OR (urlEncodeChar$ >= "a" AND urlEncodeChar$ <= "z") OR (urlEncodeChar$ >= "0" AND urlEncodeChar$ <= "9") OR urlEncodeChar$ = "-" OR urlEncodeChar$ = "_" OR urlEncodeChar$ = "." OR urlEncodeChar$ = "~" THEN
            urlEncodeResult$ = urlEncodeResult$ + urlEncodeChar$
        ELSEIF urlEncodeChar$ = " " THEN
            urlEncodeResult$ = urlEncodeResult$ + "%20"
        ELSE
            urlEncodeResult$ = urlEncodeResult$ + "%" + RIGHT$("0" + HEX$(ASC(urlEncodeChar$)), 2)
        END IF
    NEXT
    UrlEncodeComponent$ = urlEncodeResult$
END FUNCTION

FUNCTION UrlDecode$ (urlDecodeText AS STRING)
    DIM urlDecodeIndex%
    DIM urlDecodeResult$
    DIM urlDecodeHex$

    urlDecodeResult$ = ""
    urlDecodeIndex% = 1
    DO WHILE urlDecodeIndex% <= LEN(urlDecodeText)
        IF MID$(urlDecodeText, urlDecodeIndex%, 1) = "%" AND urlDecodeIndex% + 2 <= LEN(urlDecodeText) THEN
            urlDecodeHex$ = MID$(urlDecodeText, urlDecodeIndex% + 1, 2)
            urlDecodeResult$ = urlDecodeResult$ + CHR$(VAL("&H" + urlDecodeHex$))
            urlDecodeIndex% = urlDecodeIndex% + 3
        ELSE
            urlDecodeResult$ = urlDecodeResult$ + MID$(urlDecodeText, urlDecodeIndex%, 1)
            urlDecodeIndex% = urlDecodeIndex% + 1
        END IF
    LOOP
    UrlDecode$ = urlDecodeResult$
END FUNCTION

FUNCTION UrlBuildQueryString$ (urlBuildParts AS UrlParts)
    DIM urlBuildQuery$

    urlBuildQuery$ = urlBuildParts.queryData
    IF LEN(urlBuildQuery$) > 0 THEN
        urlBuildQuery$ = REPLACE_LINEBREAKS$(urlBuildQuery$, "&")
    END IF
    UrlBuildQueryString$ = urlBuildQuery$
END FUNCTION

SUB UrlSetQueryParam (urlSetParts AS UrlParts, urlSetKey AS STRING, urlSetValue AS STRING)
    DIM urlSetEncoded$

    urlSetEncoded$ = UrlEncodeComponent$(urlSetKey) + "=" + UrlEncodeComponent$(urlSetValue)
    IF LEN(urlSetParts.queryData) > 0 THEN
        urlSetParts.queryData = urlSetParts.queryData + CHR$(10)
    END IF
    urlSetParts.queryData = urlSetParts.queryData + urlSetEncoded$
    urlSetParts.queryCount = urlSetParts.queryCount + 1
    urlSetParts.search = "?" + REPLACE_LINEBREAKS$(urlSetParts.queryData, "&")
END SUB

FUNCTION UrlGetQueryParam$ (urlGetParts AS UrlParts, urlGetKey AS STRING)
    DIM urlGetLines$
    DIM urlGetItem$
    DIM urlGetPos%
    DIM urlGetEq%

    urlGetLines$ = urlGetParts.queryData
    DO WHILE LEN(urlGetLines$) > 0
        urlGetPos% = INSTR(urlGetLines$, CHR$(10))
        IF urlGetPos% > 0 THEN
            urlGetItem$ = LEFT$(urlGetLines$, urlGetPos% - 1)
            urlGetLines$ = MID$(urlGetLines$, urlGetPos% + 1)
        ELSE
            urlGetItem$ = urlGetLines$
            urlGetLines$ = ""
        END IF
        urlGetEq% = INSTR(urlGetItem$, "=")
        IF urlGetEq% > 0 THEN
            IF UrlDecode$(LEFT$(urlGetItem$, urlGetEq% - 1)) = urlGetKey THEN
                UrlGetQueryParam$ = UrlDecode$(MID$(urlGetItem$, urlGetEq% + 1))
                EXIT FUNCTION
            END IF
        END IF
    LOOP
    UrlGetQueryParam$ = ""
END FUNCTION

FUNCTION UrlFormat$ (urlFormatParts AS UrlParts)
    DIM urlFormatResult$
    urlFormatResult$ = ""
    IF LEN(urlFormatParts.protocol) > 0 THEN
        urlFormatResult$ = urlFormatResult$ + urlFormatParts.protocol + "//"
    END IF
    IF LEN(urlFormatParts.username) > 0 THEN
        urlFormatResult$ = urlFormatResult$ + urlFormatParts.username
        IF LEN(urlFormatParts.password) > 0 THEN
            urlFormatResult$ = urlFormatResult$ + ":" + urlFormatParts.password
        END IF
        urlFormatResult$ = urlFormatResult$ + "@"
    END IF
    urlFormatResult$ = urlFormatResult$ + urlFormatParts.hostname
    IF urlFormatParts.port > 0 THEN
        urlFormatResult$ = urlFormatResult$ + ":" + LTRIM$(STR$(urlFormatParts.port))
    END IF
    urlFormatResult$ = urlFormatResult$ + urlFormatParts.pathname + urlFormatParts.search + urlFormatParts.hash
    UrlFormat$ = urlFormatResult$
END FUNCTION

SUB UrlResolve (urlResolveBase AS STRING, urlResolveRelative AS STRING, urlResolveResult AS UrlParts)
    IF UrlIsAbsolute%(urlResolveRelative) THEN
        UrlParse urlResolveRelative, urlResolveResult
    ELSE
        UrlParse UrlJoin$(urlResolveBase, urlResolveRelative), urlResolveResult
    END IF
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
    DIM urlDirPos%
    urlDirPos% = UrlInstrRev%(urlDirPath, "/")
    IF urlDirPos% > 0 THEN
        UrlDirname$ = LEFT$(urlDirPath, urlDirPos% - 1)
    ELSE
        UrlDirname$ = ""
    END IF
END FUNCTION

FUNCTION UrlBasename$ (urlBasePath AS STRING, urlBaseExt AS STRING)
    DIM urlBasePos%
    DIM urlBaseLeaf$
    urlBasePos% = UrlInstrRev%(urlBasePath, "/")
    IF urlBasePos% > 0 THEN
        urlBaseLeaf$ = MID$(urlBasePath, urlBasePos% + 1)
    ELSE
        urlBaseLeaf$ = urlBasePath
    END IF
    IF LEN(urlBaseExt) > 0 AND RIGHT$(urlBaseLeaf$, LEN(urlBaseExt)) = urlBaseExt THEN
        urlBaseLeaf$ = LEFT$(urlBaseLeaf$, LEN(urlBaseLeaf$) - LEN(urlBaseExt))
    END IF
    UrlBasename$ = urlBaseLeaf$
END FUNCTION

FUNCTION UrlExtname$ (urlExtPath AS STRING)
    DIM urlExtPos%
    urlExtPos% = UrlInstrRev%(urlExtPath, ".")
    IF urlExtPos% > 0 THEN
        UrlExtname$ = MID$(urlExtPath, urlExtPos%)
    ELSE
        UrlExtname$ = ""
    END IF
END FUNCTION

FUNCTION UrlFileToPath$ (urlFileUrl AS STRING)
    IF LEFT$(LCASE$(urlFileUrl), 8) = "file:///" THEN
        UrlFileToPath$ = MID$(urlFileUrl, 9)
    ELSE
        UrlFileToPath$ = urlFileUrl
    END IF
END FUNCTION

FUNCTION UrlPathToFile$ (urlFilePath AS STRING)
    UrlPathToFile$ = "file:///" + urlFilePath
END FUNCTION

FUNCTION UrlInstrRev% (urlInstrText AS STRING, urlInstrNeedle AS STRING)
    DIM urlInstrIndex%
    FOR urlInstrIndex% = LEN(urlInstrText) - LEN(urlInstrNeedle) + 1 TO 1 STEP -1
        IF MID$(urlInstrText, urlInstrIndex%, LEN(urlInstrNeedle)) = urlInstrNeedle THEN
            UrlInstrRev% = urlInstrIndex%
            EXIT FUNCTION
        END IF
    NEXT
    UrlInstrRev% = 0
END FUNCTION

FUNCTION UrlIsAbsolute% (urlAbsoluteText AS STRING)
    IF INSTR(urlAbsoluteText, "://") > 0 OR LEFT$(urlAbsoluteText, 1) = "/" THEN
        UrlIsAbsolute% = -1
    ELSE
        UrlIsAbsolute% = 0
    END IF
END FUNCTION

FUNCTION UrlNormalizePath$ (urlNormalizeValue AS STRING)
    UrlNormalizePath$ = urlNormalizeValue
END FUNCTION

FUNCTION REPLACE_LINEBREAKS$ (replaceLineText AS STRING, replaceLineReplacement AS STRING)
    DIM replaceLineResult$
    DIM replaceLineIndex%
    replaceLineResult$ = ""
    FOR replaceLineIndex% = 1 TO LEN(replaceLineText)
        IF MID$(replaceLineText, replaceLineIndex%, 1) = CHR$(10) THEN
            replaceLineResult$ = replaceLineResult$ + replaceLineReplacement
        ELSEIF MID$(replaceLineText, replaceLineIndex%, 1) <> CHR$(13) THEN
            replaceLineResult$ = replaceLineResult$ + MID$(replaceLineText, replaceLineIndex%, 1)
        END IF
    NEXT
    REPLACE_LINEBREAKS$ = replaceLineResult$
END FUNCTION
