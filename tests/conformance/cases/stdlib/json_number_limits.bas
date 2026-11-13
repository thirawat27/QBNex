$CONSOLE:ONLY
'$IMPORT:'json'
' JSON numbers near the DOUBLE limit: a leading "0." is no overflow, and a
' mantissa above the largest DOUBLE is clamped to it.
Json_Init
PRINT JsonGetNumber#(JSON_Parse%("0.1e307")); JsonGetNumber#(JSON_Parse%("0.5e308"))
PRINT JsonGetNumber#(JSON_Parse%("1.8e308")); JsonGetNumber#(JSON_Parse%("-1.8e308"))
PRINT JsonGetNumber#(JSON_Parse%("1e309")); JsonGetNumber#(JSON_Parse%("-12.5e2")); JsonGetNumber#(JSON_Parse%("0.001"))
SYSTEM
