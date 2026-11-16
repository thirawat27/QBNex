$CONSOLE:ONLY
'$IMPORT:'qbnex'
' List Set / RemoveAt / Pop$ splice in place (a 2000-item queue drains at once),
' Date_Iso$ keeps milliseconds below 1000, Result Context adds where an error
' happened in front of its message.
DIM q AS List
FOR i& = 1 TO 2000: q.Add "item" + STR$(i&): NEXT
FOR i& = 1 TO 2000: s$ = q.Pop$(0): NEXT
PRINT s$; q.Count
DIM l AS List
l.Add "a": l.Add "bb": l.Add "ccc"
l.Set 1, "XYZW": l.RemoveAt 0
PRINT l.Get$(0); "|"; l.Get$(1); "|"; l.Count
l.Set 1, "": l.RemoveAt 1
PRINT l.Get$(0); l.Count
l.Remove "XYZW": l.Remove "nope"
PRINT l.Count
PRINT Date_Iso$(999.6#); " "; Date_Iso$(-0.4#)
DIM r AS Result
r.Fail "boom", 2
r.Context "saving report"
r.Context "port"
PRINT "["; r.Message; "]"; r.Code; r.Failed
r.Ok
PRINT "["; r.Message; "]"; r.Code; r.Failed; "["; r.Value; "]"
r.Context "ignored on success"
PRINT "["; r.Message; "]"
SYSTEM
