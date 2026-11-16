$CONSOLE:ONLY
'$IMPORT:'regex'
'$IMPORT:'hash'
' The native regex and hash modules: regular expressions, digests, Base64, hex.
PRINT Regex_Test&("^\d{3}-\d{4}$", "555-1234"); Regex_Test&("^\d+$", "12a")
m$ = Regex_Find$("(\w+)@(\w+)\.com", "mail bob@example.com now")
PRINT Regex_Pos&; m$; " groups "; Regex_Group$(0); " "; Regex_Group$(1); " "; Regex_Group$(2); " ["; Regex_Group$(3); "]"
m$ = Regex_Find$("a", "banana", 3)
PRINT Regex_Pos&; m$;
m$ = Regex_Find$("z", "banana")
PRINT Regex_Pos&; "["; m$; "] ["; Regex_Group$(0); "]"
PRINT Regex_Replace$("(\d+)", "a1b22c333", "<$1>"); " "; Regex_Replace$("\s+", "x   y   z", "_", -1)
PRINT Regex_Replace$("(?<user>\w+)@(?<host>\w+)", "bob@example", "${host} has ${user}, $$5")
DIM parts AS List
Regex_Split "\s*,\s*", "a , b,c ,  d", parts
PRINT parts.Count;
FOR i& = 0 TO parts.Count - 1: PRINT "["; parts.Get$(i&); "]";: NEXT: PRINT
DIM found AS List
Regex_FindAll "\d+", "a1b22c333d", found
PRINT found.Count; found.Join$("|")
Regex_FindAll "x*", "ab", found
PRINT found.Count; "["; found.Join$("|"); "]"
PRINT Regex_Test&("(", "x"); " error: "; LEFT$(LastError$, 18)
PRINT Regex_Test&("(?i)HELLO", "say hello"); " ["; LastError$; "]"

PRINT LCASE$(HEX$(Hash_Crc32~&("123456789"))); Hash_Crc32~&("")
PRINT Hash_Md5$("abc")
PRINT Hash_Sha1$("abc")
PRINT Hash_Sha256$("abc")
PRINT LEFT$(Hash_Sha512$("abc"), 32)
PRINT Hash_Md5$("")
b$ = Base64_Encode$("foobar" + CHR$(0) + CHR$(255))
PRINT b$; " "; LEN(Base64_Decode$(b$)); (Base64_Decode$(b$) = "foobar" + CHR$(0) + CHR$(255))
PRINT Base64_Encode$(CHR$(251) + CHR$(255), -1); " "; Base64_Encode$(CHR$(251) + CHR$(255)); " "; Hex_Encode$(Base64_Decode$("-_8"))
PRINT "["; Base64_Decode$("@@@"); "] "; LastError$
PRINT Hex_Encode$("AZ" + CHR$(0)); " "; Hex_Decode$("41 42 43"); " ["; Hex_Decode$("zz"); "] "; LastError$
PRINT Hex_Decode$("6869"); " ["; LastError$; "]"
PRINT "done"
SYSTEM
