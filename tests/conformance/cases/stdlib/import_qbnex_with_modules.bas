$CONSOLE:ONLY
'$IMPORT:'collections'
'$IMPORT:'json'
'$IMPORT:'qbnex'
'$IMPORT:'url'
' 'qbnex' next to json and url, and after one of the modules it bundles: every
' import is in the program once (the later ones used to be dropped).
DIM l AS List
l.Add "x"
DIM st AS List
st.Add "p"
v% = Json_Parse%("[1," + CHR$(34) + "a" + CHR$(34) + "]")
PRINT Json_Stringify$(v%); l.Count; st.Pop$
PRINT Json_Type$(v%, "[1]"); " "; Json_Str$(v%, "1")
DIM u AS Url
u.Parse "https://example.com:8080/a?q=1"
PRINT u.hostname; u.port; " "; u.Query$("q")
SYSTEM
