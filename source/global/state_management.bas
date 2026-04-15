'===============================================================================
' QBNex State Management System
'===============================================================================
' Centralized state management for the compiler to replace global variable
' pollution with structured state containers.
'
' Features:
' - Encapsulated compiler state
' - State validation and integrity checking
' - Snapshot/restore for error recovery
' - Thread-safe state access (future)
'===============================================================================

'-------------------------------------------------------------------------------
' STATE SCOPE TYPES
'-------------------------------------------------------------------------------

CONST SCOPE_GLOBAL = 0
CONST SCOPE_MODULE = 1
CONST SCOPE_FUNCTION = 2
CONST SCOPE_SUB = 3
CONST SCOPE_TYPE = 4
CONST SCOPE_BLOCK = 5

'-------------------------------------------------------------------------------
' COMPILER STATE CONTAINER
'-------------------------------------------------------------------------------

TYPE CompilerGlobalState
    'Compilation flags
    isCompiling AS _BYTE
    hasErrors AS _BYTE
    hasWarnings AS _BYTE
    currentPass AS INTEGER
    
    'Metacommand states
    noprefixDesired AS _BYTE
    noprefixSet AS _BYTE
    vwatchDesired AS _BYTE
    vwatchOn AS _BYTE
    optionExplicitDesired AS _BYTE
    optionExplicitSet AS _BYTE
    optionExplicitArrayDesired AS _BYTE
    optionExplicitArraySet AS _BYTE
    
    'Statistics
    startTime AS SINGLE
    endTime AS SINGLE
    linesProcessed AS LONG
    linesCompiled AS LONG
    symbolsDefined AS LONG
    symbolsResolved AS LONG
    errorsReported AS LONG
    warningsReported AS LONG
    
    'Configuration
    optimizeLevel AS INTEGER
    debugMode AS _BYTE
    consoleMode AS _BYTE
    quietMode AS _BYTE
    showWarnings AS _BYTE
END TYPE

'-------------------------------------------------------------------------------
' SYMBOL TABLE STATE
'-------------------------------------------------------------------------------

TYPE SymbolTableState
    'Hash table state
    hashListNext AS LONG
    hashListFreeLast AS LONG
    hashTableSize AS LONG
    
    'Identifier tracking
    idCount AS LONG
    idCapacity AS LONG
    
    'Scope tracking
    currentScopeLevel AS INTEGER
    scopeStackTop AS INTEGER
    
    'UDT tracking
    udtCount AS INTEGER
    udtCapacity AS INTEGER
    
    'Array tracking
    arrayCount AS LONG
    arrayCapacity AS LONG
END TYPE

'-------------------------------------------------------------------------------
' PARSER STATE
'-------------------------------------------------------------------------------

TYPE ParserStateContainer
    'Position tracking
    currentLine AS LONG
    currentColumn AS INTEGER
    currentTokenPos AS LONG
    
    'Input state
    sourceFile AS STRING * 260
    sourceLine AS STRING * 2048
    lineLength AS INTEGER
    
    'Token buffer
    tokenCount AS INTEGER
    tokenIndex AS INTEGER
    
    'Parsing flags
    inComment AS _BYTE
    inString AS _BYTE
    inDirective AS _BYTE
    expectEOL AS _BYTE
    
    'Control structure tracking
    controlLevel AS INTEGER
    ifLevel AS INTEGER
    loopLevel AS INTEGER
    selectLevel AS INTEGER
END TYPE

'-------------------------------------------------------------------------------
' CODE GENERATION STATE
'-------------------------------------------------------------------------------

TYPE CodeGenStateContainer
    'Output state
    outputFile AS INTEGER
    tempFile AS INTEGER
    indentLevel AS INTEGER
    currentLine AS LONG
    
    'Function context
    inFunction AS _BYTE
    inSub AS _BYTE
    currentFunction AS STRING * 128
    currentSub AS STRING * 128
    
    'Label and variable tracking
    labelCount AS LONG
    tempVarCount AS LONG
    stringPoolCount AS INTEGER
    
    'Code statistics
    linesGenerated AS LONG
    functionsGenerated AS LONG
    subsGenerated AS LONG
END TYPE

'-------------------------------------------------------------------------------
' CONTROL FLOW STATE
'-------------------------------------------------------------------------------

TYPE ControlFlowState
    'Nesting levels
    maxControlLevel AS INTEGER
    currentControlLevel AS INTEGER
    
    'Control type tracking
    controlType(1 TO 100) AS INTEGER
    controlLine(1 TO 100) AS LONG
    controlLabel(1 TO 100) AS STRING * 32
    
    'Execution tracking
    execLevel AS INTEGER
    execCounter AS INTEGER
    execState(1 TO 100) AS INTEGER
    
    'Define/conditional compilation
    defineStackTop AS INTEGER
    defineElse(1 TO 100) AS _BYTE
    defineActive(1 TO 100) AS _BYTE
END TYPE

'-------------------------------------------------------------------------------
' STATE VALIDATION
'-------------------------------------------------------------------------------

TYPE StateValidation
    isValid AS _BYTE
    lastModified AS SINGLE
    checksum AS LONG
    validationTag AS STRING * 8
END TYPE

'-------------------------------------------------------------------------------
' STATE SNAPSHOT (FOR ERROR RECOVERY)
'-------------------------------------------------------------------------------

TYPE StateSnapshot
    snapshotTime AS SINGLE
    snapshotTag AS STRING * 32
    
    'Snapshot data (serialized states)
    globalStateSerialized AS STRING * 512
    symbolStateSerialized AS STRING * 256
    parserStateSerialized AS STRING * 512
    codeGenStateSerialized AS STRING * 256
    controlFlowSerialized AS STRING * 512
    
    isValid AS _BYTE
END TYPE

'-------------------------------------------------------------------------------
' MODULE STATE
'-------------------------------------------------------------------------------

DIM SHARED GlobalState AS CompilerGlobalState
DIM SHARED SymbolState AS SymbolTableState
DIM SHARED ParserContainer AS ParserStateContainer
DIM SHARED CodeGenContainer AS CodeGenStateContainer
DIM SHARED ControlFlowContainer AS ControlFlowState
DIM SHARED StateValidator AS StateValidation

DIM SHARED StateSnapshots(1 TO 10) AS StateSnapshot
DIM SHARED SnapshotCount AS INTEGER
DIM SHARED CurrentSnapshotIndex AS INTEGER

CONST MAX_SNAPSHOTS = 10

'-------------------------------------------------------------------------------
' INITIALIZATION
'-------------------------------------------------------------------------------

SUB InitStateManagement
    'Initialize global state
    WITH GlobalState
        .isCompiling = 0
        .hasErrors = 0
        .hasWarnings = 0
        .currentPass = 0
        
        .noprefixDesired = 0
        .noprefixSet = 0
        .vwatchDesired = 0
        .vwatchOn = 0
        .optionExplicitDesired = 0
        .optionExplicitSet = 0
        .optionExplicitArrayDesired = 0
        .optionExplicitArraySet = 0
        
        .startTime = 0
        .endTime = 0
        .linesProcessed = 0
        .linesCompiled = 0
        .symbolsDefined = 0
        .symbolsResolved = 0
        .errorsReported = 0
        .warningsReported = 0
        
        .optimizeLevel = 1
        .debugMode = 0
        .consoleMode = 0
        .quietMode = 0
        .showWarnings = -1
    END WITH
    
    'Initialize symbol table state
    WITH SymbolState
        .hashListNext = 1
        .hashListFreeLast = 0
        .hashTableSize = 0
        .idCount = 0
        .idCapacity = 0
        .currentScopeLevel = 0
        .scopeStackTop = 0
        .udtCount = 0
        .udtCapacity = 0
        .arrayCount = 0
        .arrayCapacity = 0
    END WITH
    
    'Initialize parser state
    WITH ParserContainer
        .currentLine = 0
        .currentColumn = 0
        .currentTokenPos = 0
        .sourceFile = ""
        .sourceLine = ""
        .lineLength = 0
        .tokenCount = 0
        .tokenIndex = 0
        .inComment = 0
        .inString = 0
        .inDirective = 0
        .expectEOL = 0
        .controlLevel = 0
        .ifLevel = 0
        .loopLevel = 0
        .selectLevel = 0
    END WITH
    
    'Initialize code generation state
    WITH CodeGenContainer
        .outputFile = 0
        .tempFile = 0
        .indentLevel = 0
        .currentLine = 0
        .inFunction = 0
        .inSub = 0
        .currentFunction = ""
        .currentSub = ""
        .labelCount = 0
        .tempVarCount = 0
        .stringPoolCount = 0
        .linesGenerated = 0
        .functionsGenerated = 0
        .subsGenerated = 0
    END WITH
    
    'Initialize control flow state
    WITH ControlFlowContainer
        .maxControlLevel = 100
        .currentControlLevel = 0
        .execLevel = 0
        .execCounter = 0
        .defineStackTop = 0
    END WITH
    
    'Initialize validator
    StateValidator.isValid = -1
    StateValidator.lastModified = TIMER
    StateValidator.checksum = 0
    StateValidator.validationTag = "QBNEXST1"
    
    'Initialize snapshots
    SnapshotCount = 0
    CurrentSnapshotIndex = 0
END SUB

SUB CleanupStateManagement
    StateValidator.isValid = 0
    SnapshotCount = 0
    CurrentSnapshotIndex = 0
END SUB

'-------------------------------------------------------------------------------
' GLOBAL STATE ACCESS
'-------------------------------------------------------------------------------

FUNCTION GetGlobalState () AS CompilerGlobalState
    GetGlobalState = GlobalState
END FUNCTION

SUB SetGlobalState (newState AS CompilerGlobalState)
    GlobalState = newState
    UpdateStateTimestamp
END SUB

'-------------------------------------------------------------------------------
' SYMBOL TABLE STATE ACCESS
'-------------------------------------------------------------------------------

FUNCTION GetSymbolState () AS SymbolTableState
    GetSymbolState = SymbolState
END FUNCTION

SUB SetSymbolState (newState AS SymbolTableState)
    SymbolState = newState
    UpdateStateTimestamp
END SUB

'-------------------------------------------------------------------------------
' PARSER STATE ACCESS
'-------------------------------------------------------------------------------

FUNCTION GetParserState () AS ParserStateContainer
    GetParserState = ParserContainer
END FUNCTION

SUB SetParserState (newState AS ParserStateContainer)
    ParserContainer = newState
    UpdateStateTimestamp
END SUB

'-------------------------------------------------------------------------------
' CODE GENERATION STATE ACCESS
'-------------------------------------------------------------------------------

FUNCTION GetCodeGenState () AS CodeGenStateContainer
    GetCodeGenState = CodeGenContainer
END FUNCTION

SUB SetCodeGenState (newState AS CodeGenStateContainer)
    CodeGenContainer = newState
    UpdateStateTimestamp
END SUB

'-------------------------------------------------------------------------------
' CONTROL FLOW STATE ACCESS
'-------------------------------------------------------------------------------

FUNCTION GetControlFlowState () AS ControlFlowState
    GetControlFlowState = ControlFlowContainer
END FUNCTION

SUB SetControlFlowState (newState AS ControlFlowState)
    ControlFlowContainer = newState
    UpdateStateTimestamp
END SUB

'-------------------------------------------------------------------------------
' STATE MODIFIERS
'-------------------------------------------------------------------------------

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
    UpdateStateTimestamp
END SUB

SUB IncrementSymbolCount
    GlobalState.symbolsDefined = GlobalState.symbolsDefined + 1
    UpdateStateTimestamp
END SUB

SUB IncrementErrorCount
    GlobalState.errorsReported = GlobalState.errorsReported + 1
    UpdateStateTimestamp
END SUB

SUB IncrementWarningCount
    GlobalState.warningsReported = GlobalState.warningsReported + 1
    UpdateStateTimestamp
END SUB

'-------------------------------------------------------------------------------
' METACOMMAND STATE MANAGEMENT
'-------------------------------------------------------------------------------

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
            IF getDesired THEN
                GetMetacommandState% = GlobalState.noprefixDesired
            ELSE
                GetMetacommandState% = GlobalState.noprefixSet
            END IF
        CASE "VWATCH"
            IF getDesired THEN
                GetMetacommandState% = GlobalState.vwatchDesired
            ELSE
                GetMetacommandState% = GlobalState.vwatchOn
            END IF
        CASE "OPTION_EXPLICIT"
            IF getDesired THEN
                GetMetacommandState% = GlobalState.optionExplicitDesired
            ELSE
                GetMetacommandState% = GlobalState.optionExplicitSet
            END IF
        CASE "OPTION_EXPLICIT_ARRAY"
            IF getDesired THEN
                GetMetacommandState% = GlobalState.optionExplicitArrayDesired
            ELSE
                GetMetacommandState% = GlobalState.optionExplicitArraySet
            END IF
        CASE ELSE
            GetMetacommandState% = 0
    END SELECT
END FUNCTION

'-------------------------------------------------------------------------------
' SNAPSHOT MANAGEMENT
'-------------------------------------------------------------------------------

SUB CreateStateSnapshot (tag AS STRING)
    IF SnapshotCount >= MAX_SNAPSHOTS THEN
        'Remove oldest snapshot
        RemoveOldestSnapshot
    END IF
    
    SnapshotCount = SnapshotCount + 1
    CurrentSnapshotIndex = SnapshotCount
    
    WITH StateSnapshots(SnapshotCount)
        .snapshotTime = TIMER
        .snapshotTag = tag
        .globalStateSerialized = SerializeGlobalState$
        .symbolStateSerialized = SerializeSymbolState$
        .parserStateSerialized = SerializeParserState$
        .codeGenStateSerialized = SerializeCodeGenState$
        .controlFlowSerialized = SerializeControlFlowState$
        .isValid = -1
    END WITH
END SUB

FUNCTION RestoreStateSnapshot% (snapshotIndex AS INTEGER)
    IF snapshotIndex < 1 OR snapshotIndex > SnapshotCount THEN
        RestoreStateSnapshot% = 0
        EXIT FUNCTION
    END IF
    
    IF NOT StateSnapshots(snapshotIndex).isValid THEN
        RestoreStateSnapshot% = 0
        EXIT FUNCTION
    END IF
    
    'Restore all states from snapshot
    DeserializeGlobalState StateSnapshots(snapshotIndex).globalStateSerialized
    DeserializeSymbolState StateSnapshots(snapshotIndex).symbolStateSerialized
    DeserializeParserState StateSnapshots(snapshotIndex).parserStateSerialized
    DeserializeCodeGenState StateSnapshots(snapshotIndex).codeGenStateSerialized
    DeserializeControlFlowState StateSnapshots(snapshotIndex).controlFlowSerialized
    
    CurrentSnapshotIndex = snapshotIndex
    RestoreStateSnapshot% = -1
END FUNCTION

SUB RemoveOldestSnapshot
    DIM i AS INTEGER
    
    FOR i = 1 TO MAX_SNAPSHOTS - 1
        StateSnapshots(i) = StateSnapshots(i + 1)
    NEXT
    
    SnapshotCount = MAX_SNAPSHOTS - 1
END SUB

SUB ClearAllSnapshots
    DIM i AS INTEGER
    
    FOR i = 1 to MAX_SNAPSHOTS
        StateSnapshots(i).isValid = 0
    NEXT
    
    SnapshotCount = 0
    CurrentSnapshotIndex = 0
END SUB

'-------------------------------------------------------------------------------
' STATE SERIALIZATION (Simplified)
'-------------------------------------------------------------------------------

FUNCTION SerializeGlobalState$ ()
    'Simple serialization - convert key values to string
    DIM s AS STRING
    s = MKI$(GlobalState.currentPass)
    s = s + MKI$(GlobalState.optimizeLevel)
    s = s + CHR$(GlobalState.isCompiling)
    s = s + CHR$(GlobalState.hasErrors)
    s = s + CHR$(GlobalState.hasWarnings)
    SerializeGlobalState$ = s
END FUNCTION

SUB DeserializeGlobalState (s AS STRING)
    IF LEN(s) >= 2 THEN GlobalState.currentPass = CVI(LEFT$(s, 2))
    IF LEN(s) >= 4 THEN GlobalState.optimizeLevel = CVI(MID$(s, 3, 2))
    IF LEN(s) >= 5 THEN GlobalState.isCompiling = ASC(MID$(s, 5, 1))
    IF LEN(s) >= 6 THEN GlobalState.hasErrors = ASC(MID$(s, 6, 1))
    IF LEN(s) >= 7 THEN GlobalState.hasWarnings = ASC(MID$(s, 7, 1))
END SUB

FUNCTION SerializeSymbolState$ ()
    DIM s AS STRING
    s = MKL$(SymbolState.idCount)
    s = s + MKI$(SymbolState.currentScopeLevel)
    SerializeSymbolState$ = s
END FUNCTION

SUB DeserializeSymbolState (s AS STRING)
    IF LEN(s) >= 4 THEN SymbolState.idCount = CVL(LEFT$(s, 4))
    IF LEN(s) >= 6 THEN SymbolState.currentScopeLevel = CVI(MID$(s, 5, 2))
END SUB

FUNCTION SerializeParserState$ ()
    DIM s AS STRING
    s = MKL$(ParserContainer.currentLine)
    s = s + MKI$(ParserContainer.currentColumn)
    SerializeParserState$ = s
END FUNCTION

SUB DeserializeParserState (s AS STRING)
    IF LEN(s) >= 4 THEN ParserContainer.currentLine = CVL(LEFT$(s, 4))
    IF LEN(s) >= 6 THEN ParserContainer.currentColumn = CVI(MID$(s, 5, 2))
END SUB

FUNCTION SerializeCodeGenState$ ()
    DIM s AS STRING
    s = MKL$(CodeGenContainer.labelCount)
    s = s + MKL$(CodeGenContainer.tempVarCount)
    SerializeCodeGenState$ = s
END FUNCTION

SUB DeserializeCodeGenState (s AS STRING)
    IF LEN(s) >= 4 THEN CodeGenContainer.labelCount = CVL(LEFT$(s, 4))
    IF LEN(s) >= 8 THEN CodeGenContainer.tempVarCount = CVL(MID$(s, 5, 4))
END SUB

FUNCTION SerializeControlFlowState$ ()
    DIM s AS STRING
    s = MKI$(ControlFlowContainer.currentControlLevel)
    s = s + MKI$(ControlFlowContainer.execLevel)
    SerializeControlFlowState$ = s
END FUNCTION

SUB DeserializeControlFlowState (s AS STRING)
    IF LEN(s) >= 2 THEN ControlFlowContainer.currentControlLevel = CVI(LEFT$(s, 2))
    IF LEN(s) >= 4 THEN ControlFlowContainer.execLevel = CVI(MID$(s, 3, 2))
END SUB

'-------------------------------------------------------------------------------
' STATE VALIDATION
'-------------------------------------------------------------------------------

FUNCTION ValidateState% ()
    'Check if all states are consistent
    DIM isValid AS _BYTE
    isValid = -1
    
    'Check for obvious inconsistencies
    IF GlobalState.linesProcessed < 0 THEN isValid = 0
    IF GlobalState.symbolsDefined < 0 THEN isValid = 0
    IF SymbolState.currentScopeLevel < 0 THEN isValid = 0
    IF ParserContainer.currentLine < 0 THEN isValid = 0
    
    ValidateState% = isValid
END FUNCTION

SUB UpdateStateTimestamp
    StateValidator.lastModified = TIMER
END SUB

'-------------------------------------------------------------------------------
' STATISTICS AND REPORTING
'-------------------------------------------------------------------------------

SUB PrintStateReport
    PRINT "=== Compiler State Report ==="
    PRINT "Compiling: "; IIF(GlobalState.isCompiling, "Yes", "No")
    PRINT "Current Pass: "; GlobalState.currentPass
    PRINT "Lines Processed: "; GlobalState.linesProcessed
    PRINT "Symbols Defined: "; GlobalState.symbolsDefined
    PRINT "Errors: "; GlobalState.errorsReported
    PRINT "Warnings: "; GlobalState.warningsReported
    PRINT "Scope Level: "; SymbolState.currentScopeLevel
    PRINT "Parser Line: "; ParserContainer.currentLine
    PRINT "Labels Generated: "; CodeGenContainer.labelCount
    PRINT "Snapshots Available: "; SnapshotCount
    PRINT "============================="
END SUB

FUNCTION IIF% (condition AS _BYTE, trueVal AS INTEGER, falseVal AS INTEGER)
    IF condition THEN IIF% = trueVal ELSE IIF% = falseVal
END FUNCTION

