$CONSOLE:ONLY
'$IMPORT:'ext.regex'
'$IMPORT:'ext.hash'
' qb-ext through DECLARE LIBRARY: regular expressions, digests, Base64, hex.
PRINT Regex_IsMatch&("^\d{3}-\d{4}$", "555-1234"); Regex_IsMatch&("^\d+$", "12a")
p& = Regex_Find&("(\w+)@(\w+)\.com", "mail bob@example.com now", 1)
PRINT p&; Regex_Match$; " groups"; Regex_GroupCount&; Regex_Group$(1); " "; Regex_Group$(2)
PRINT Regex_Find&("a", "banana", 3); Regex_Find&("z", "banana", 1); "["; Regex_Match$; "]"
PRINT Regex_Replace$("(\d+)", "a1b22c333", "<$1>"); " "; Regex_ReplaceFirst$("\s+", "x   y   z", "_")
n& = Regex_Split&("\s*,\s*", "a , b,c ,  d")
PRINT n&;
FOR i& = 0 TO n& - 1: PRINT "["; Regex_Part$(i&); "]";: NEXT: PRINT
PRINT Regex_IsMatch&("(", "x"); " error: "; LEFT$(Ext_LastError$, 23)
PRINT Regex_IsMatch&("(?i)HELLO", "say hello")

PRINT Hash_CRC32$("123456789"); " "; HEX$(Hash_CRC32Value~&("123456789"))
PRINT Hash_MD5$("abc")
PRINT Hash_SHA1$("abc")
PRINT Hash_SHA256$("abc")
PRINT LEFT$(Hash_SHA512$("abc"), 32)
PRINT Hash_MD5$("")
b$ = Base64_Encode$("foobar" + CHR$(0) + CHR$(255))
PRINT b$; " "; LEN(Base64_Decode$(b$)); (Base64_Decode$(b$) = "foobar" + CHR$(0) + CHR$(255))
PRINT Base64Url_Encode$(CHR$(251) + CHR$(255)); " "; Hex_Encode$(Base64_Decode$("-_8"))
PRINT Hex_Encode$("AZ" + CHR$(0)); " "; Hex_Decode$("41 42 43"); " ["; Hex_Decode$("zz"); "] "; Ext_LastError$
PRINT "done"
SYSTEM
