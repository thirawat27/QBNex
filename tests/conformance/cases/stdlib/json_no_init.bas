$CONSOLE:ONLY
'$IMPORT:'json'
' Regression: JSON used without an init call. The first value used to land in
' unallocated pool arrays ("Subscript out of range" at json.bas line 169).
v% = Json_Parse%("[1,[2,{" + CHR$(34) + "k" + CHR$(34) + ":null}]]")
PRINT v%; Json_Count%(v%); Json_Stringify$(v%)
a% = Json_Array
FOR i% = 1 TO 700
    Json_PushNum a%, i%
NEXT
PRINT Json_Count%(a%); Json_Num#(a%, "699")
SYSTEM
