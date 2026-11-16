$CONSOLE:ONLY
'$IMPORT:'url'
'$IMPORT:'path'
' URL parsing, query parameters, formatting and RFC 3986 resolution.
DIM u AS Url
u.Parse "HTTPS://user:pw@Example.COM:8080/a/b/file.txt?q=1&x=%41%zz&s=a+b#frag"
PRINT "protocol ["; u.protocol; "] user ["; u.username; "] pw ["; u.password; "]"
PRINT "hostname ["; u.hostname; "] port"; u.port; " host ["; u.host; "]"
PRINT "pathname ["; u.pathname; "] search ["; u.search; "] hash ["; u.hash; "]"
PRINT "origin ["; u.origin; "] queryCount"; u.queryCount
PRINT "format ["; u.Format$; "]"
' the path of a URL with the path module
PRINT "base ["; Path_Base$(u.pathname); "] stem ["; Path_Stem$(u.pathname); "] ext ["; Path_Ext$(u.pathname); "]"
PRINT "q ["; u.Query$("q"); "] x ["; u.Query$("x"); "] s ["; u.Query$("s"); "] none ["; u.Query$("none"); "]"
u.SetQuery "q", "two words&more"
u.SetQuery "new", "v"
PRINT "after set q ["; u.Query$("q"); "] new ["; u.Query$("new"); "] count"; u.queryCount
PRINT "search ["; u.search; "]"
PRINT "href ["; u.href; "]"
u.Parse "http://h:65535"
PRINT "port"; u.port; " path ["; u.pathname; "]"
u.Parse "mailto:someone@example.com"
PRINT "mailto ["; u.protocol; "] ["; u.pathname; "] host ["; u.hostname; "] origin ["; u.origin; "]"
DIM v AS Url
v.SetQuery "a b", "c&d"
PRINT "new url ["; v.Format$; "]"; v.queryCount; " ["; v.Query$("a b"); "]"

PRINT "encode ["; Url_Encode$("a b/c?d=e&f~_.-"); "] decode ["; Url_Decode$("a%20b%2Fc%zz%4"); "]"
PRINT "normalize ["; Url_Resolve$("http://h", "/a/b/c/./../../g"); "] ["; Url_Resolve$("http://h/mid/x", "content=5/../6"); "] ["; Url_Resolve$("http://h", "/a/.."); "]"

' RFC 3986 section 5.4 examples
base$ = "http://a/b/c/d;p?q"
DATA "g:h","g","./g","g/","/g","//g","?y","g?y","#s","g#s","g?y#s",";x","g;x","g;x?y#s",""
DATA ".","./","..","../","../g","../..","../../","../../g","../../../g","../../../../g","/./g","/../g","g.","..g","g..","./../g","./g/.","g/./h","g/../h","g;x=1/./y","g;x=1/../y"
FOR i% = 1 TO 36
    READ rel$
    PRINT " ["; rel$; "] -> "; Url_Resolve$(base$, rel$)
NEXT
PRINT Url_FromPath$("C:\dir name\f#1.txt"); " "; Url_ToPath$("file:///C:/dir%20name/f.txt"); " "; Url_ToPath$("file:///usr/lib"); " "; Url_ToPath$("relative/x"); " "; Url_FromPath$("/usr/a b")
PRINT "done"
SYSTEM
