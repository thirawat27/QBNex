$CONSOLE:ONLY
'$IMPORT:'collections'
' A method call without brackets to a FUNCTION with an OPTIONAL parameter, on
' the line of the DIM of its variable (st.Join$ is List_Join$(st), sep ", ").
DIM st AS List: st.Add "a": st.Add "b": PRINT st.Join$; st.Pop$; st.Count
SYSTEM
