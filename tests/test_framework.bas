' Minimal test framework bootstrap used by source\qbnex.bas during startup.
' The full implementation has not been restored yet, but the compiler entrypoint
' should not fail just because the optional test harness is absent.

DIM SHARED TestFrameworkInitialized AS _BYTE

SUB InitTestFramework
    TestFrameworkInitialized = -1
END SUB
