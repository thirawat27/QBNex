$CONSOLE:ONLY
' DIM SHARED and a SUB of the program after an imported module that has SUBs
'$IMPORT:'collections.list'
DIM SHARED total AS LONG
DIM items AS QBNex_List
List_Init items
List_Add items, "alpha"
List_Add items, "beta"
AddCount List_Count(items)
AddCount 40
PRINT "total"; total
SYSTEM

SUB AddCount (n AS LONG)
    total = total + n
END SUB
