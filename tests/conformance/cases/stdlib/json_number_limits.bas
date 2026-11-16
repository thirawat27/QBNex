$CONSOLE:ONLY
'$IMPORT:'json'
' JSON numbers near the DOUBLE limit: a leading "0." is no overflow, and a
' mantissa above the largest DOUBLE is clamped to it.
PRINT Json_Num#(Json_Parse%("0.1e307")); Json_Num#(Json_Parse%("0.5e308"))
PRINT Json_Num#(Json_Parse%("1.8e308")); Json_Num#(Json_Parse%("-1.8e308"))
PRINT Json_Num#(Json_Parse%("1e309")); Json_Num#(Json_Parse%("-12.5e2")); Json_Num#(Json_Parse%("0.001"))
SYSTEM
