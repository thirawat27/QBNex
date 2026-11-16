$CONSOLE:ONLY
'$IMPORT:'zip'
'$IMPORT:'http'
' The native zip and http modules; HTTP against the in-process test server (no internet).
DECLARE LIBRARY
    FUNCTION qb_ext_test_http_server& (BYVAL count AS LONG)
END DECLARE
z$ = "ext_zip_test.zip"
h& = Zip_Create&(z$)
PRINT "create"; (h& > 0); "["; LastError$; "]"
Zip_Write h&, "hello.txt", "hello hello hello"
Zip_AddFolder h&, "sub/"
Zip_Write h&, "sub/bin.dat", CHR$(0) + CHR$(1) + CHR$(255)
PRINT "["; LastError$; "]";
Zip_Write 999, "x", ""
PRINT " bad handle "; (LEN(LastError$) > 0);
Zip_Close h&
PRINT " closed ["; LastError$; "]"
DIM names AS List
Zip_List z$, names
PRINT "members"; names.Count; names.Join$(" ")
PRINT "[" + Zip_Read$(z$, "hello.txt") + "]"; LEN(Zip_Read$(z$, "sub/bin.dat")); "["; Zip_Read$(z$, "nope"); "] "; (LEN(LastError$) > 0)
Zip_Extract z$, "ext_zip_out"
PRINT "extract ["; LastError$; "]"
OPEN "ext_zip_out/sub/bin.dat" FOR BINARY AS #1: PRINT "extracted size"; LOF(1): CLOSE #1
Zip_List "no_such_file.zip", names
PRINT "missing archive"; names.Count; (LEN(LastError$) > 0)
KILL z$: KILL "ext_zip_out/hello.txt": KILL "ext_zip_out/sub/bin.dat": RMDIR "ext_zip_out/sub": RMDIR "ext_zip_out"

port& = qb_ext_test_http_server&(5) ' get, post, put, redirect + its target
base$ = "http://127.0.0.1:" + LTRIM$(STR$(port&))
b$ = Http_Get$(base$ + "/hello?a=1", 5000)
PRINT "get"; Http_Status&; "["; b$; "] "; Http_Header$("X-Server"); " "; Http_Header$("content-type"); " ["; Http_Header$("none"); "]"
b$ = Http_Post$(base$ + "/p", "data" + CHR$(0) + "!", "text/plain", 5000)
PRINT "post"; Http_Status&; LEN(b$); RIGHT$(b$, 6) = "data" + CHR$(0) + "!"
b$ = Http_Request$("PUT", base$ + "/missing", "", "X-Test: put", 5000)
PRINT "put"; Http_Status&; "["; b$; "]"
b$ = Http_Get$(base$ + "/redirect", 5000)
PRINT "redirect"; Http_Status&; "["; b$; "]"
b$ = Http_Get$("not a url", 2000)
PRINT "bad url"; Http_Status&; (LEN(LastError$) > 0); "["; b$; "] ["; Http_Headers$; "]"
PRINT "done"
SYSTEM
