$CONSOLE:ONLY
' DIM SHARED after importing the json module, and a few json calls
'$IMPORT:'json'
DIM SHARED tag$
tag$ = "json"
v% = JsonCreateNumber%(3)
b% = JsonCreateBoolean%(-1)
s% = JsonCreateString%("hi")
PRINT tag$; JSON_Stringify$(v%); " "; JSON_Stringify$(b%); " "; JSON_Stringify$(s%)
SYSTEM
