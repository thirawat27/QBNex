' ============================================================================
' QBNex Standard Library - System: Environment
' ============================================================================

FUNCTION Env_Get$ (varName AS STRING, defaultValue AS STRING)
    DIM value AS STRING

    value = ENVIRON$(varName)
    IF LEN(value) = 0 THEN
        Env_Get = defaultValue
    ELSE
        Env_Get = value
    END IF
END FUNCTION

FUNCTION Env_Has& (varName AS STRING)
    Env_Has = LEN(ENVIRON$(varName)) <> 0
END FUNCTION

FUNCTION Env_Platform$ ()
    IF INSTR(_OS$, "WIN") THEN
        Env_Platform = "WINDOWS"
    ELSEIF INSTR(_OS$, "LINUX") THEN
        Env_Platform = "LINUX"
    ELSEIF INSTR(_OS$, "MAC") THEN
        Env_Platform = "MACOS"
    ELSE
        Env_Platform = "UNKNOWN"
    END IF
END FUNCTION

FUNCTION Env_Is64Bit& ()
    Env_Is64Bit = INSTR(_OS$, "64BIT") <> 0
END FUNCTION

FUNCTION Env_GetHome$ ()
    IF INSTR(_OS$, "WIN") THEN
        Env_GetHome = Env_Get$("USERPROFILE", "C:\")
    ELSE
        Env_GetHome = Env_Get$("HOME", "/")
    END IF
END FUNCTION
