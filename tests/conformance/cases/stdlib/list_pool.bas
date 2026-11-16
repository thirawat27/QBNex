$CONSOLE:ONLY
'$IMPORT:'qbnex'
' Lists have no fixed limit (there were 256 handles; a Dict takes two), items
' are read in constant time, and the count is the list's own.
DIM d(1 TO 300) AS Dict
FOR i = 1 TO 300: d(i).Set "k", STR$(i): NEXT
PRINT d(300).Get$("k", "-"); d(300).Count; d(1).Get$("x", "-")
FOR i = 1 TO 300: d(i).Free: NEXT
DIM h AS Set
FOR i = 1 TO 3000: h.Add STR$(i MOD 1500): NEXT
PRINT h.Count; h.Has(STR$(7)); h.Has("7")
h.Remove STR$(7): PRINT h.Count; h.Has(STR$(7))
DIM l AS List, c AS List
FOR i = 1 TO 20000: l.Add STR$(i): NEXT
s& = 0: FOR i = 0 TO 19999: s& = s& + VAL(l.Get$(i)): NEXT
PRINT s&
c = l: l.Add "x": PRINT c.Count; l.IndexOf("x")
l.RemoveAt 0: PRINT l.Get$(0); l.Count
l.Set 0, CHR$(0) + "z": PRINT LEN(l.Get$(0))
l.Free: PRINT l.Count; "["; l.Get$(3); "]"
' a queue and a stack are a List: Add, then Pop$(0) or Pop$
DIM q AS List
q.Add "a": q.Add "b": PRINT q.Pop$(0); q.Count
DIM st AS List
st.Add "a": st.Add "b": PRINT st.Pop$; st.Count
PRINT "["; st.Pop$; "]["; st.Pop$; "]"; st.Count
SYSTEM
