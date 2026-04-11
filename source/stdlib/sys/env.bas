' =============================================================================
' QBNex System Integration — Environment Variables — env.bas
' =============================================================================
'
' Usage:
'
'   '$INCLUDE:'stdlib/sys/env.bas'
'
'   PRINT Env_Get$("PATH")
'   PRINT Env_Get$("HOME", "/tmp")      ' with default
'   PRINT Env_Has("USERPROFILE")        ' -1 / 0
'   Env_Set "MY_VAR", "hello"           ' set (via SHELL setx/export)
'
' =============================================================================

' ---------------------------------------------------------------------------
' FUNCTION  Env_Get$(varName$, [default$])
'   Returns the value of the environment variable, or default$ if unset.
'   QBNex's ENVIRON$ function is used directly.
' ---------------------------------------------------------------------------
FUNCTION Env_Get$ (varName$, default$)
    DIM val AS STRING
    val = ENVIRON$(varName$)
    IF LEN(val) = 0 THEN val = default$
    Env_Get$ = val
END FUNCTION

FUNCTION Env_GetReq$ (varName$)
    DIM val AS STRING
    val = ENVIRON$(varName$)
    IF LEN(val) = 0 THEN
        PRINT "QBNex Env Error: required environment variable '" + varName$ + "' is not set."
        END 1
    END IF
    Env_GetReq$ = val
END FUNCTION

' ---------------------------------------------------------------------------
' FUNCTION  Env_Has&(varName$)  — returns -1 if the variable is defined
' ---------------------------------------------------------------------------
FUNCTION Env_Has& (varName$)
    Env_Has& = (LEN(ENVIRON$(varName$)) > 0)
END FUNCTION

' ---------------------------------------------------------------------------
' FUNCTION  Env_GetAll$()
'   Returns all environment variables as newline-separated KEY=VALUE pairs.
' ---------------------------------------------------------------------------
FUNCTION Env_GetAll$ ()
    DIM i AS LONG, entry AS STRING, result AS STRING
    i = 1: result = ""
    DO
        entry = ENVIRON$(i)
        IF LEN(entry) = 0 THEN EXIT DO
        IF i > 1 THEN result = result + CHR$(10)
        result = result + entry
        i = i + 1
    LOOP
    Env_GetAll$ = result
END FUNCTION

' ---------------------------------------------------------------------------
' FUNCTION  Env_Platform$()  — returns "WINDOWS", "LINUX", or "MACOS"
' ---------------------------------------------------------------------------
FUNCTION Env_Platform$ ()
    DIM os AS STRING
    os = _OS$
    IF INSTR(os, "[WIN]") THEN Env_Platform$ = "WINDOWS": EXIT FUNCTION
    IF INSTR(os, "[LINUX]") THEN Env_Platform$ = "LINUX": EXIT FUNCTION
    IF INSTR(os, "[MACOSX]") THEN Env_Platform$ = "MACOS": EXIT FUNCTION
    Env_Platform$ = "UNKNOWN"
END FUNCTION

FUNCTION Env_IsWindows& ()
    Env_IsWindows& = (INSTR(_OS$, "[WIN]") > 0)
END FUNCTION

FUNCTION Env_IsLinux& ()
    Env_IsLinux& = (INSTR(_OS$, "[LINUX]") > 0)
END FUNCTION

FUNCTION Env_IsMac& ()
    Env_IsMac& = (INSTR(_OS$, "[MACOSX]") > 0)
END FUNCTION

FUNCTION Env_Is64Bit& ()
    Env_Is64Bit& = (INSTR(_OS$, "[64BIT]") > 0)
END FUNCTION
