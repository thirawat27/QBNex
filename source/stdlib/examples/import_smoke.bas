' ============================================================================
' QBNex Import Smoke Test
' ============================================================================
' Metacommands live inside comment lines: '$IMPORT:'module'
' Place the import line before the first SUB/FUNCTION or code that uses stdlib.
' ============================================================================

'$IMPORT:'qbnex'

SUB ImportSmoke ()
    PRINT "Platform: "; Env_Platform$
    PRINT "Home: "; Env_GetHome$
    PRINT "Joined path: "; Path_Join$("root", "child/file.txt")
    PRINT "Extension: "; Path_Extension$("child/file.txt")
END SUB
