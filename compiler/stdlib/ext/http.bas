'===============================================================================
' QBNex Standard Library - HTTP/HTTPS client ('$IMPORT:'ext.http')
'===============================================================================
' TLS through rustls with the Mozilla root certificates (no OpenSSL needed).
' Calls return the HTTP status (200, 404, ...) or -1 when no response came
' back (bad URL, refused, DNS, TLS, timeout; see Ext_LastError$). Redirects
' are followed. timeoutMs& <= 0 means 30 seconds for the whole request.
'
'   st& = Http_Get&("https://example.com/", 10000)
'   st& = Http_Post&(url$, "application/json", body$, 10000)
'   st& = Http_Request&("PUT", url$, "Authorization: Bearer x" + CHR$(10), body$, 0)
'   Http_Body$          body of the last response
'   Http_Headers$       "Name: value" lines (CR LF), names in lower case
'   Http_Header$("content-type")   one header (case-insensitive), "" if absent
'===============================================================================

'$IMPORT:'ext.core'

DECLARE LIBRARY
    FUNCTION qb_ext_http& (BYVAL method AS _OFFSET, BYVAL ml AS _INTEGER64, BYVAL url AS _OFFSET, BYVAL ul AS _INTEGER64, BYVAL headers AS _OFFSET, BYVAL hl AS _INTEGER64, BYVAL body AS _OFFSET, BYVAL bl AS _INTEGER64, BYVAL timeoutMs AS LONG)
    FUNCTION qb_ext_http_headers&& ()
END DECLARE

DIM SHARED HttpLastBody$
DIM SHARED HttpLastHeaders$

FUNCTION Http_Request& (method AS STRING, url AS STRING, headers AS STRING, body AS STRING, timeoutMs AS LONG)
    DIM st AS LONG
    st = qb_ext_http&(_OFFSET(method), LEN(method), _OFFSET(url), LEN(url), _OFFSET(headers), LEN(headers), _OFFSET(body), LEN(body), timeoutMs)
    IF st < 0 THEN
        HttpLastBody$ = ""
        HttpLastHeaders$ = ""
    ELSE
        HttpLastBody$ = Ext_Result$
        IF qb_ext_http_headers&& > 0 THEN HttpLastHeaders$ = Ext_Result$ ELSE HttpLastHeaders$ = ""
    END IF
    Http_Request& = st
END FUNCTION

FUNCTION Http_Get& (url AS STRING, timeoutMs AS LONG)
    Http_Get& = Http_Request&("GET", url, "", "", timeoutMs)
END FUNCTION

FUNCTION Http_Post& (url AS STRING, contentType AS STRING, body AS STRING, timeoutMs AS LONG)
    DIM h AS STRING
    IF LEN(contentType) THEN h = "Content-Type: " + contentType
    Http_Post& = Http_Request&("POST", url, h, body, timeoutMs)
END FUNCTION

FUNCTION Http_Body$ ()
    Http_Body$ = HttpLastBody$
END FUNCTION

FUNCTION Http_Headers$ ()
    Http_Headers$ = HttpLastHeaders$
END FUNCTION

FUNCTION Http_Header$ (headerName AS STRING)
    DIM rest AS STRING, lineText AS STRING, p AS LONG, c AS LONG, want AS STRING
    want = LCASE$(headerName)
    rest = HttpLastHeaders$
    DO WHILE LEN(rest)
        p = INSTR(rest, CHR$(13) + CHR$(10))
        IF p THEN
            lineText = LEFT$(rest, p - 1)
            rest = MID$(rest, p + 2)
        ELSE
            lineText = rest
            rest = ""
        END IF
        c = INSTR(lineText, ":")
        IF c THEN
            IF LCASE$(LEFT$(lineText, c - 1)) = want THEN
                Http_Header$ = LTRIM$(MID$(lineText, c + 1))
                EXIT FUNCTION
            END IF
        END IF
    LOOP
END FUNCTION
