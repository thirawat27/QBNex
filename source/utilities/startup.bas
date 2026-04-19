SUB InitCompilerServices
    QBNex_uptime! = TIMER

    InitErrorHandler
    SetVerboseMode -1
    SetMaxErrors 100
END SUB

SUB InitDebugLog
    debugPath$ = _CWD$
    IF _FILEEXISTS(debugPath$ + "/qbnex.log") THEN
        KILL debugPath$ + "/qbnex.log"
    END IF
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
    os$ = "WIN"
    IF INSTR(_OS$, "[LINUX]") THEN os$ = "LNX"

    MacOSX = 0
    IF INSTR(_OS$, "[MACOSX]") THEN MacOSX = 1

    inline_DATA = 0
    IF MacOSX THEN inline_DATA = 1

    OS_BITS = 64
    IF INSTR(_OS$, "[32BIT]") THEN OS_BITS = 32

    IF OS_BITS = 32 THEN WindowTitle = "QBNex x32" ELSE WindowTitle = "QBNex x64"
    _TITLE WindowTitle
END SUB
