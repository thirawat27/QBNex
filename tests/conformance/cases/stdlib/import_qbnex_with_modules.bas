$CONSOLE:ONLY
'$IMPORT:'collections.list'
'$IMPORT:'json'
'$IMPORT:'qbnex'
'$IMPORT:'url'
' 'qbnex' next to json and url, and after one of the modules it bundles: every
' import is in the program once (the later ones used to be dropped).
DIM l AS QBNex_List
List_Init l
List_Add l, "x"
DIM st AS QBNex_Stack
Stack_Init st
Stack_Push st, "p"
v% = JSON_Parse%("[1," + CHR$(34) + "a" + CHR$(34) + "]")
PRINT JSON_Stringify$(v%); l.Count; Stack_Pop$(st)
PRINT JsonGetType%(JsonArrayGet%(v%, 1)) = JSON_TYPE_STRING; Json_String$("q")
DIM u AS UrlParts
UrlParse "https://example.com:8080/a?q=1", u
PRINT RTRIM$(u.hostname); u.port
SYSTEM
