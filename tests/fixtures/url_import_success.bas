'$IMPORT:'url'

DIM parts AS Url

parts.Parse "https://user:pass@example.com:8080/path/file.txt?q=1#frag"
IF parts.hostname <> "example.com" OR parts.port <> 8080 THEN
    PRINT "hostname failed"
    SYSTEM 1
END IF

IF parts.Query$("q") <> "1" OR parts.pathname <> "/path/file.txt" THEN
    PRINT "query failed"
    SYSTEM 1
END IF

PRINT parts.hostname
PRINT parts.pathname
