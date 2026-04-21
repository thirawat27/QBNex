'$IMPORT:'url'

DIM parts AS UrlParts
DIM basename$

UrlParse "https://user:pass@example.com:8080/path/file.txt?q=1#frag", parts
IF parts.hostname <> "example.com" THEN
    PRINT "hostname failed"
    SYSTEM 1
END IF

basename$ = UrlBasename$(parts.pathname, ".txt")
IF basename$ <> "file" THEN
    PRINT "basename failed"
    SYSTEM 1
END IF

PRINT parts.hostname
PRINT basename$
