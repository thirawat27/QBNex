$CONSOLE:ONLY
'$IMPORT:'url'
' Components of any length are kept whole (they were cut to fixed-length fields),
' and a port outside 0..65535 or not a number is an error.
DIM u AS UrlParts
UrlParse "http://h/p?" + STRING$(3000, "a") + "=1", u
PRINT LEN(UrlFormat$(u)); LEN(u.search); u.queryCount; "["; UrlLastError$; "]"
UrlParse "http://" + STRING$(300, "h") + ".com:8080/" + STRING$(2000, "p") + "#" + STRING$(700, "f"), u
PRINT LEN(u.hostname); u.port; LEN(u.pathname); LEN(u.hash); LEN(UrlFormat$(u))
UrlSetQueryParam u, "k", STRING$(5000, "v")
PRINT LEN(UrlGetQueryParam$(u, "k"))
UrlParse "http://h:99999/x", u
PRINT u.port; "["; UrlLastError$; "] "; u.hostname; u.pathname
UrlParse "http://h:80a/x", u
PRINT u.port; "["; UrlLastError$; "]"
UrlParse "http://h:-1/x", u
PRINT u.port; "["; UrlLastError$; "]"
UrlParse "http://h:/x", u
PRINT u.port; "["; UrlLastError$; "] "; UrlFormat$(u)
UrlParse "http://[::1]:65535/x", u
PRINT u.port; "["; UrlLastError$; "] "; u.hostname
SYSTEM
