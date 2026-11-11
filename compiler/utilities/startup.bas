SUB InitCompilerServices
    InitErrorHandler
    SetVerboseMode -1
    SetMaxErrors 100
END SUB

'The QBNex root: the folder that holds crates\ (the QIR runner) and compiler\ (stdlib,
'support files). VerifyInternalFolderOrExit makes it the current directory, so the
'runner and target\qb-temp are found relative to it; source paths given on the command
'line are resolved against _STARTDIR$.
FUNCTION QBNexRoot$
    DIM r AS STRING
    r = _CWD$
    IF RIGHT$(r, 1) <> "\" AND RIGHT$(r, 1) <> "/" THEN r = r + pathsep$
    QBNexRoot$ = r
END FUNCTION

SUB VerifyInternalFolderOrExit
    DIM compilerPath AS STRING
    DIM compilerDir AS STRING
    DIM separatorPos AS LONG

    'the compiler's own folder first (a crates\ folder in the current directory may belong
    'to another project), then the current directory (a compiler built outside the tree)
    compilerPath = COMMAND$(0)
    separatorPos = _INSTRREV(compilerPath, "\")
    IF separatorPos = 0 THEN separatorPos = _INSTRREV(compilerPath, "/")
    IF separatorPos > 0 THEN
        compilerDir = LEFT$(compilerPath, separatorPos - 1)
        IF LEN(compilerDir) THEN
            IF _DIREXISTS(compilerDir + "/crates") THEN CHDIR compilerDir: EXIT SUB
        END IF
    END IF
    IF _DIREXISTS("crates") THEN EXIT SUB

    'No window and no key wait: this also runs from scripts and CI.
    PRINT "QBNex cannot locate the 'crates' folder"
    PRINT "Check that QBNex has been extracted properly."
    SYSTEM 1
END SUB

SUB InitPlatformDefaults
    MacOSX = 0
    IF INSTR(_OS$, "[MACOSX]") THEN MacOSX = 1

    os$ = "WIN"
    IF INSTR(_OS$, "[LINUX]") OR MacOSX THEN os$ = "LNX"

    inline_DATA = 0
    IF MacOSX THEN inline_DATA = 1

    OS_BITS = 64
    IF INSTR(_OS$, "[32BIT]") THEN OS_BITS = 32

    IF OS_BITS = 32 THEN WindowTitle = "QBNex x32" ELSE WindowTitle = "QBNex x64"
    _TITLE WindowTitle
END SUB
