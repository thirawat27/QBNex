' Minimal regression-test bootstrap used by source\qbnex.bas during startup.
' Keep this lightweight until the real regression suite is restored.

DIM SHARED RegressionTestsInitialized AS _BYTE

SUB InitRegressionTests
    RegressionTestsInitialized = -1
END SUB
