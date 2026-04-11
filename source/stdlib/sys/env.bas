' ============================================================================
' QBNex Standard Library - System: Environment Variables
' ============================================================================
' Environment variable access and platform detection
' ============================================================================

' ============================================================================
' FUNCTION: Env_Get
' Get environment variable with default fallback
' ============================================================================
FUNCTION Env_Get$ (NAME AS STRING, defaultValue AS STRING)
    DIM value AS STRING
    value = ENVIRON$(NAME)
    
    IF LEN(value) = 0 THEN
        Env_Get = defaultValue
    ELSE
        Env_Get = value
    END IF
END FUNCTION

' ============================================================================
' FUNCTION: Env_GetReq
' Get required environment variable (terminates if missing)
' ============================================================================
FUNCTION Env_GetReq$ (NAME AS STRING)
    DIM value AS STRING
    value = ENVIRON$(NAME)
    
    IF LEN(value) = 0 THEN
        PRINT "FATAL: Required environment variable not set: "; NAME
        SYSTEM
    END IF
    
    Env_GetReq = value
END FUNCTION

' ============================================================================
' FUNCTION: Env_Platform
' Get platform name (WINDOWS, LINUX, MACOS)
' ============================================================================
FUNCTION Env_Platform$ ()
    DIM os AS STRING
    os = _OS$
    
    IF INSTR(os, "WIN") > 0 THEN
        Env_Platform = "WINDOWS"
    ELSEIF INSTR(os, "LIN") > 0 THEN
        Env_Platform = "LINUX"
    ELSEIF INSTR(os, "MAC") > 0 THEN
        Env_Platform = "MACOS"
    ELSE
        Env_Platform = "UNKNOWN"
    END IF
END FUNCTION

' ============================================================================
' FUNCTION: Env_Is64Bit
' Check if running on 64-bit platform
' ============================================================================
FUNCTION Env_Is64Bit& ()
    Env_Is64Bit = INSTR(_OS$, "64") > 0
END FUNCTION

' ============================================================================
' FUNCTION: Env_IsWindows
' Check if running on Windows
' ============================================================================
FUNCTION Env_IsWindows& ()
    Env_IsWindows = INSTR(_OS$, "WIN") > 0
END FUNCTION

' ============================================================================
' FUNCTION: Env_IsLinux
' Check if running on Linux
' ============================================================================
FUNCTION Env_IsLinux& ()
    Env_IsLinux = INSTR(_OS$, "LIN") > 0
END FUNCTION

' ============================================================================
' FUNCTION: Env_IsMac
' Check if running on macOS
' ============================================================================
FUNCTION Env_IsMac& ()
    Env_IsMac = INSTR(_OS$, "MAC") > 0
END FUNCTION

' ============================================================================
' FUNCTION: Env_GetAll
' Get all environment variables as newline-separated KEY=VALUE
' ============================================================================
FUNCTION Env_GetAll$ ()
    DIM result AS STRING
    DIM i AS LONG
    DIM envVar AS STRING
    
    result = ""
    i = 1
    
    DO
        envVar = ENVIRON$(i)
        IF LEN(envVar) = 0 THEN EXIT DO
        
        IF LEN(result) > 0 THEN
            result = result + CHR$(13) + CHR$(10)
        END IF
        result = result + envVar
        
        i = i + 1
    LOOP
    
    Env_GetAll = result
END FUNCTION

' ============================================================================
' FUNCTION: Env_Has
' Check if environment variable exists
' ============================================================================
FUNCTION Env_Has& (NAME AS STRING)
    Env_Has = LEN(ENVIRON$(NAME)) > 0
END FUNCTION

' ============================================================================
' FUNCTION: Env_GetHome
' Get user home directory
' ============================================================================
FUNCTION Env_GetHome$ ()
    IF Env_IsWindows THEN
        Env_GetHome = Env_Get("USERPROFILE", "C:\")
    ELSE
        Env_GetHome = Env_Get("HOME", "/home")
    END IF
END FUNCTION

' ============================================================================
' FUNCTION: Env_GetTemp
' Get temporary directory
' ============================================================================
FUNCTION Env_GetTemp$ ()
    IF Env_IsWindows THEN
        Env_GetTemp = Env_Get("TEMP", "C:\Temp")
    ELSE
        Env_GetTemp = Env_Get("TMPDIR", "/tmp")
    END IF
END FUNCTION
