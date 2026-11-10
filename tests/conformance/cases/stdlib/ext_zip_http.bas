$CONSOLE:ONLY
'$IMPORT:'ext.zip'
'$IMPORT:'ext.http'
' qb-ext zip archives and HTTP against the in-process test server (no internet).
DECLARE LIBRARY
    FUNCTION qb_ext_test_http_server& (BYVAL count AS LONG)
END DECLARE
z$ = "ext_zip_test.zip"
h& = Zip_Create&(z$)
PRINT "create"; (h& > 0); Zip_AddData&(h&, "hello.txt", "hello hello hello"); Zip_AddFolder&(h&, "sub/");
PRINT Zip_AddData&(h&, "sub/bin.dat", CHR$(0) + CHR$(1) + CHR$(255)); Zip_AddData&(999, "x", ""); Zip_Close&(h&)
n& = Zip_List&(z$)
PRINT "members"; n&
FOR i& = 0 TO n& - 1: PRINT " "; Zip_EntryName$(i&); Zip_EntrySize&&(i&): NEXT
PRINT "[" + Zip_ReadEntry$(z$, "hello.txt") + "]"; LEN(Zip_ReadEntry$(z$, "sub/bin.dat")); "["; Zip_ReadEntry$(z$, "nope"); "]"
PRINT "extract"; Zip_Extract&(z$, "ext_zip_out")
OPEN "ext_zip_out/sub/bin.dat" FOR BINARY AS #1: PRINT "extracted size"; LOF(1): CLOSE #1
PRINT "missing archive"; Zip_List&("no_such_file.zip")
KILL z$: KILL "ext_zip_out/hello.txt": KILL "ext_zip_out/sub/bin.dat": RMDIR "ext_zip_out/sub": RMDIR "ext_zip_out"

port& = qb_ext_test_http_server&(4)
base$ = "http://127.0.0.1:" + LTRIM$(STR$(port&))
st& = Http_Get&(base$ + "/hello?a=1", 5000)
PRINT "get"; st&; "["; Http_Body$; "] "; Http_Header$("X-Server"); " "; Http_Header$("content-type")
st& = Http_Post&(base$ + "/p", "text/plain", "data" + CHR$(0) + "!", 5000)
PRINT "post"; st&; LEN(Http_Body$); RIGHT$(Http_Body$, 6) = "data" + CHR$(0) + "!"
st& = Http_Request&("PUT", base$ + "/missing", "X-Test: put" + CHR$(13) + CHR$(10), "", 5000)
PRINT "put"; st&; "["; Http_Body$; "]"
st& = Http_Get&(base$ + "/redirect", 5000)
PRINT "redirect"; st&; "["; Http_Body$; "]"
st& = Http_Get&("not a url", 2000)
PRINT "bad url"; st&; (LEN(Ext_LastError$) > 0); "["; Http_Body$; "]"
PRINT "done"
SYSTEM
