$CONSOLE:ONLY
'$IMPORT:'collections'
' Method syntax on a field that is not the first one of its TYPE: d.ValueList is
' the second field of Dict, p.Second the second field of Pair.
TYPE Pair
    First AS List
    Second AS List
END TYPE
DIM d AS Dict
d.KeyList.Add "k"
d.ValueList.Add "v"
PRINT d.Count; d.Get$("k"); d.ValueList.Count
DIM p AS Pair
p.Second.Add "x"
PRINT p.Second.Count; p.Second.Get$(0)
SYSTEM
