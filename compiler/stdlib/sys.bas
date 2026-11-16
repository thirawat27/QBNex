'===============================================================================
' QBNex Standard Library - sys ('$IMPORT:'sys')
'===============================================================================
' The computer and the command line, like Python's sys and os.environ.
'
'   Sys_Platform       "windows", "linux" or "macos"
'   Sys_Env("HOME")    an environment variable; Sys_Env(name$, "default")
'   Sys_HasEnv(name$)  Sys_Home  Sys_Is64Bit
'   Sys_ArgCount       arguments after the program name
'   Sys_Arg(1)         argument 1; Sys_Arg(0) is the program
'===============================================================================

' "windows", "linux", "macos" (or "unknown").
FUNCTION Sys_Platform$ ()
    'macOS first: its _OS$ names LINUX too ("[MACOSX][64BIT][LINUX]")
    IF INSTR(_OS$, "WIN") THEN
        Sys_Platform$ = "windows"
    ELSEIF INSTR(_OS$, "MAC") THEN
        Sys_Platform$ = "macos"
    ELSEIF INSTR(_OS$, "LINUX") THEN
        Sys_Platform$ = "linux"
    ELSE
        Sys_Platform$ = "unknown"
    END IF
END FUNCTION

' -1 on a 64-bit program.
FUNCTION Sys_Is64Bit& ()
    Sys_Is64Bit& = (INSTR(_OS$, "64BIT") > 0)
END FUNCTION

' Value of environment variable varName, or fallback when it is not set.
FUNCTION Sys_Env$ (varName AS STRING, OPTIONAL fallback AS STRING = "")
    DIM v AS STRING
    v = ENVIRON$(varName)
    IF LEN(v) THEN Sys_Env$ = v ELSE Sys_Env$ = fallback
END FUNCTION

' -1 when environment variable varName is set.
FUNCTION Sys_HasEnv& (varName AS STRING)
    Sys_HasEnv& = (LEN(ENVIRON$(varName)) > 0)
END FUNCTION

' The user's home folder.
FUNCTION Sys_Home$ ()
    IF INSTR(_OS$, "WIN") THEN Sys_Home$ = Sys_Env$("USERPROFILE", "C:\") ELSE Sys_Home$ = Sys_Env$("HOME", "/")
END FUNCTION

' Number of command-line arguments (without the program name).
FUNCTION Sys_ArgCount& ()
    Sys_ArgCount& = _COMMANDCOUNT
END FUNCTION

' Argument number index (0 = the program), or fallback when there is none.
FUNCTION Sys_Arg$ (index AS LONG, OPTIONAL fallback AS STRING = "")
    IF index < 0 OR index > _COMMANDCOUNT THEN Sys_Arg$ = fallback ELSE Sys_Arg$ = COMMAND$(index)
END FUNCTION
