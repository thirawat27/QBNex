$CONSOLE:ONLY
' The module is imported at the end, after the program's own SUBs: its
' module-level DIM SHARED must still belong to the main module.
DIM SHARED names AS List
AddName "x"
AddName "y"
AddName "z"
PRINT "names"; List_Count(names); names.Count
SYSTEM

SUB AddName (s AS STRING)
    names.Add s
END SUB

'$IMPORT:'collections'
