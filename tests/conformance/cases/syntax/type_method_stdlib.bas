$CONSOLE:ONLY
'$IMPORT:'collections'
' Method syntax on the TYPEs of the standard library: a List needs no Init, a
' Dict's fields are Lists (d.KeyList.Add), and a List passed to a SUB.
DIM names AS List
names.Add "Ann": names.Add "Bob"
names.Insert 0, "Cy"
PRINT names.Count; names.Get$(-1); " "; names.Join$(", "); names.Has("x") = 0
DIM d AS Dict
d.Set "a", "1"
d.KeyList.Add "b"
PRINT d.Count; d.KeyList.Count; d.KeyList.Join$("|"); d.Has("b"); "["; d.Get$("b"); "]"
d.Remove "b"
d.Set "c", "3"
PRINT d.Count; d.Key$(1); d.Value$(1); d.Get$("zz", "none")
Fill names, 3
PRINT names.Count; names.Pop$; names.Pop$(0); names.Count
IF names.Has("Ann") THEN PRINT "has Ann" ELSE PRINT "no Ann"
names.Clear
PRINT names.Count
SYSTEM

SUB Fill (l AS List, n AS LONG)
    FOR i = 1 TO n
        l.Add "n" + LTRIM$(STR$(i))
    NEXT
END SUB
