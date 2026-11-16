$CONSOLE:ONLY
' 'qbnex' imported in an $INCLUDE file, with one of its modules imported before
' it and another one after it: each module is in the program once.
'$INCLUDE:'import_qbnex_in_include.bi'
'$IMPORT:'collections'
DIM st AS List
st.Add "p"
PRINT st.Pop$; Str_Repeat$("ab", 2); QBNEX_EnsureClass&("X", 0); LastError$ = ""
SYSTEM
