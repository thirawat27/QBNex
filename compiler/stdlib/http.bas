'===============================================================================
' QBNex Standard Library - http ('$IMPORT:'http')
'===============================================================================
' HTTP and HTTPS client, like Python's requests. TLS is rustls with the
' Mozilla root certificates; redirects are followed. A call returns the body;
' Http_Status is the status (200, 404, ...) or -1 when no answer came back
' (bad URL, refused, DNS, TLS, timeout: see LastError). timeoutMs 0 means 30
' seconds for the whole request. Not available in the web build.
'
'   page$ = Http_Get("https://example.com/")
'   IF Http_Status <> 200 THEN PRINT "failed:"; Http_Status; LastError
'   PRINT Http_Header("content-type")
'   reply$ = Http_Post(url$, json$)                     sent as application/json
'   reply$ = Http_Request("PUT", url$, body$, "Authorization: Bearer x")
'===============================================================================

'$IMPORT:'ext'

DECLARE LIBRARY
    FUNCTION qb_ext_http& (BYVAL method AS _OFFSET, BYVAL ml AS _INTEGER64, BYVAL url AS _OFFSET, BYVAL ul AS _INTEGER64, BYVAL headers AS _OFFSET, BYVAL hl AS _INTEGER64, BYVAL body AS _OFFSET, BYVAL bl AS _INTEGER64, BYVAL timeoutMs AS LONG)
    FUNCTION qb_ext_http_headers&& ()
END DECLARE

DIM SHARED Http_LastStatus AS LONG
DIM SHARED Http_LastHeaders AS STRING

' Sends a request and returns the body of the answer. headers: "Name: value"
' lines separated by CHR$(10) or CR LF.
FUNCTION Http_Request$ (method AS STRING, url AS STRING, OPTIONAL body AS STRING = "", OPTIONAL headers AS STRING = "", OPTIONAL timeoutMs AS LONG = 0)
    DIM st AS LONG, answer AS STRING
    st = qb_ext_http&(_OFFSET(method), LEN(method), _OFFSET(url), LEN(url), _OFFSET(headers), LEN(headers), _OFFSET(body), LEN(body), timeoutMs)
    Http_LastHeaders = ""
    IF st >= 0 THEN
        answer = Ext_Result$
        IF qb_ext_http_headers&& > 0 THEN Http_LastHeaders = Ext_Result$
    END IF
    Ext_Sync
    Http_LastStatus = st
    Http_Request$ = answer
END FUNCTION

' GET url; returns the body.
FUNCTION Http_Get$ (url AS STRING, OPTIONAL timeoutMs AS LONG = 0)
    Http_Get$ = Http_Request$("GET", url, "", "", timeoutMs)
END FUNCTION

' POST body to url; returns the body of the answer.
FUNCTION Http_Post$ (url AS STRING, body AS STRING, OPTIONAL contentType AS STRING = "application/json", OPTIONAL timeoutMs AS LONG = 0)
    DIM h AS STRING
    IF LEN(contentType) THEN h = "Content-Type: " + contentType
    Http_Post$ = Http_Request$("POST", url, body, h, timeoutMs)
END FUNCTION

' Status of the last answer (200, 404, ...); -1 when none came back.
FUNCTION Http_Status& ()
    Http_Status& = Http_LastStatus
END FUNCTION

' Every header of the last answer: "name: value" lines (CR LF), names in lower case.
FUNCTION Http_Headers$ ()
    Http_Headers$ = Http_LastHeaders
END FUNCTION

' One header of the last answer (any letter case); "" when it is not there.
FUNCTION Http_Header$ (headerName AS STRING)
    DIM rest AS STRING, one AS STRING, p AS LONG, c AS LONG, want AS STRING
    want = LCASE$(headerName)
    rest = Http_LastHeaders
    DO WHILE LEN(rest)
        p = INSTR(rest, CHR$(13) + CHR$(10))
        IF p THEN one = LEFT$(rest, p - 1): rest = MID$(rest, p + 2) ELSE one = rest: rest = ""
        c = INSTR(one, ":")
        IF c THEN
            IF LCASE$(LEFT$(one, c - 1)) = want THEN Http_Header$ = LTRIM$(MID$(one, c + 1)): EXIT FUNCTION
        END IF
    LOOP
END FUNCTION
