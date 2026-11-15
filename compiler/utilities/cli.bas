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
    HelpRow "-h, --help", "Show this help"
    HelpRow "-v, --version", "Show the compiler version (one line)"
    HelpRow "-i, --info, --about", "Show project information: folders, settings"
    HelpRow "", "   file, platform, license and links"
    HelpRow "-g, --examples", "Show common command lines with what each does"
    PRINT
    HelpHead "Options:"
    HelpRow "<file>", "Source file to compile"
    HelpRow "-o <output file>", "Write output executable to <output file>"
    HelpRow "--exe-with-source", "Without -o: write the executable next to the"
    HelpRow "", "   source file (this build only)"
    HelpRow "-w", "Show warnings, and a warning count at the end"
    HelpRow "-Werror, --warnings-as-errors", ""
    HelpRow "", "Treat warnings as blocking diagnostics (exit code 1)"
    HelpRow "-q", "Quiet mode: no banner or progress (warnings and"
    HelpRow "", "   errors are still shown)"
    HelpRow "-m", "Do not colorize compiler output (monochrome mode;"
    HelpRow "", "   also when NO_COLOR is set)"
    HelpRow "-d, --verbose-errors", "Detailed diagnostics (the default) plus the"
    HelpRow "", "   source lines around each one"
    HelpRow "-k, --compact-errors", "Use compact diagnostics (hide extra detail notes)"
    HelpRow "-e", "Enable OPTION _EXPLICIT, making variable declaration"
    HelpRow "", "   mandatory (per-compilation; doesn't affect the"
    HelpRow "", "   source file or global settings)"
    HelpRow "-s[:switch=true/false]", "View/edit compiler settings (-s lists them)"
    HelpRow "--emit-ir", "Write the program's QIR text instead of an executable"
    HelpRow "", "   (also when -o names a .qir file; for qb-web)"
    PRINT
END SUB

SUB ShowDoubleClickHelp
    'qb was started by a double-click: say what it is and how to use it, and keep
    'the window open until a key is pressed
    _DEST _CONSOLE
    PRINT
    PrintCommandLineBanner
    PRINT
    TuiDefault: PRINT "QBNex is a command-line compiler: it runs in a terminal, not on its own."
    PRINT
    TuiColor 248: PRINT "  1. ";
    TuiDefault: PRINT "Open a terminal (Windows Terminal, PowerShell or Command Prompt)."
    TuiColor 248: PRINT "  2. ";
    TuiDefault: PRINT "Go to the folder of your program and run:  ";
    TuiColor 111: PRINT "qb myprogram.bas"
    TuiColor 248: PRINT "  3. ";
    TuiDefault: PRINT "Run ";
    TuiColor 111: PRINT "qb -h";
    TuiDefault: PRINT " to see every option."
    PRINT
    TuiColor 248: PRINT "To install QBNex and put qb on your PATH, run the qbnex-setup installer."
    PRINT
    TuiDefault: PRINT "Press any key to close this window . . .";
    SLEEP
    PRINT
END SUB

SUB InfoRow (label$, value$)
    TuiColor 248: PRINT label$;
    TuiDefault: PRINT value$
    TuiDefault
END SUB

FUNCTION NativePath$ (p$)
    'a path as the system writes it: backslashes on Windows
    r$ = p$
    IF os$ = "WIN" THEN
        FOR k = 1 TO LEN(r$)
            IF MID$(r$, k, 1) = "/" THEN MID$(r$, k, 1) = "\"
        NEXT
    END IF
    NativePath$ = r$
END FUNCTION

FUNCTION SettingsFilePath$
    SettingsFilePath$ = NativePath$(QBNexRoot$ + ConfigFile$)
END FUNCTION

SUB ShowProjectInfo
    _DEST _CONSOLE
    TuiTitle "QBNex Compiler "
    TuiColor 114: PRINT Version$
    TuiDefault
    TuiColor 248: PRINT "A QBasic/QB64-compatible BASIC compiler: BASIC -> QIR -> executable"
    TuiDefault
    PRINT
    platform$ = "Linux"
    IF os$ = "WIN" THEN platform$ = "Windows"
    IF MacOSX THEN platform$ = "macOS"
    InfoRow "Executable:  ", "qb (" + NativePath$(COMMAND$(0)) + ")"
    InfoRow "Home:        ", NativePath$(QBNexRoot$) + "  (holds crates and compiler)"
    IF SaveExeWithSource THEN
        InfoRow "Output:      ", "next to the source file (without -o; -s:exewithsource)"
    ELSE
        InfoRow "Output:      ", "the Home folder (without -o)"
    END IF
    InfoRow "Settings:    ", SettingsFilePath$ + "  (qb -s)"
    InfoRow "Platform:    ", platform$ + " " + LTRIM$(STR$(OS_BITS)) + "-bit"
    InfoRow "License:     ", "GPL-3.0 with the QBNex Compiled Program Exception"
    InfoRow "Owner:       ", "thirawat27"
    InfoRow "Repository:  ", "https://github.com/thirawat27/QBNex"
    InfoRow "Manual:      ", "https://github.com/thirawat27/QBNex/tree/main/docs/manual"
END SUB

SUB ExampleRow (cmd$, desc$)
    'a command line in a 36-column field, then what it does
    TuiColor 111: PRINT "  " + cmd$;
    TuiColor 248: PRINT SPACE$(36 - LEN(cmd$)) + desc$
    TuiDefault
END SUB

SUB ShowCommandExamples
    _DEST _CONSOLE
    IF os$ = "WIN" THEN exe$ = "hello.exe" ELSE exe$ = "hello"
    HelpHead "Examples:"
    ExampleRow "qb hello.bas", "compile; the executable goes to the QBNex folder"
    ExampleRow "qb hello.bas -o " + exe$, "choose the output file"
    ExampleRow "qb hello.bas --exe-with-source", "write the executable next to hello.bas"
    ExampleRow "qb hello.bas -w", "also show warnings"
    ExampleRow "qb hello.bas -Werror", "fail the build on any warning"
    ExampleRow "qb hello.bas -e", "every variable must be declared (DIM)"
    ExampleRow "qb hello.bas -q", "no banner or progress, only problems"
    ExampleRow "qb hello.bas -m", "no colors (setting NO_COLOR does the same)"
    ExampleRow "qb broken.bas -k", "short diagnostics"
    ExampleRow "qb broken.bas -d", "diagnostics with the lines around them"
    ExampleRow "qb hello.bas -o hello.qir", "write QIR text for qb-web (or --emit-ir)"
    ExampleRow "qb -s", "list the compiler settings"
    ExampleRow "qb -s:exewithsource=true", "always write executables next to the source"
    ExampleRow "qb --info", "show folders, settings file and license"
END SUB

SUB ShowSettingsUsageAndExit (token$)
    PRINT "Invalid settings switch: "; token$
    PRINT
    PRINT "Valid switches:"
    PRINT "    -s:exewithsource=true/false  (Save .EXE in the source folder)"
    PRINT "    -s:ignorewarnings=true/false (Hide warnings, even with -w or -Werror)"
    PRINT "    -s:debuginfo=true/false      (Accepted for old scripts; no effect)"
    SYSTEM 1
END SUB

SUB SettingRow (name$, value AS LONG, note$)
    TuiColor 111: PRINT "  " + name$ + SPACE$(15 - LEN(name$));
    TuiDefault: PRINT "= ";
    IF value THEN PRINT "true "; ELSE PRINT "false";
    TuiColor 248: PRINT "   " + note$
    TuiDefault
END SUB

SUB HandleSettingsSwitch (token$)
    _DEST _CONSOLE
    PrintCommandLineBanner

    SELECT CASE LCASE$(MID$(token$, 3))
    CASE ""
        TuiColor 248: PRINT "Settings file: ";
        TuiDefault: PRINT SettingsFilePath$
        SettingRow "exewithsource", (SaveExeWithSource), "write the executable next to the source (without -o)"
        SettingRow "ignorewarnings", (IgnoreWarnings), "hide warnings, even with -w or -Werror"
        SettingRow "debuginfo", (compilerdebuginfo), "kept for old scripts; no effect"
        PRINT
        TuiColor 248: PRINT "Change one with -s:<name>=true or -s:<name>=false."
        TuiDefault
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
    CASE ":ignorewarnings"
        PRINT "ignorewarnings = ";
        IF IgnoreWarnings THEN PRINT "true" ELSE PRINT "false"
        SYSTEM
    CASE ":ignorewarnings=true"
        WriteConfigSetting generalSettingsSection$, "IgnoreWarnings", "True"
        PRINT "ignorewarnings = true"
        IgnoreWarnings = -1
    CASE ":ignorewarnings=false"
        WriteConfigSetting generalSettingsSection$, "IgnoreWarnings", "False"
        PRINT "ignorewarnings = false"
        IgnoreWarnings = 0
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
    IF normalized = "--exe-with-source" THEN normalized = "-exewithsource"

    NormalizeCommandToken$ = normalized
END FUNCTION

FUNCTION ParseCMDLineArgs$ ()
    DIM PassedFileName$

    'NO_COLOR (no-color.org): any non-empty value turns colors off, like -m
    IF LEN(ENVIRON$("NO_COLOR")) THEN MonochromeLoggingMode = -1
    'output going to a pipe, a file or a CI log gets no color escape sequences
    IF ENVIRON$("QBNEX_STDOUT_TERMINAL") <> "1" THEN MonochromeLoggingMode = -1
    '-m anywhere on the line, also after -h, -i, -g or -s, which print before the
    'loop below reaches it
    FOR i = 1 TO _COMMANDCOUNT
        IF LCASE$(COMMAND$(i)) = "-m" THEN MonochromeLoggingMode = -1
    NEXT i

    'Recall that COMMAND$ is a concatenation of argv[] elements, so we don't have
    'to worry about more than one space between things (unless they used quotes,
    'in which case they're simply asking for trouble).
    FOR i = 1 TO _COMMANDCOUNT
        token$ = COMMAND$(i)
        token$ = NormalizeCommandToken$(token$)

        'A switch is its letter alone: -emit-ir was taken for -e, and -O2 took the next
        'word (the source file) as the name after -o. Longer ones: -ir, -irtext, -s:name.
        sw$ = LCASE$(LEFT$(token$, 2))
        IF LEN(token$) > 2 AND LEFT$(token$, 1) = "-" THEN
            IF token$ <> "-ir" AND token$ <> "-irtext" AND sw$ <> "-s" THEN sw$ = ""
            IF token$ = "-exewithsource" THEN sw$ = token$
        END IF
        SELECT CASE sw$
        CASE "-exewithsource"
            'this build only (-s:exewithsource=true is the setting): the executable is
            'written next to the source file, not into the compiler's folder
            SaveExeWithSource = -1
            cmdlineswitch = -1
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
                cmdlineswitch = -1 'not in the help; accepted for old scripts: QIR is the only output
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
        CASE "-c", "-x", "-p", "-z"
            'QB64 switches that change nothing here (compiling is what qb always does,
            'it always prints to the console, nothing is precompiled and there is no C++
            'step): not in the help, but accepted so old build scripts keep working
            cmdlineswitch = -1
        CASE "-o"
            IF LEN(COMMAND$(i + 1)) = 0 THEN
                PRINT "Missing file name after -o"
                PRINT "Run 'qb --help' for usage."
                SYSTEM 1
            END IF
            outputfile_cmd$ = COMMAND$(i + 1): i = i + 1
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
            'the default detail, plus the source lines around each diagnostic
            SetVerboseMode -1
            DiagnosticNeighborLines = 2
            cmdlineswitch = -1
        CASE "-k"
            SetVerboseMode 0
            DiagnosticNeighborLines = 0
            cmdlineswitch = -1
        CASE "-e"
            optionexplicit_cmd = -1
            cmdlineswitch = -1
        CASE "-s"
            HandleSettingsSwitch token$
            'set here: the one in HandleSettingsSwitch is that SUB's own variable, so
            '-s:name=value went on to the "no source file" help and exit code 1
            settingsMode = -1
        CASE "-l"
            PRINT "The -l switch is no longer supported in the CLI compiler."
            SYSTEM 1
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
            IF PassedFileName$ = "" THEN
                PassedFileName$ = rawtoken$
            ELSE
                PRINT "Only one source file can be compiled: "; PassedFileName$; " and "; rawtoken$
                PRINT "Run 'qb --help' for usage."
                SYSTEM 1
            END IF
        END SELECT
    NEXT i

    IF LEN(PassedFileName$) THEN
        ParseCMDLineArgs$ = PassedFileName$
    ELSE
        IF cmdlineswitch = 0 AND settingsMode = -1 THEN SYSTEM
    END IF
END FUNCTION
