' ============================================================================
' QBNex Runtime Smoke Test
' ============================================================================
' Requires a qb.exe built from current source/qbnex.bas (bootstrap); installed
' snapshots may not include compiler fixes for stdlib multi-arg calls.
' ============================================================================

'$IMPORT:'qbnex'

DIM failures AS LONG

IF Text_PadRight$("QB", 4, ".") <> "QB.." THEN failures = failures + 1
IF CSV_Row3$("a", "b", "c") <> "a,b,c" THEN failures = failures + 1
IF Math_Clamp#(12#, 0#, 10#) <> 10# THEN failures = failures + 1
IF LEN(Path_Join$("root", "child/file.txt")) = 0 THEN failures = failures + 1
IF LEN(Env_Platform$) = 0 THEN failures = failures + 1

IF failures <> 0 THEN
    PRINT "RUNTIME_SMOKE_FAIL "; failures
    SYSTEM 1
END IF

PRINT "RUNTIME_SMOKE_OK"
