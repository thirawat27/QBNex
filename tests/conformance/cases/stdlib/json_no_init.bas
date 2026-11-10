$CONSOLE:ONLY
'$IMPORT:'json'
' Regression: JSON used without Json_Init. The first value used to land in
' unallocated pool arrays ("Subscript out of range" at json.bas line 169).
v% = JSON_Parse%("[1,[2,{" + CHR$(34) + "k" + CHR$(34) + ":null}]]")
PRINT v%; JsonArrayLength%(v%); JSON_Stringify$(v%)
a% = JsonCreateArray%
FOR i% = 1 TO 700
    JsonArrayPush a%, JsonCreateNumber%(i%)
NEXT
PRINT JsonArrayLength%(a%); JsonGetNumber#(JsonArrayGet%(a%, 699))
SYSTEM
