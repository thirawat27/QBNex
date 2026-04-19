SUB PrintCommandLineBanner
    IF qbnexversionprinted = 0 THEN
        qbnexversionprinted = -1
        PRINT "QBNex Compiler V" + Version$
    END IF
END SUB

SUB ShowCommandLineHelp
    _DEST _CONSOLE
    PrintCommandLineBanner
    PRINT
    PRINT "Usage: qb <file> [switches]"
    PRINT
    PRINT "Commands:"
    PRINT "  -h, --help              Show help"
    PRINT "  -v, --version           Show compiler version"
    PRINT "  -i, --info, --about     Show project information"
    PRINT "  -g, --examples          Show common CLI examples"
    PRINT
    PRINT "Options:"
    PRINT "  <file>                  Source file to load"
    PRINT "  -c                      Compile the source file (default)"
    PRINT "  -o <output file>        Write output executable to <output file>"
    PRINT "  -x                      Compile and output the result to the"
    PRINT "                             console"
    PRINT "  -w                      Show warnings"
    PRINT "  -Werror, --warnings-as-errors"
    PRINT "                          Treat warnings as blocking diagnostics"
    PRINT "  -q                      Quiet mode (does not inhibit warnings or errors)"
    PRINT "  -m                      Do not colorize compiler output (monochrome mode)"
    PRINT "  -d, --verbose-errors    Legacy alias (detailed diagnostics are default)"
    PRINT "  -k, --compact-errors    Use compact diagnostics (hide extra detail notes)"
    PRINT "  -e                      Enable OPTION _EXPLICIT, making variable declaration"
    PRINT "                             mandatory (per-compilation; doesn't affect the"
    PRINT "                             source file or global settings)"
    PRINT "  -s[:switch=true/false]  View/edit compiler settings"
    PRINT "  -p                      Purge all pre-compiled content first"
    PRINT "  -z                      Generate C code without compiling to executable"
    PRINT
END SUB

SUB ShowProjectInfo
    _DEST _CONSOLE
    PRINT "QBNex Compiler " + Version$
    PRINT "Executable: qb"
    PRINT "Owner: thirawat27"
    PRINT "Repository: https://github.com/thirawat27/QBNex"
END SUB

SUB ShowCommandExamples
    _DEST _CONSOLE
    PRINT "Examples:"
    PRINT "  qb hello.bas"
    PRINT "  qb hello.bas -x"
    PRINT "  qb hello.bas -o hello.exe"
    PRINT "  qb broken.bas"
    PRINT "  qb broken.bas --compact-errors"
    PRINT "  qb --version"
    PRINT "  qb -s"
END SUB

SUB PurgePrecompiledContent
    IF os$ = "WIN" THEN
        CHDIR "internal\c"
        SHELL _HIDE "cmd /c purge_all_precompiled_content_win.bat"
        CHDIR "..\.."
    END IF

    IF os$ = "LNX" THEN
        CHDIR "./internal/c"
        IF INSTR(_OS$, "[MACOSX]") THEN
            SHELL _HIDE "./purge_all_precompiled_content_osx.command"
        ELSE
            SHELL _HIDE "./purge_all_precompiled_content_lnx.sh"
        END IF
        CHDIR "../.."
    END IF
END SUB

SUB ShowSettingsUsageAndExit (token$)
    PRINT "Invalid settings switch: "; token$
    PRINT
    PRINT "Valid switches:"
    PRINT "    -s:debuginfo=true/false     (Embed C++ debug info into .EXE)"
    PRINT "    -s:exewithsource=true/false (Save .EXE in the source folder)"
    SYSTEM 1
END SUB

SUB HandleSettingsSwitch (token$)
    settingsMode = -1
    _DEST _CONSOLE
    PrintCommandLineBanner

    SELECT CASE LCASE$(MID$(token$, 3))
    CASE ""
        PRINT "debuginfo     = ";
        IF compilerdebuginfo THEN PRINT "true" ELSE PRINT "false"
        PRINT "exewithsource = ";
        IF SaveExeWithSource THEN PRINT "true" ELSE PRINT "false"
        SYSTEM
    CASE ":exewithsource"
        PRINT "exewithsource = ";
        IF SaveExeWithSource THEN PRINT "true" ELSE PRINT "false"
        SYSTEM
    CASE ":exewithsource=true"
        WriteConfigSetting generalSettingsSection$, "SaveExeWithSource", "True"
        PRINT "exewithsource = true"
        SaveExeWithSource = -1
    CASE ":exewithsource=false"
        WriteConfigSetting generalSettingsSection$, "SaveExeWithSource", "False"
        PRINT "exewithsource = false"
        SaveExeWithSource = 0
    CASE ":debuginfo"
        PRINT "debuginfo = ";
        IF compilerdebuginfo THEN PRINT "true" ELSE PRINT "false"
        SYSTEM
    CASE ":debuginfo=true"
        PRINT "debuginfo = true"
        WriteConfigSetting generalSettingsSection$, "DebugInfo", "True" + DebugInfoIniWarning$
        compilerdebuginfo = 1
        Include_GDB_Debugging_Info = compilerdebuginfo
        PurgePrecompiledContent
    CASE ":debuginfo=false"
        PRINT "debuginfo = false"
        WriteConfigSetting generalSettingsSection$, "DebugInfo", "False" + DebugInfoIniWarning$
        compilerdebuginfo = 0
        Include_GDB_Debugging_Info = compilerdebuginfo
        PurgePrecompiledContent
    CASE ELSE
        ShowSettingsUsageAndExit token$
    END SELECT

    _DEST 0
END SUB

FUNCTION NormalizeCommandToken$ (token$)
    DIM normalized AS STRING

    normalized = LCASE$(token$)
    IF normalized = "-h" OR normalized = "/?" OR normalized = "--help" OR normalized = "/help" THEN normalized = "-?"
    IF normalized = "-v" OR normalized = "--version" OR normalized = "/version" THEN normalized = "-v"
    IF normalized = "-i" OR normalized = "--info" OR normalized = "--about" OR normalized = "/info" OR normalized = "/about" THEN normalized = "-i"
    IF normalized = "-g" OR normalized = "--examples" OR normalized = "--example" OR normalized = "/examples" OR normalized = "/example" THEN normalized = "-g"
    IF normalized = "--verbose-errors" OR normalized = "--detailed-errors" THEN normalized = "-d"
    IF normalized = "--compact-errors" OR normalized = "--compact-diagnostics" THEN normalized = "-k"
    IF normalized = "-werror" OR normalized = "--warnings-as-errors" THEN normalized = "-r"

    NormalizeCommandToken$ = normalized
END FUNCTION

FUNCTION ParseCMDLineArgs$ ()
    DIM PassedFileName$

    'Recall that COMMAND$ is a concatenation of argv[] elements, so we don't have
    'to worry about more than one space between things (unless they used quotes,
    'in which case they're simply asking for trouble).
    FOR i = 1 TO _COMMANDCOUNT
        token$ = COMMAND$(i)
        token$ = NormalizeCommandToken$(token$)

        SELECT CASE LCASE$(LEFT$(token$, 2))
        CASE "-?"
            ShowCommandLineHelp
            SYSTEM
        CASE "-v"
            _DEST _CONSOLE
            PRINT "QBNex Compiler " + Version$
            SYSTEM
        CASE "-i"
            ShowProjectInfo
            SYSTEM
        CASE "-g"
            ShowCommandExamples
            SYSTEM
        CASE "-c"
            cmdlineswitch = -1
        CASE "-o"
            IF LEN(COMMAND$(i + 1)) > 0 THEN outputfile_cmd$ = COMMAND$(i + 1): i = i + 1
            cmdlineswitch = -1
        CASE "-x"
            ConsoleMode = 1
            cmdlineswitch = -1
        CASE "-w"
            ShowWarnings = -1
            cmdlineswitch = -1
        CASE "-r"
            SetWarningsAsErrors -1
            cmdlineswitch = -1
        CASE "-q"
            QuietMode = -1
            cmdlineswitch = -1
        CASE "-m"
            MonochromeLoggingMode = -1
            cmdlineswitch = -1
        CASE "-d"
            SetVerboseMode -1
            cmdlineswitch = -1
        CASE "-k"
            SetVerboseMode 0
            cmdlineswitch = -1
        CASE "-e"
            optionexplicit_cmd = -1
            cmdlineswitch = -1
        CASE "-s"
            HandleSettingsSwitch token$
        CASE "-l"
            PRINT "The -l switch is no longer supported in the CLI compiler."
            SYSTEM 1
        CASE "-p"
            PurgePrecompiledContent
            cmdlineswitch = -1
        CASE "-z"
            No_C_Compile_Mode = 1
            ConsoleMode = 1
            cmdlineswitch = -1
        CASE ELSE
            IF LEFT$(token$, 1) = "-" OR LEFT$(token$, 1) = "/" THEN
                _DEST _CONSOLE
                PrintCommandLineBanner
                PRINT
                PRINT "Unknown switch: "; token$
                PRINT "Run 'qb --help' for usage."
                SYSTEM 1
            END IF
            IF PassedFileName$ = "" THEN PassedFileName$ = token$
        END SELECT
    NEXT i

    IF LEN(PassedFileName$) THEN
        ParseCMDLineArgs$ = PassedFileName$
    ELSE
        IF cmdlineswitch = 0 AND settingsMode = -1 THEN SYSTEM
    END IF
END FUNCTION
