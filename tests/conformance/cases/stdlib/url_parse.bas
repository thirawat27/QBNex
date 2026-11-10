$CONSOLE:ONLY
'$IMPORT:'url'
' URL parsing, query parameters, formatting and RFC 3986 resolution.
DIM u AS UrlParts
UrlParse "HTTPS://user:pw@Example.COM:8080/a/b/file.txt?q=1&x=%41%zz&s=a+b#frag", u
PRINT "protocol ["; RTRIM$(u.protocol); "] user ["; RTRIM$(u.username); "] pw ["; RTRIM$(u.password); "]"
PRINT "hostname ["; RTRIM$(u.hostname); "] port"; u.port; " host ["; RTRIM$(u.host); "]"
PRINT "pathname ["; RTRIM$(u.pathname); "] search ["; RTRIM$(u.search); "] hash ["; RTRIM$(u.hash); "]"
PRINT "origin ["; RTRIM$(u.origin); "] queryCount"; u.queryCount
PRINT "format ["; UrlFormat$(u); "]"
PRINT "basename ["; UrlBasename$(u.pathname, ".txt"); "] ext ["; UrlExtname$(u.pathname); "] dir ["; UrlDirname$(u.pathname); "]"
PRINT "q ["; UrlGetQueryParam$(u, "q"); "] x ["; UrlGetQueryParam$(u, "x"); "] s ["; UrlGetQueryParam$(u, "s"); "] none ["; UrlGetQueryParam$(u, "none"); "]"
UrlSetQueryParam u, "q", "two words&more"
UrlSetQueryParam u, "new", "v"
PRINT "after set q ["; UrlGetQueryParam$(u, "q"); "] new ["; UrlGetQueryParam$(u, "new"); "] count"; u.queryCount
PRINT "search ["; RTRIM$(u.search); "]"
PRINT "href ["; RTRIM$(u.href); "]"
UrlParse "http://h:65535", u
PRINT "port"; u.port; " path ["; RTRIM$(u.pathname); "]"
UrlParse "mailto:someone@example.com", u
PRINT "mailto ["; RTRIM$(u.protocol); "] ["; RTRIM$(u.pathname); "] host ["; RTRIM$(u.hostname); "]"

PRINT "encode ["; UrlEncodeComponent$("a b/c?d=e&f~_.-"); "] decode ["; UrlDecode$("a%20b%2Fc%zz%4"); "]"
PRINT "normalize ["; UrlNormalizePath$("/a/b/c/./../../g"); "] ["; UrlNormalizePath$("mid/content=5/../6"); "] ["; UrlNormalizePath$("/a/.."); "]"

' RFC 3986 section 5.4 examples
base$ = "http://a/b/c/d;p?q"
DATA "g:h","g","./g","g/","/g","//g","?y","g?y","#s","g#s","g?y#s",";x","g;x","g;x?y#s",""
DATA ".","./","..","../","../g","../..","../../","../../g","../../../g","../../../../g","/./g","/../g","g.","..g","g..","./../g","./g/.","g/./h","g/../h","g;x=1/./y","g;x=1/../y"
FOR i% = 1 TO 36
    READ rel$
    DIM r AS UrlParts
    UrlResolve base$, rel$, r
    PRINT " ["; rel$; "] -> "; UrlFormat$(r)
NEXT
PRINT UrlPathToFile$("C:\dir name\f#1.txt"); " "; UrlFileToPath$("file:///C:/dir%20name/f.txt"); " "; UrlFileToPath$("file:///usr/lib")
PRINT "done"
SYSTEM
