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
    PRINT "       qb run <file> [switches] [-- program arguments]"
    PRINT "       qb fmt|lsp|dap [tool options]"
    TuiDefault
    PRINT
    HelpHead "Commands:"
    HelpRow "run <file>", "Compile, then run the program (same as --run)"
    HelpRow "-h, --help", "Show this help"
    HelpRow "-v, --version", "Show the compiler version (one line)"
    HelpRow "-i, --info, --about", "Show project information: folders, settings"
    HelpRow "", "   file, platform, license and links"
    HelpRow "-g, --examples", "Show common command lines with what each does"
    HelpRow "fmt [options] [files]", "Format source files (qb fmt --help)"
    HelpRow "lsp", "Language server on stdio (for editors)"
    HelpRow "dap", "Debug adapter on stdio (for editors)"
    PRINT
    HelpHead "Options:"
    HelpRow "<file>", "Source file to compile; without -o the executable"
    HelpRow "", "   is written next to it"
    HelpRow "-o, --output <file>", "Write output executable to <file>"
    HelpRow "--run", "Run the program after a successful build (qb then"
    HelpRow "", "   exits with the program's exit code)"
    HelpRow "-- <arguments>", "Arguments for the program (COMMAND$); also runs it"
    HelpRow "-w, --warnings", "Show warnings, and a warning count at the end"
    HelpRow "-Werror, --warnings-as-errors", ""
    HelpRow "", "Treat warnings as blocking diagnostics (exit code 1)"
    HelpRow "-q, --quiet", "Quiet mode: no banner or progress (warnings and"
    HelpRow "", "   errors are still shown)"
    HelpRow "-m, --no-color", "Do not colorize compiler output (monochrome mode;"
    HelpRow "", "   also when NO_COLOR is set)"
    HelpRow "-d, --verbose-errors", "Detailed diagnostics (the default) plus the"
    HelpRow "", "   source lines around each one"
    HelpRow "-k, --compact-errors", "Use compact diagnostics (hide extra detail notes)"
    HelpRow "-e, --explicit", "Enable OPTION _EXPLICIT, making variable declaration"
    HelpRow "", "   mandatory (per-compilation; doesn't affect the"
    HelpRow "", "   source file or global settings)"
    HelpRow "-s[:switch=true/false]", "View/edit compiler settings (-s lists them)"
    HelpRow "--emit-ir", "Write the program's QIR text instead of an executable"
    HelpRow "", "   (also when -o names a .qir file; for qb-web)"
    PRINT
    TuiColor 248: PRINT "One-letter switches can be run together: -wq is -w -q."
    TuiDefault
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

'qb was started with a source file in a console window of its own (Explorer's "Compile with
'QBNex" runs qb.exe "file.bas"), which closes when qb ends: the build runs as a child in
'this window, then the window waits for a key and qb ends with the child's exit code. The
'child shares this console, so the runner does not mark it as owning one; the variable is
'cleared too, so the child builds instead of doing this again.
SUB RunInOwnConsoleWindow
    DIM c$, k AS LONG, rc AS LONG
    c$ = ShellArg$(COMMAND$(0))
    FOR k = 1 TO _COMMANDCOUNT
        c$ = c$ + " " + ShellArg$(COMMAND$(k))
    NEXT
    ENVIRON "QBNEX_OWN_CONSOLE="
    'the build folder (#26) is free for the child, and relative names are resolved where
    'qb was started
    FOR k = 1 TO 255: CLOSE k: NEXT
    CHDIR _STARTDIR$
    'cmd /c takes away the first and the last quote of a line with several quoted names
    IF os$ = "WIN" THEN c$ = CHR$(34) + c$ + CHR$(34)
    rc = SHELL(c$)
    _DEST _CONSOLE
    PRINT
    TuiDefault: PRINT "Press any key to close this window . . .";
    SLEEP
    PRINT
    SYSTEM rc
END SUB

'target/qb-temp in the QBNex folder cannot be made or written (installed in a folder the
'user cannot write to): say which folder, not an internal error or a wait for ever
SUB ReportNoBuildFolder
    PRINT "Unable to create a build folder in 'target': " + QBNexRoot$ + "target" + pathsep$ + " cannot be written"
    PRINT "QBNex builds in its own folder: install it where you can write, or make that folder writable."
    SYSTEM 1
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
    InfoRow "Output:      ", "next to the source file (without -o)"
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
    ExampleRow "qb hello.bas", "compile; " + exe$ + " goes next to hello.bas"
    ExampleRow "qb run hello.bas", "compile, then run the program"
    ExampleRow "qb run hello.bas -- a b", "run it with COMMAND$ = " + CHR$(34) + "a b" + CHR$(34)
    ExampleRow "qb hello.bas -o " + exe$, "choose the output file"
    ExampleRow "qb hello.bas -w", "also show warnings"
    ExampleRow "qb hello.bas -Werror", "fail the build on any warning"
    ExampleRow "qb hello.bas -e", "every variable must be declared (DIM)"
    ExampleRow "qb hello.bas -q", "no banner or progress, only problems"
    ExampleRow "qb hello.bas -wq", "-w and -q together"
    ExampleRow "qb hello.bas -m", "no colors (setting NO_COLOR does the same)"
    ExampleRow "qb broken.bas -k", "short diagnostics"
    ExampleRow "qb broken.bas -d", "diagnostics with the lines around them"
    ExampleRow "qb hello.bas -o hello.qir", "write QIR text for qb-web (or --emit-ir)"
    ExampleRow "qb -s", "list the compiler settings"
    ExampleRow "qb fmt --write hello.bas", "format hello.bas in place"
    ExampleRow "qb --info", "show folders, settings file and license"
END SUB

SUB ShowSettingsUsageAndExit (token$)
    PRINT "Invalid settings switch: "; token$
    PRINT
    PRINT "Valid switches:"
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
        SettingRow "ignorewarnings", (IgnoreWarnings), "hide warnings, even with -w or -Werror"
        SettingRow "debuginfo", (compilerdebuginfo), "kept for old scripts; no effect"
        PRINT
        TuiColor 248: PRINT "Change one with -s:<name>=true or -s:<name>=false."
        TuiDefault
        SYSTEM
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
    IF normalized = "--warnings-as-errors" OR normalized = "--werror" THEN normalized = "-werror"
    IF normalized = "--run" THEN normalized = "-run"
    'long spellings people try first
    IF normalized = "--output" THEN normalized = "-o"
    IF normalized = "--quiet" THEN normalized = "-q"
    IF normalized = "--warnings" THEN normalized = "-w"
    IF normalized = "--explicit" THEN normalized = "-e"
    IF normalized = "--no-color" OR normalized = "--no-colour" OR normalized = "--monochrome" THEN normalized = "-m"
    IF normalized = "--settings" THEN normalized = "-s"
    IF normalized = "--ir" THEN normalized = "-ir"
    IF normalized = "--emit-ir" THEN normalized = "-irtext"

    NormalizeCommandToken$ = normalized
END FUNCTION

'the switches and settings that change this build, for the line under the banner, so each
'one shows that it was taken (empty: none). -q has no banner and -m shows by itself.
FUNCTION ActiveOptionsText$ (sep$)
    DIM r$
    IF ShowWarnings THEN r$ = r$ + sep$ + "-w show warnings"
    IF WarningsAsErrors THEN r$ = r$ + sep$ + "-Werror warnings stop the build"
    IF optionexplicit_cmd THEN r$ = r$ + sep$ + "-e OPTION _EXPLICIT"
    IF DiagnosticNeighborLines > 0 THEN r$ = r$ + sep$ + "-d errors with the lines around them"
    IF VerboseMode = 0 THEN r$ = r$ + sep$ + "-k compact errors"
    IF LEN(outputfile_cmd$) THEN r$ = r$ + sep$ + "-o " + outputfile_cmd$
    IF IrEmitText THEN r$ = r$ + sep$ + "--emit-ir QIR text"
    IF RunAfterBuild THEN r$ = r$ + sep$ + "--run run after the build"
    IF IgnoreWarnings THEN r$ = r$ + sep$ + "ignorewarnings=true (qb -s)"
    ActiveOptionsText$ = MID$(r$, LEN(sep$) + 1)
END FUNCTION

'one program argument for the shell: "..." on Windows (the quoting rules the tools'
'C runtime and Rust read), '...' elsewhere
FUNCTION ShellArg$ (a$)
    DIM r$, k AS LONG, bs AS LONG, c$
    IF os$ <> "WIN" THEN ShellArg$ = QuotedFilename$(a$): EXIT FUNCTION
    r$ = CHR$(34)
    FOR k = 1 TO LEN(a$)
        c$ = MID$(a$, k, 1)
        IF c$ = "\" THEN
            bs = bs + 1
        ELSEIF c$ = CHR$(34) THEN
            'backslashes before a quote are doubled, and the quote is escaped
            r$ = r$ + STRING$(bs * 2 + 1, "\") + c$: bs = 0
        ELSE
            r$ = r$ + STRING$(bs, "\") + c$: bs = 0
        END IF
    NEXT
    'backslashes at the end are doubled, so the closing quote stays a quote (C:\dir\)
    r$ = r$ + STRING$(bs * 2, "\") + CHR$(34)
    'SHELL runs cmd /c, which replaces %NAME% even between quotes: each % leaves the
    'quotes as "^%" (cmd drops the ^, and the program still reads one argument, as
    '"a"%"b" is a%b). Between two such % signs cmd sees a name with " and ^ in it, which
    'is no variable, so nothing is replaced.
    k = INSTR(r$, "%")
    DO WHILE k
        r$ = LEFT$(r$, k - 1) + CHR$(34) + "^%" + CHR$(34) + MID$(r$, k + 1)
        k = INSTR(k + 4, r$, "%")
    LOOP
    ShellArg$ = r$
END FUNCTION

'qb run: what qb itself prints (banner, diagnostics, "Running ...") goes to stderr, so that
'the program's output is alone on stdout (qb run app.bas > out.txt). onoff -1 switches it on;
'0 gives stdout back, for the program qb starts.
SUB StdErrForMessages (onoff AS LONG)
    IF onoff THEN
        _STDERR ON
    ELSE
        _STDERR OFF
    END IF
END SUB

'qb fmt / qb lsp / qb dap: run qb-fmt, qb-lsp or qb-dap with the arguments after the
'subcommand, and exit with its exit code. On the PATH, qb finds qb.exe before qb.cmd, so
'qb.exe has to do what qb.cmd and the Unix launcher do. The current directory is the
'QBNex folder here (VerifyInternalFolderOrExit): the tool is looked up next to qb, then
'in a cargo build of this checkout, then on the PATH.
SUB RunToolSubcommand (sc$)
    DIM tool$, exe$, c$, d$, k AS LONG, rc AS LONG
    tool$ = "qb-" + sc$
    IF os$ = "WIN" THEN tool$ = tool$ + ".exe"
    exe$ = ""
    FOR k = 1 TO 3
        SELECT CASE k
        CASE 1: d$ = ""
        CASE 2: d$ = "target" + pathsep$ + "release" + pathsep$
        CASE 3: d$ = "target" + pathsep$ + "debug" + pathsep$
        END SELECT
        IF _FILEEXISTS(d$ + tool$) THEN exe$ = QBNexRoot$ + d$ + tool$: EXIT FOR
    NEXT
    IF LEN(exe$) THEN c$ = ShellArg$(exe$) ELSE c$ = ShellArg$("qb-" + sc$)
    FOR k = 2 TO _COMMANDCOUNT
        c$ = c$ + " " + ShellArg$(COMMAND$(k))
    NEXT
    'the tools drop their first argument when it is the subcommand named here (qb.cmd
    'passes it on); it is not passed on from here
    ENVIRON "QBNEX_SUBCOMMAND="
    'the tool reads and writes files relative to the folder qb was started in
    CHDIR _STARTDIR$
    'cmd /c takes away the first and the last quote of a line with several quoted names:
    'a line that starts with two quotes is one the runtime passes as it is
    IF os$ = "WIN" THEN c$ = CHR$(34) + c$ + CHR$(34)
    rc = SHELL(c$)
    IF LEN(exe$) = 0 AND ((os$ = "WIN" AND rc = 9009) OR (os$ <> "WIN" AND rc = 127)) THEN
        PRINT "qb: qb-" + sc$ + " was not found. Build it with: cargo build --release -p qb-" + sc$
    END IF
    SYSTEM rc
END SUB

'how many one-character edits turn a$ into b$, letter case ignored ("did you mean")
FUNCTION EditDistance& (a$, b$)
    DIM x$, y$, i AS LONG, j AS LONG, m AS LONG, diag AS LONG, up AS LONG, c AS LONG
    x$ = LCASE$(a$): y$ = LCASE$(b$)
    m = LEN(y$)
    REDIM row(0 TO m) AS LONG
    FOR j = 0 TO m: row(j) = j: NEXT
    FOR i = 1 TO LEN(x$)
        diag = row(0): row(0) = i
        FOR j = 1 TO m
            up = row(j)
            c = diag: IF MID$(x$, i, 1) <> MID$(y$, j, 1) THEN c = diag + 1
            IF up + 1 < c THEN c = up + 1
            IF row(j - 1) + 1 < c THEN c = row(j - 1) + 1
            row(j) = c
            diag = up
        NEXT
    NEXT
    EditDistance& = row(m)
END FUNCTION

'the closest of the space-separated words$ to w$: at most two edits away, or a word that
'starts with w$ (--out for --output). "" when none is close.
FUNCTION ClosestWord$ (w$, words$)
    DIM p AS LONG, q AS LONG, c$, best$, bestd AS LONG, d AS LONG
    bestd = 3
    p = 1
    DO WHILE p <= LEN(words$)
        q = INSTR(p, words$, " ")
        IF q = 0 THEN q = LEN(words$) + 1
        c$ = MID$(words$, p, q - p)
        p = q + 1
        IF LEN(c$) THEN
            d = EditDistance&(w$, c$)
            IF LEN(w$) >= 4 AND LCASE$(LEFT$(c$, LEN(w$))) = LCASE$(w$) THEN d = 1
            IF d < bestd THEN bestd = d: best$ = c$
        END IF
    LOOP
    ClosestWord$ = best$
END FUNCTION

SUB ReportUnknownSwitch (raw$)
    DIM s$
    _DEST _CONSOLE
    PRINT "Unknown switch: "; raw$
    'one-letter switches are too short to guess from
    IF LEN(raw$) > 2 THEN
        s$ = ClosestWord$(raw$, "--help --version --info --examples --output --run --warnings -Werror --warnings-as-errors --quiet --no-color --verbose-errors --compact-errors --explicit --settings --emit-ir")
        IF LEN(s$) THEN PRINT "Did you mean "; s$; "?"
    END IF
    PRINT "Run 'qb --help' for usage."
    SYSTEM 1
END SUB

'the source file is not there: say so, and name the .bas file that was probably meant,
'or the ones that are in that folder. isFolder: f$ is a folder (qb sub), so its own
'.bas files are listed.
SUB ReportMissingSource (f$, isFolder AS LONG)
    DIM typed$, dir$, nm$, lst$, c$, listing$, e$, best$, names$, fh AS LONG, p AS LONG, q AS LONG, n AS LONG, rc AS LONG
    DIM d AS LONG, bestd AS LONG, allowed AS LONG
    _DEST _CONSOLE
    IF isFolder THEN
        PRINT f$ + " is a folder, not a source file"
        typed$ = f$
        IF RIGHT$(typed$, 1) <> "/" AND RIGHT$(typed$, 1) <> "\" THEN typed$ = typed$ + pathsep$
        nm$ = ""
    ELSE
        PRINT "Cannot find the source file: " + f$
        typed$ = getfilepath$(f$)
        nm$ = MID$(f$, LEN(typed$) + 1)
    END IF
    'a short name is close only one edit away: sub was taken for gui.bas
    allowed = 2
    IF LEN(RemoveFileExtension$(nm$)) <= 4 THEN allowed = 1
    bestd = allowed + 1
    dir$ = typed$
    IF IsAbsolutePath%(dir$) = 0 THEN dir$ = JoinPath$(_STARTDIR$, dir$)
    IF RIGHT$(dir$, 1) <> "/" AND RIGHT$(dir$, 1) <> "\" THEN dir$ = dir$ + pathsep$
    IF _DIREXISTS(dir$) = 0 THEN
        PRINT "The folder does not exist: " + typed$
        SYSTEM 1
    END IF
    'the shell lists the folder's .bas files (the runtime has no function that does)
    lst$ = tmpdir$ + "basfiles.txt"
    IF os$ = "WIN" THEN
        'SHELL keeps the quotes of the line as written (the runtime adds the pair cmd /c
        'takes away); ShellArg$ keeps a % in a folder name from being expanded
        c$ = "dir /b /a-d " + ShellArg$(dir$ + "*.bas") + " > " + ShellArg$(lst$) + " 2> nul"
    ELSE
        c$ = "(cd -- " + QuotedFilename$(dir$) + " 2>/dev/null && for f in *.bas *.BAS *.Bas; do [ -f " + CHR$(34) + "$f" + CHR$(34) + " ] && printf '%s\n' " + CHR$(34) + "$f" + CHR$(34) + "; done) > " + QuotedFilename$(lst$) + " 2>/dev/null"
    END IF
    rc = _SHELLHIDE(c$)
    IF _FILEEXISTS(lst$) THEN
        fh = FREEFILE
        OPEN lst$ FOR BINARY AS #fh
        IF LOF(fh) THEN listing$ = SPACE$(LOF(fh)): GET #fh, , listing$
        CLOSE #fh
        KILL lst$
    END IF
    p = 1
    DO WHILE p <= LEN(listing$)
        q = INSTR(p, listing$, CHR$(10))
        IF q = 0 THEN q = LEN(listing$) + 1
        e$ = MID$(listing$, p, q - p)
        p = q + 1
        IF RIGHT$(e$, 1) = CHR$(13) THEN e$ = LEFT$(e$, LEN(e$) - 1)
        IF LEN(e$) THEN
            n = n + 1
            IF n <= 6 THEN names$ = names$ + "  " + typed$ + e$ + CHR$(10)
            IF n = 7 THEN names$ = names$ + "  ..." + CHR$(10)
            'the closest name, with or without .bas (qb helo)
            IF LEN(nm$) THEN
                d = EditDistance&(nm$, e$)
                IF EditDistance&(RemoveFileExtension$(nm$), RemoveFileExtension$(e$)) < d THEN d = EditDistance&(RemoveFileExtension$(nm$), RemoveFileExtension$(e$))
                IF d < bestd THEN bestd = d: best$ = e$
            END IF
        END IF
    LOOP
    IF LEN(best$) THEN
        PRINT "Did you mean "; typed$ + best$; "?"
    ELSEIF n THEN
        IF isFolder THEN PRINT "Source files in it:" ELSE PRINT "Source files in that folder:"
        PRINT names$;
    ELSE
        PRINT "There is no .bas file in that folder."
    END IF
    SYSTEM 1
END SUB

'-w -q -m -d -k -e, alone or run together (-wq)
SUB ApplyLetterSwitch (c$)
    SELECT CASE c$
    CASE "w": ShowWarnings = -1
    CASE "q": QuietMode = -1
    CASE "m": MonochromeLoggingMode = -1
    CASE "d"
        'the default detail, plus the source lines around each diagnostic
        SetVerboseMode -1
        DiagnosticNeighborLines = 2
    CASE "k"
        SetVerboseMode 0
        DiagnosticNeighborLines = 0
    CASE "e": optionexplicit_cmd = -1
    END SELECT
END SUB

FUNCTION ParseCMDLineArgs$ ()
    DIM PassedFileName$, firstArg AS LONG, runSubcommand AS LONG, k AS LONG, termVar$

    'qb fmt / lsp / dap go to the tools; qb run <file> builds and runs; everything else
    'is for the compiler
    firstArg = 1
    IF _COMMANDCOUNT >= 1 THEN
        SELECT CASE LCASE$(COMMAND$(1))
        CASE "fmt", "lsp", "dap": RunToolSubcommand LCASE$(COMMAND$(1))
        CASE "run": runSubcommand = -1: RunAfterBuild = -1: firstArg = 2: StdErrForMessages -1
        END SELECT
    END IF

    'NO_COLOR (no-color.org): any non-empty value turns colors off, like -m
    IF LEN(ENVIRON$("NO_COLOR")) THEN MonochromeLoggingMode = -1
    'output going to a pipe, a file or a CI log gets no color escape sequences; qb run
    'writes its messages to stderr
    termVar$ = "QBNEX_STDOUT_TERMINAL"
    IF runSubcommand THEN termVar$ = "QBNEX_STDERR_TERMINAL"
    IF ENVIRON$(termVar$) <> "1" THEN MonochromeLoggingMode = -1
    '-m anywhere on the line, also after -h, -i, -g or -s, which print before the
    'loop below reaches it
    FOR i = 1 TO _COMMANDCOUNT
        IF COMMAND$(i) = "--" THEN EXIT FOR
        IF LCASE$(COMMAND$(i)) = "-m" THEN MonochromeLoggingMode = -1
    NEXT i

    FOR i = firstArg TO _COMMANDCOUNT
        'everything after -- is for the program, which then runs after the build
        IF COMMAND$(i) = "--" THEN
            FOR k = i + 1 TO _COMMANDCOUNT
                RunArgs$ = RunArgs$ + " " + ShellArg$(COMMAND$(k))
                IF INSTR(COMMAND$(k), " ") OR LEN(COMMAND$(k)) = 0 THEN
                    RunArgsShown$ = RunArgsShown$ + " " + CHR$(34) + COMMAND$(k) + CHR$(34)
                ELSE
                    RunArgsShown$ = RunArgsShown$ + " " + COMMAND$(k)
                END IF
            NEXT
            RunAfterBuild = -1
            cmdlineswitch = -1
            EXIT FOR
        END IF
        token$ = COMMAND$(i)
        token$ = NormalizeCommandToken$(token$)

        'A switch is its letter alone: -emit-ir was taken for -e, and -O2 took the next
        'word (the source file) as the name after -o. Longer ones: -ir, -irtext, -s:name.
        sw$ = LCASE$(LEFT$(token$, 2))
        IF LEN(token$) > 2 AND LEFT$(token$, 1) = "-" THEN
            IF token$ <> "-ir" AND token$ <> "-irtext" AND sw$ <> "-s" THEN sw$ = ""
            IF token$ = "-werror" OR token$ = "-run" THEN sw$ = token$
        END IF
        SELECT CASE sw$
        CASE "-run"
            RunAfterBuild = -1
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
            'it always prints to the console and nothing is precompiled): not in the help,
            'but accepted so old build scripts keep working
            cmdlineswitch = -1
        CASE "-o"
            IF LEN(COMMAND$(i + 1)) = 0 THEN
                PRINT "Missing file name after -o"
                PRINT "Run 'qb --help' for usage."
                SYSTEM 1
            END IF
            'qb x.bas -o -q wrote -q.exe and dropped -q; a name that starts with - is
            'written .\-name (./-name)
            IF LEFT$(COMMAND$(i + 1), 1) = "-" THEN
                PRINT "Missing file name after -o: " + COMMAND$(i + 1) + " is a switch"
                PRINT "For a file whose name starts with -, write ." + pathsep$ + COMMAND$(i + 1)
                SYSTEM 1
            END IF
            outputfile_cmd$ = COMMAND$(i + 1): i = i + 1
            cmdlineswitch = -1
        CASE "-w", "-q", "-m", "-d", "-k", "-e"
            ApplyLetterSwitch MID$(sw$, 2, 1)
            cmdlineswitch = -1
        CASE "-werror"
            SetWarningsAsErrors -1
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
                'one-letter switches run together: -wq is -w -q
                letters = (LEFT$(token$, 1) = "-" AND LEN(token$) > 2)
                FOR k = 2 TO LEN(token$)
                    IF INSTR("wqmdke", MID$(token$, k, 1)) = 0 THEN letters = 0
                NEXT
                IF letters = 0 THEN ReportUnknownSwitch rawtoken$
                FOR k = 2 TO LEN(token$)
                    ApplyLetterSwitch MID$(token$, k, 1)
                NEXT
                cmdlineswitch = -1
            ELSEIF PassedFileName$ = "" THEN
                PassedFileName$ = rawtoken$
            ELSE
                PRINT "Only one source file can be compiled: "; PassedFileName$; " and "; rawtoken$
                PRINT "To pass arguments to the program, put them after --:"
                PRINT "  qb run "; PassedFileName$; " -- "; rawtoken$
                SYSTEM 1
            END IF
        END SELECT
    NEXT i

    'the ignorewarnings setting hides warnings, also the ones -Werror would block on
    IF IgnoreWarnings THEN SetWarningsAsErrors 0

    'qb run / --run / --: qb's own messages go to stderr (StdErrForMessages), so no colors
    'when stderr is no terminal either
    IF RunAfterBuild THEN
        IF ENVIRON$("QBNEX_STDERR_TERMINAL") <> "1" THEN MonochromeLoggingMode = -1
        StdErrForMessages -1
    END IF

    IF LEN(PassedFileName$) THEN
        ParseCMDLineArgs$ = PassedFileName$
    ELSEIF runSubcommand OR LEN(RunArgs$) THEN
        PRINT "Name the source file to build and run, for example:  qb run hello.bas"
        SYSTEM 1
    ELSE
        IF cmdlineswitch = 0 AND settingsMode = -1 THEN SYSTEM
    END IF
END FUNCTION
