$CONSOLE:ONLY
' DIM SHARED after importing the json module, and a few json calls
'$IMPORT:'json'
DIM SHARED tag$
tag$ = "json"
o% = Json_Object
Json_SetNum o%, "n", 3
Json_SetBool o%, "b", -1
Json_SetStr o%, "s", "hi"
PRINT tag$; Json_Stringify$(o%); " "; Json_Str$(o%, "b"); Json_Num#(o%, "n")
SYSTEM
