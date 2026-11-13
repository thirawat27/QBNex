SUB PrintCommandLineBanner
    IF qbnexversionprinted = 0 THEN
        qbnexversionprinted = -1
        TuiTitle "QBNex Compiler "
        TuiColor 114: PRINT "V" + Version$
        TuiDefault
    END IF
END SUB

SUB HelpHead (h$)
    TuiColor 114: PRINT h$
    TuiDefault
END SUB

SUB HelpRow (sw$, desc$)
    'a switch in a 24-column field, then its description
    TuiColor 111: PRINT "  " + sw$;
    IF LEN(desc$) THEN
        TuiDefault: PRINT SPACE$(24 - LEN(sw$)) + desc$
    ELSE
        PRINT
    END IF
    TuiDefault
END SUB

SUB ShowCommandLineHelp
    _DEST _CONSOLE
    PrintCommandLineBanner
    PRINT
    TuiColor 114: PRINT "Usage: ";
    TuiColor 152: PRINT "qb <file> [switches]"
    TuiDefault
    PRINT
    HelpHead "Commands:"
    HelpRow "-h, --help", "Show help"
    HelpRow "-v, --version", "Show compiler version"
    HelpRow "-i, --info, --about", "Show project information"
    HelpRow "-g, --examples", "Show common CLI examples"
    PRINT
    HelpHead "Options:"
    HelpRow "<file>", "Source file to load"
    HelpRow "-c", "Compile the source file (default)"
    HelpRow "-o <output file>", "Write output executable to <output file>"
    HelpRow "-x", "Compile and output the result to the"
    HelpRow "", "   console"
    HelpRow "-w", "Show warnings"
    HelpRow "-Werror, --warnings-as-errors", ""
    HelpRow "", "Treat warnings as blocking diagnostics"
    HelpRow "-q", "Quiet mode (does not inhibit warnings or errors)"
    HelpRow "-m", "Do not colorize compiler output (monochrome mode;"
    HelpRow "", "   also when NO_COLOR is set)"
    HelpRow "-d, --verbose-errors", "Legacy alias (detailed diagnostics are default)"
    HelpRow "-k, --compact-errors", "Use compact diagnostics (hide extra detail notes)"
    HelpRow "-e", "Enable OPTION _EXPLICIT, making variable declaration"
    HelpRow "", "   mandatory (per-compilation; doesn't affect the"
    HelpRow "", "   source file or global settings)"
    HelpRow "-s[:switch=true/false]", "View/edit compiler settings"
    HelpRow "-p, -z", "Accepted for old scripts; they change nothing"
    HelpRow "--emit-ir", "Write the program's QIR text instead of an executable"
    HelpRow "", "   (also when -o names a .qir file; for qb-web)"
    PRINT
END SUB

SUB InfoRow (label$, value$)
    TuiColor 248: PRINT label$;
    TuiDefault: PRINT value$
    TuiDefault
END SUB

SUB ShowProjectInfo
    _DEST _CONSOLE
    TuiTitle "QBNex Compiler "
    TuiColor 114: PRINT Version$
    TuiDefault
    InfoRow "Executable: ", "qb"
    InfoRow "Owner: ", "thirawat27"
    InfoRow "Repository: ", "https://github.com/thirawat27/QBNex"
END SUB

SUB ShowCommandExamples
    _DEST _CONSOLE
    HelpHead "Examples:"
    TuiColor 111
    PRINT "  qb hello.bas"
    PRINT "  qb hello.bas -x"
    PRINT "  qb hello.bas -o hello.exe"
    PRINT "  qb broken.bas"
    PRINT "  qb broken.bas --compact-errors"
    PRINT "  qb --version"
    PRINT "  qb -s"
    TuiDefault
END SUB

SUB ShowSettingsUsageAndExit (token$)
    PRINT "Invalid settings switch: "; token$
    PRINT
    PRINT "Valid switches:"
    PRINT "    -s:debuginfo=true/false     (Accepted for old scripts; no effect)"
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
    CASE ":debuginfo=false"
        PRINT "debuginfo = false"
        WriteConfigSetting generalSettingsSection$, "DebugInfo", "False" + DebugInfoIniWarning$
        compilerdebuginfo = 0
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
    IF normalized = "--ir" THEN normalized = "-ir"
    IF normalized = "--emit-ir" THEN normalized = "-irtext"

    NormalizeCommandToken$ = normalized
END FUNCTION

FUNCTION ParseCMDLineArgs$ ()
    DIM PassedFileName$

    'NO_COLOR (no-color.org): any non-empty value turns colors off, like -m
    IF LEN(ENVIRON$("NO_COLOR")) THEN MonochromeLoggingMode = -1

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
            TuiTitle "QBNex Compiler "
            TuiColor 114: PRINT Version$
            TuiDefault
            SYSTEM
        CASE "-i"
            IF LCASE$(token$) = "-ir" THEN
                cmdlineswitch = -1 'accepted for compatibility: QIR is the only output
            ELSEIF LCASE$(token$) = "-irtext" THEN
                IrEmitText = -1 'write the QIR text (qb-web) instead of an executable
                cmdlineswitch = -1
            ELSE
                ShowProjectInfo
                SYSTEM
            END IF
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
            'nothing is precompiled any more; the switch is kept for old scripts
            cmdlineswitch = -1
        CASE "-z"
            ConsoleMode = 1
            cmdlineswitch = -1
        CASE ELSE
            'a switch starts with "-" (or "/" on Windows); on macOS/Linux "/..." is an absolute path,
            'and an existing file is always a path. Keep the original case of file names.
            rawtoken$ = COMMAND$(i)
            isswitch = (LEFT$(token$, 1) = "-")
            IF LEFT$(token$, 1) = "/" AND os$ = "WIN" THEN isswitch = -1
            IF _FILEEXISTS(rawtoken$) THEN isswitch = 0
            IF isswitch THEN
                _DEST _CONSOLE
                PrintCommandLineBanner
                PRINT
                PRINT "Unknown switch: "; token$
                PRINT "Run 'qb --help' for usage."
                SYSTEM 1
            END IF
            IF PassedFileName$ = "" THEN PassedFileName$ = rawtoken$
        END SELECT
    NEXT i

    IF LEN(PassedFileName$) THEN
        ParseCMDLineArgs$ = PassedFileName$
    ELSE
        IF cmdlineswitch = 0 AND settingsMode = -1 THEN SYSTEM
    END IF
END FUNCTION
