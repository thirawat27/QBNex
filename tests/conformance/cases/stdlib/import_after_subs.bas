$CONSOLE:ONLY
' The module is imported at the end, after the program's own SUBs: its
' module-level DIM SHARED must still belong to the main module.
DIM SHARED names AS QBNex_List
List_Init names
AddName "x"
AddName "y"
AddName "z"
PRINT "names"; List_Count(names)
SYSTEM

SUB AddName (s AS STRING)
    List_Add names, s
END SUB

'$IMPORT:'collections.list'
