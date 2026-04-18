'===============================================================================
' QBNex State Management Compatibility Module
'===============================================================================
' Stage0-compatible compiler state container. Keeps the public API intact while
' avoiding unsupported array members inside TYPE declarations during bootstrap.
'===============================================================================

CONST SCOPE_GLOBAL = 0
CONST SCOPE_MODULE = 1
CONST SCOPE_FUNCTION = 2
CONST SCOPE_SUB = 3
CONST SCOPE_TYPE = 4
CONST SCOPE_BLOCK = 5

CONST MAX_SNAPSHOTS = 10

TYPE CompilerGlobalState
    isCompiling AS _BYTE
    hasErrors AS _BYTE
    hasWarnings AS _BYTE
    currentPass AS INTEGER
    noprefixDesired AS _BYTE
    noprefixSet AS _BYTE
    vwatchDesired AS _BYTE
    vwatchOn AS _BYTE
    optionExplicitDesired AS _BYTE
    optionExplicitSet AS _BYTE
    optionExplicitArrayDesired AS _BYTE
    optionExplicitArraySet AS _BYTE
    startTime AS SINGLE
    endTime AS SINGLE
    linesProcessed AS LONG
    linesCompiled AS LONG
    symbolsDefined AS LONG
    symbolsResolved AS LONG
    errorsReported AS LONG
    warningsReported AS LONG
    optimizeLevel AS INTEGER
    debugMode AS _BYTE
    consoleMode AS _BYTE
    quietMode AS _BYTE
    showWarnings AS _BYTE
END TYPE

TYPE SymbolTableState
    hashListNext AS LONG
    hashListFreeLast AS LONG
    hashTableSize AS LONG
    idCount AS LONG
    idCapacity AS LONG
    currentScopeLevel AS INTEGER
    scopeStackTop AS INTEGER
    udtCount AS INTEGER
    udtCapacity AS INTEGER
    arrayCount AS LONG
    arrayCapacity AS LONG
END TYPE

TYPE ParserStateContainer
    currentLine AS LONG
    currentColumn AS INTEGER
    currentTokenPos AS LONG
    sourceFile AS STRING * 260
    sourceLine AS STRING * 512
    lineLength AS INTEGER
    tokenCount AS INTEGER
    tokenIndex AS INTEGER
    inComment AS _BYTE
    inString AS _BYTE
    inDirective AS _BYTE
    expectEOL AS _BYTE
    controlLevel AS INTEGER
    ifLevel AS INTEGER
    loopLevel AS INTEGER
    selectLevel AS INTEGER
END TYPE

TYPE CodeGenStateContainer
    outputFile AS INTEGER
    tempFile AS INTEGER
    indentLevel AS INTEGER
    currentLine AS LONG
    inFunction AS _BYTE
    inSub AS _BYTE
    currentFunction AS STRING * 128
    currentSub AS STRING * 128
    labelCount AS LONG
    tempVarCount AS LONG
    stringPoolCount AS INTEGER
    linesGenerated AS LONG
    functionsGenerated AS LONG
    subsGenerated AS LONG
END TYPE

TYPE ControlFlowState
    maxControlLevel AS INTEGER
    currentControlLevel AS INTEGER
    execLevel AS INTEGER
    execCounter AS INTEGER
    defineStackTop AS INTEGER
END TYPE

TYPE StateValidation
    isValid AS _BYTE
    lastModified AS SINGLE
    checksum AS LONG
    validationTag AS STRING * 8
END TYPE

TYPE StateSnapshot
    snapshotTime AS SINGLE
    snapshotTag AS STRING * 32
    globalStateSerialized AS STRING * 512
    symbolStateSerialized AS STRING * 256
    parserStateSerialized AS STRING * 512
    codeGenStateSerialized AS STRING * 256
    controlFlowSerialized AS STRING * 128
    isValid AS _BYTE
END TYPE

DIM SHARED GlobalState AS CompilerGlobalState
DIM SHARED SymbolState AS SymbolTableState
DIM SHARED ParserContainer AS ParserStateContainer
DIM SHARED CodeGenContainer AS CodeGenStateContainer
DIM SHARED ControlFlowContainer AS ControlFlowState
DIM SHARED StateValidator AS StateValidation
DIM SHARED StateSnapshots(1 TO MAX_SNAPSHOTS) AS StateSnapshot
DIM SHARED SnapshotCount%
DIM SHARED CurrentSnapshotIndex%

SUB InitStateManagement
    GlobalState.isCompiling = 0
    GlobalState.hasErrors = 0
    GlobalState.hasWarnings = 0
    GlobalState.currentPass = 0
    GlobalState.optimizeLevel = 1
    GlobalState.showWarnings = -1
    GlobalState.startTime = TIMER

    SymbolState.idCount = 0
    SymbolState.idCapacity = 0
    SymbolState.currentScopeLevel = 0
    SymbolState.scopeStackTop = 0
    SymbolState.udtCount = 0
    SymbolState.arrayCount = 0

    ParserContainer.currentLine = 1
    ParserContainer.currentColumn = 1
    ParserContainer.currentTokenPos = 0
    ParserContainer.sourceFile = ""
    ParserContainer.sourceLine = ""
    ParserContainer.lineLength = 0
    ParserContainer.tokenCount = 0
    ParserContainer.tokenIndex = 0
    ParserContainer.inComment = 0
    ParserContainer.inString = 0
    ParserContainer.inDirective = 0
    ParserContainer.expectEOL = 0
    ParserContainer.controlLevel = 0
    ParserContainer.ifLevel = 0
    ParserContainer.loopLevel = 0
    ParserContainer.selectLevel = 0

    CodeGenContainer.outputFile = 0
    CodeGenContainer.tempFile = 0
    CodeGenContainer.indentLevel = 0
    CodeGenContainer.currentLine = 0
    CodeGenContainer.inFunction = 0
    CodeGenContainer.inSub = 0
    CodeGenContainer.currentFunction = ""
    CodeGenContainer.currentSub = ""
    CodeGenContainer.labelCount = 0
    CodeGenContainer.tempVarCount = 0
    CodeGenContainer.stringPoolCount = 0
    CodeGenContainer.linesGenerated = 0
    CodeGenContainer.functionsGenerated = 0
    CodeGenContainer.subsGenerated = 0

    ControlFlowContainer.maxControlLevel = 0
    ControlFlowContainer.currentControlLevel = 0
    ControlFlowContainer.execLevel = 0
    ControlFlowContainer.execCounter = 0
    ControlFlowContainer.defineStackTop = 0

    SnapshotCount% = 0
    CurrentSnapshotIndex% = 0
    StateValidator.isValid = -1
    StateValidator.lastModified = TIMER
    StateValidator.checksum = 0
    StateValidator.validationTag = "QBNEX"
END SUB

SUB CleanupStateManagement
    SnapshotCount% = 0
    CurrentSnapshotIndex% = 0
    StateValidator.isValid = 0
END SUB

SUB GetGlobalState (state AS CompilerGlobalState)
    state = GlobalState
END SUB

SUB SetGlobalState (newState AS CompilerGlobalState)
    GlobalState = newState
    UpdateStateTimestamp
END SUB

SUB GetSymbolState (state AS SymbolTableState)
    state = SymbolState
END SUB

SUB SetSymbolState (newState AS SymbolTableState)
    SymbolState = newState
    UpdateStateTimestamp
END SUB

SUB GetParserState (state AS ParserStateContainer)
    state = ParserContainer
END SUB

SUB SetParserState (newState AS ParserStateContainer)
    ParserContainer = newState
    UpdateStateTimestamp
END SUB

SUB GetCodeGenState (state AS CodeGenStateContainer)
    state = CodeGenContainer
END SUB

SUB SetCodeGenState (newState AS CodeGenStateContainer)
    CodeGenContainer = newState
    UpdateStateTimestamp
END SUB

SUB GetControlFlowState (state AS ControlFlowState)
    state = ControlFlowContainer
END SUB

SUB SetControlFlowState (newState AS ControlFlowState)
    ControlFlowContainer = newState
    UpdateStateTimestamp
END SUB

SUB SetCompilingState (isCompiling AS _BYTE)
    GlobalState.isCompiling = isCompiling
    UpdateStateTimestamp
END SUB

SUB SetErrorState (hasErrors AS _BYTE, hasWarnings AS _BYTE)
    GlobalState.hasErrors = hasErrors
    GlobalState.hasWarnings = hasWarnings
    UpdateStateTimestamp
END SUB

SUB SetCurrentPass (passNum AS INTEGER)
    GlobalState.currentPass = passNum
    UpdateStateTimestamp
END SUB

SUB UpdateLineCount (linesProcessed AS LONG)
    GlobalState.linesProcessed = linesProcessed
    ParserContainer.currentLine = linesProcessed
    UpdateStateTimestamp
END SUB

SUB IncrementSymbolCount
    GlobalState.symbolsDefined = GlobalState.symbolsDefined + 1
    SymbolState.idCount = SymbolState.idCount + 1
    UpdateStateTimestamp
END SUB

SUB IncrementErrorCount
    GlobalState.errorsReported = GlobalState.errorsReported + 1
    GlobalState.hasErrors = -1
    UpdateStateTimestamp
END SUB

SUB IncrementWarningCount
    GlobalState.warningsReported = GlobalState.warningsReported + 1
    GlobalState.hasWarnings = -1
    UpdateStateTimestamp
END SUB

SUB SetMetacommandState (metacommand AS STRING, desiredState AS _BYTE, actualState AS _BYTE)
    SELECT CASE UCASE$(metacommand)
        CASE "NOPREFIX"
            GlobalState.noprefixDesired = desiredState
            GlobalState.noprefixSet = actualState
        CASE "VWATCH"
            GlobalState.vwatchDesired = desiredState
            GlobalState.vwatchOn = actualState
        CASE "OPTION_EXPLICIT"
            GlobalState.optionExplicitDesired = desiredState
            GlobalState.optionExplicitSet = actualState
        CASE "OPTION_EXPLICIT_ARRAY"
            GlobalState.optionExplicitArrayDesired = desiredState
            GlobalState.optionExplicitArraySet = actualState
    END SELECT
    UpdateStateTimestamp
END SUB

FUNCTION GetMetacommandState% (metacommand AS STRING, getDesired AS _BYTE)
    SELECT CASE UCASE$(metacommand)
        CASE "NOPREFIX"
            IF getDesired THEN GetMetacommandState% = GlobalState.noprefixDesired ELSE GetMetacommandState% = GlobalState.noprefixSet
        CASE "VWATCH"
            IF getDesired THEN GetMetacommandState% = GlobalState.vwatchDesired ELSE GetMetacommandState% = GlobalState.vwatchOn
        CASE "OPTION_EXPLICIT"
            IF getDesired THEN GetMetacommandState% = GlobalState.optionExplicitDesired ELSE GetMetacommandState% = GlobalState.optionExplicitSet
        CASE "OPTION_EXPLICIT_ARRAY"
            IF getDesired THEN GetMetacommandState% = GlobalState.optionExplicitArrayDesired ELSE GetMetacommandState% = GlobalState.optionExplicitArraySet
        CASE ELSE
            GetMetacommandState% = 0
    END SELECT
END FUNCTION

SUB CreateStateSnapshot (tag AS STRING)
    IF SnapshotCount% >= MAX_SNAPSHOTS THEN RemoveOldestSnapshot

    SnapshotCount% = SnapshotCount% + 1
    CurrentSnapshotIndex% = SnapshotCount%
    StateSnapshots(SnapshotCount%).snapshotTime = TIMER
    StateSnapshots(SnapshotCount%).snapshotTag = tag
    StateSnapshots(SnapshotCount%).globalStateSerialized = SerializeGlobalState$
    StateSnapshots(SnapshotCount%).symbolStateSerialized = SerializeSymbolState$
    StateSnapshots(SnapshotCount%).parserStateSerialized = SerializeParserState$
    StateSnapshots(SnapshotCount%).codeGenStateSerialized = SerializeCodeGenState$
    StateSnapshots(SnapshotCount%).controlFlowSerialized = SerializeControlFlowState$
    StateSnapshots(SnapshotCount%).isValid = -1
END SUB

FUNCTION RestoreStateSnapshot% (snapshotIndex AS INTEGER)
    IF snapshotIndex < 1 OR snapshotIndex > SnapshotCount% THEN
        RestoreStateSnapshot% = 0
        EXIT FUNCTION
    END IF
    IF NOT StateSnapshots(snapshotIndex).isValid THEN
        RestoreStateSnapshot% = 0
        EXIT FUNCTION
    END IF

    DeserializeGlobalState StateSnapshots(snapshotIndex).globalStateSerialized
    DeserializeSymbolState StateSnapshots(snapshotIndex).symbolStateSerialized
    DeserializeParserState StateSnapshots(snapshotIndex).parserStateSerialized
    DeserializeCodeGenState StateSnapshots(snapshotIndex).codeGenStateSerialized
    DeserializeControlFlowState StateSnapshots(snapshotIndex).controlFlowSerialized
    CurrentSnapshotIndex% = snapshotIndex
    RestoreStateSnapshot% = -1
END FUNCTION

SUB RemoveOldestSnapshot
    DIM i%

    FOR i% = 1 TO MAX_SNAPSHOTS - 1
        StateSnapshots(i%) = StateSnapshots(i% + 1)
    NEXT

    SnapshotCount% = MAX_SNAPSHOTS - 1
    IF CurrentSnapshotIndex% > SnapshotCount% THEN CurrentSnapshotIndex% = SnapshotCount%
END SUB

SUB ClearAllSnapshots
    DIM i%

    FOR i% = 1 TO MAX_SNAPSHOTS
        StateSnapshots(i%).isValid = 0
    NEXT

    SnapshotCount% = 0
    CurrentSnapshotIndex% = 0
END SUB

FUNCTION SerializeGlobalState$ ()
    SerializeGlobalState$ = MKI$(GlobalState.currentPass) + MKI$(GlobalState.optimizeLevel) + CHR$(GlobalState.isCompiling) + CHR$(GlobalState.hasErrors) + CHR$(GlobalState.hasWarnings)
END FUNCTION

SUB DeserializeGlobalState (s AS STRING)
    IF LEN(s) >= 2 THEN GlobalState.currentPass = CVI(LEFT$(s, 2))
    IF LEN(s) >= 4 THEN GlobalState.optimizeLevel = CVI(MID$(s, 3, 2))
    IF LEN(s) >= 5 THEN GlobalState.isCompiling = ASC(MID$(s, 5, 1))
    IF LEN(s) >= 6 THEN GlobalState.hasErrors = ASC(MID$(s, 6, 1))
    IF LEN(s) >= 7 THEN GlobalState.hasWarnings = ASC(MID$(s, 7, 1))
END SUB

FUNCTION SerializeSymbolState$ ()
    SerializeSymbolState$ = MKL$(SymbolState.idCount) + MKI$(SymbolState.currentScopeLevel)
END FUNCTION

SUB DeserializeSymbolState (s AS STRING)
    IF LEN(s) >= 4 THEN SymbolState.idCount = CVL(LEFT$(s, 4))
    IF LEN(s) >= 6 THEN SymbolState.currentScopeLevel = CVI(MID$(s, 5, 2))
END SUB

FUNCTION SerializeParserState$ ()
    SerializeParserState$ = MKL$(ParserContainer.currentLine) + MKI$(ParserContainer.currentColumn)
END FUNCTION

SUB DeserializeParserState (s AS STRING)
    IF LEN(s) >= 4 THEN ParserContainer.currentLine = CVL(LEFT$(s, 4))
    IF LEN(s) >= 6 THEN ParserContainer.currentColumn = CVI(MID$(s, 5, 2))
END SUB

FUNCTION SerializeCodeGenState$ ()
    SerializeCodeGenState$ = MKL$(CodeGenContainer.labelCount) + MKL$(CodeGenContainer.tempVarCount)
END FUNCTION

SUB DeserializeCodeGenState (s AS STRING)
    IF LEN(s) >= 4 THEN CodeGenContainer.labelCount = CVL(LEFT$(s, 4))
    IF LEN(s) >= 8 THEN CodeGenContainer.tempVarCount = CVL(MID$(s, 5, 4))
END SUB

FUNCTION SerializeControlFlowState$ ()
    SerializeControlFlowState$ = MKI$(ControlFlowContainer.currentControlLevel) + MKI$(ControlFlowContainer.execLevel)
END FUNCTION

SUB DeserializeControlFlowState (s AS STRING)
    IF LEN(s) >= 2 THEN ControlFlowContainer.currentControlLevel = CVI(LEFT$(s, 2))
    IF LEN(s) >= 4 THEN ControlFlowContainer.execLevel = CVI(MID$(s, 3, 2))
END SUB

FUNCTION ValidateState% ()
    IF GlobalState.linesProcessed < 0 OR GlobalState.symbolsDefined < 0 OR SymbolState.currentScopeLevel < 0 OR ParserContainer.currentLine < 0 THEN
        ValidateState% = 0
    ELSE
        ValidateState% = -1
    END IF
END FUNCTION

SUB UpdateStateTimestamp
    StateValidator.lastModified = TIMER
END SUB

SUB PrintStateReport
    PRINT "=== Compiler State Report ==="
    PRINT "Compiling: "; IIF%(GlobalState.isCompiling, -1, 0)
    PRINT "Current Pass: "; GlobalState.currentPass
    PRINT "Lines Processed: "; GlobalState.linesProcessed
    PRINT "Symbols Defined: "; GlobalState.symbolsDefined
    PRINT "Errors: "; GlobalState.errorsReported
    PRINT "Warnings: "; GlobalState.warningsReported
    PRINT "Scope Level: "; SymbolState.currentScopeLevel
    PRINT "Parser Line: "; ParserContainer.currentLine
    PRINT "Labels Generated: "; CodeGenContainer.labelCount
    PRINT "Snapshots Available: "; SnapshotCount%
    PRINT "============================="
END SUB

FUNCTION IIF% (condition AS _BYTE, trueVal AS INTEGER, falseVal AS INTEGER)
    IF condition THEN IIF% = trueVal ELSE IIF% = falseVal
END FUNCTION
