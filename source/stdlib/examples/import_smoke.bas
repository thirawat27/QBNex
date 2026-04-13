' ============================================================================
' QBNex Import Smoke Test
' ============================================================================
' Phase 1 recommendation: place function-only imports at the end of the file.
' This keeps classic BASIC statement ordering valid while still enabling the
' Python-style dotted import syntax.
' ============================================================================

SUB ImportSmoke ()
    PRINT "Platform: "; Env_Platform$
    PRINT "Home: "; Env_GetHome$
    PRINT "Joined path: "; Path_Join$("root", "child/file.txt")
    PRINT "Extension: "; Path_Extension$("child/file.txt")
END SUB

'$IMPORT:'sys.env'
'$IMPORT:'io.path'
