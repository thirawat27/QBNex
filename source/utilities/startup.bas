SUB InitCompilerServices
    InitErrorHandler
    SetVerboseMode -1
    SetMaxErrors 100
END SUB

SUB VerifyInternalFolderOrExit
    IF _DIREXISTS("internal") THEN EXIT SUB

    _SCREENSHOW
    PRINT "QBNex cannot locate the 'internal' folder"
    PRINT
    PRINT "Check that QBNex has been extracted properly."
    PRINT "For macOS, enter './qbnex' in Terminal."
    PRINT "For Linux, enter './qbnex' in the console."
    DO
        _LIMIT 1
    LOOP UNTIL INKEY$ <> ""
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
