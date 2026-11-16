$CONSOLE:ONLY
' DIM SHARED and a SUB of the program after an imported module that has SUBs
'$IMPORT:'collections'
DIM SHARED total AS LONG
DIM items AS List
items.Add "alpha"
items.Add "beta"
AddCount items.Count
AddCount 40
PRINT "total"; total
SYSTEM

SUB AddCount (n AS LONG)
    total = total + n
END SUB
