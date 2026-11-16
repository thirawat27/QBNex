$CONSOLE:ONLY
'$IMPORT:'url'
' Components of any length are kept whole (they were cut to fixed-length fields),
' and a port outside 0..65535 or not a number is an error.
DIM u AS Url
u.Parse "http://h/p?" + STRING$(3000, "a") + "=1"
PRINT LEN(u.Format$); LEN(u.search); u.queryCount; "["; LastError$; "]"
u.Parse "http://" + STRING$(300, "h") + ".com:8080/" + STRING$(2000, "p") + "#" + STRING$(700, "f")
PRINT LEN(u.hostname); u.port; LEN(u.pathname); LEN(u.hash); LEN(u.Format$)
u.SetQuery "k", STRING$(5000, "v")
PRINT LEN(u.Query$("k")); LEN(u.href)
u.Parse "http://h:99999/x"
PRINT u.port; "["; LastError$; "] "; u.hostname; u.pathname
u.Parse "http://h:80a/x"
PRINT u.port; "["; LastError$; "]"
u.Parse "http://h:-1/x"
PRINT u.port; "["; LastError$; "]"
u.Parse "http://h:/x"
PRINT u.port; "["; LastError$; "] "; u.Format$
u.Parse "http://[::1]:65535/x"
PRINT u.port; "["; LastError$; "] "; u.hostname
SYSTEM
