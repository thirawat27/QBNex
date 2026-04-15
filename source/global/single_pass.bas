'===============================================================================
' QBNex Single-Pass Compilation System
'===============================================================================
' This module implements deferred reference resolution to eliminate
' multiple recompilation passes. References that cannot be resolved
' immediately are stored and resolved after the first pass.
'===============================================================================

'-------------------------------------------------------------------------------
' DEFERRED REFERENCE TYPES
'-------------------------------------------------------------------------------

CONST DEFREF_TYPE_LABEL = 1
CONST DEFREF_TYPE_VARIABLE = 2
CONST DEFREF_TYPE_FUNCTION = 3
CONST DEFREF_TYPE_SUB = 4
CONST DEFREF_TYPE_UDT = 5
CONST DEFREF_TYPE_CONSTANT = 6

TYPE DeferredReference
    name AS STRING
    lineNumber AS LONG
    refType AS INTEGER
    scope AS LONG
    flags AS LONG
    isResolved AS _BYTE
    resolvedIndex AS LONG
END TYPE

'-------------------------------------------------------------------------------
' DEFERRED REFERENCE MANAGEMENT
'-------------------------------------------------------------------------------

DIM SHARED DeferredRefs() AS DeferredReference
DIM SHARED DeferredRefCount AS LONG
DIM SHARED DeferredRefCapacity AS LONG
DIM SHARED SinglePassMode AS _BYTE

CONST INITIAL_DEFERRED_CAPACITY = 1000

' Initialize the deferred reference system
SUB InitDeferredReferences
    DeferredRefCapacity = INITIAL_DEFERRED_CAPACITY
    REDIM DeferredRefs(1 TO DeferredRefCapacity) AS DeferredReference
    DeferredRefCount = 0
    SinglePassMode = -1 'Enabled by default
END SUB

' Clear all deferred references
SUB ClearDeferredReferences
    DeferredRefCount = 0
    ' Keep the capacity for reuse
    DIM i AS LONG
    FOR i = 1 TO DeferredRefCapacity
        DeferredRefs(i).name = ""
        DeferredRefs(i).isResolved = 0
    NEXT
END SUB

' Add a deferred reference
SUB AddDeferredReference (name AS STRING, lineNum AS LONG, refType AS INTEGER, scope AS LONG, flags AS LONG)
    IF NOT SinglePassMode THEN EXIT SUB
    
    ' Check if we need to expand the array
    IF DeferredRefCount >= DeferredRefCapacity THEN
        DeferredRefCapacity = DeferredRefCapacity * 2
        REDIM _PRESERVE DeferredRefs(1 TO DeferredRefCapacity) AS DeferredReference
    END IF
    
    DeferredRefCount = DeferredRefCount + 1
    DeferredRefs(DeferredRefCount).name = name
    DeferredRefs(DeferredRefCount).lineNumber = lineNum
    DeferredRefs(DeferredRefCount).refType = refType
    DeferredRefs(DeferredRefCount).scope = scope
    DeferredRefs(DeferredRefCount).flags = flags
    DeferredRefs(DeferredRefCount).isResolved = 0
    DeferredRefs(DeferredRefCount).resolvedIndex = 0
END SUB

' Check if a reference is already deferred
FUNCTION IsDeferred% (name AS STRING, refType AS INTEGER)
    DIM i AS LONG
    FOR i = 1 TO DeferredRefCount
        IF DeferredRefs(i).refType = refType AND DeferredRefs(i).name = name THEN
            IF NOT DeferredRefs(i).isResolved THEN
                IsDeferred% = -1
                EXIT FUNCTION
            END IF
        END IF
    NEXT
    IsDeferred% = 0
END FUNCTION

' Resolve all deferred references after the first pass
SUB ResolveDeferredReferences
    IF NOT SinglePassMode OR DeferredRefCount = 0 THEN EXIT SUB
    
    DIM i AS LONG
    DIM resultFlags AS LONG, resultRef AS LONG
    DIM searchFlags AS LONG
    
    FOR i = 1 TO DeferredRefCount
        IF DeferredRefs(i).isResolved THEN GOTO nextRef
        
        ' Determine search flags based on reference type
        SELECT CASE DeferredRefs(i).refType
            CASE DEFREF_TYPE_LABEL
                searchFlags = HASHFLAG_LABEL
            CASE DEFREF_TYPE_VARIABLE
                searchFlags = HASHFLAG_VARIABLE
            CASE DEFREF_TYPE_FUNCTION
                searchFlags = HASHFLAG_FUNCTION
            CASE DEFREF_TYPE_SUB
                searchFlags = HASHFLAG_SUB
            CASE DEFREF_TYPE_UDT
                searchFlags = HASHFLAG_UDT
            CASE DEFREF_TYPE_CONSTANT
                searchFlags = HASHFLAG_CONSTANT
            CASE ELSE
                GOTO nextRef
        END SELECT
        
        ' Try to find the symbol
        IF HashFind(DeferredRefs(i).name, searchFlags, resultFlags, resultRef) THEN
            DeferredRefs(i).isResolved = -1
            DeferredRefs(i).resolvedIndex = resultRef
        END IF
        
        nextRef:
    NEXT
END SUB

' Report unresolved references as errors
SUB ReportUnresolvedReferences
    IF DeferredRefCount = 0 THEN EXIT SUB
    
    DIM i AS LONG
    DIM hasUnresolved AS _BYTE
    hasUnresolved = 0
    
    FOR i = 1 TO DeferredRefCount
        IF NOT DeferredRefs(i).isResolved THEN
            hasUnresolved = -1
            ' Would print error here, but we need access to error reporting
            ' This is a placeholder for the actual error reporting
        END IF
    NEXT
    
    ' If all resolved, clear the list
    IF NOT hasUnresolved THEN
        ClearDeferredReferences
    END IF
END SUB

' Get the resolved index for a deferred reference
FUNCTION GetDeferredResolvedIndex& (name AS STRING, refType AS INTEGER)
    DIM i AS LONG
    FOR i = 1 TO DeferredRefCount
        IF DeferredRefs(i).refType = refType AND DeferredRefs(i).name = name THEN
            IF DeferredRefs(i).isResolved THEN
                GetDeferredResolvedIndex& = DeferredRefs(i).resolvedIndex
                EXIT FUNCTION
            END IF
        END IF
    NEXT
    GetDeferredResolvedIndex& = 0
END FUNCTION

'-------------------------------------------------------------------------------
' SMART RECOMPILATION DECISION
'-------------------------------------------------------------------------------

' Check if recompilation can be avoided using deferred references
FUNCTION CanAvoidRecompile% (triggerType AS STRING)
    IF NOT SinglePassMode THEN CanAvoidRecompile% = 0: EXIT FUNCTION
    
    ' List of triggers that can be handled via deferred references
    SELECT CASE triggerType
        CASE "LABEL", "VARIABLE", "FUNCTION", "SUB", "UDT", "CONSTANT"
            ' These can be deferred
            CanAvoidRecompile% = -1
        CASE "NOPREFIX", "VWATCH"
            ' These require immediate handling but can be optimized
            CanAvoidRecompile% = 0
        CASE ELSE
            CanAvoidRecompile% = 0
    END SELECT
END FUNCTION

'-------------------------------------------------------------------------------
' STATE MANAGEMENT FOR SINGLE-PASS
'-------------------------------------------------------------------------------

TYPE CompilationState
    ' Metacommand states
    noprefixDesired AS _BYTE
    noprefixSet AS _BYTE
    vwatchDesired AS _BYTE
    vwatchOn AS _BYTE
    optionExplicitDesired AS _BYTE
    optionExplicitSet AS _BYTE
    optionExplicitArrayDesired AS _BYTE
    optionExplicitArraySet AS _BYTE
    
    ' Compilation statistics
    linesProcessed AS LONG
    symbolsDefined AS LONG
    referencesDeferred AS LONG
    referencesResolved AS LONG
END TYPE

DIM SHARED CurrentState AS CompilationState

SUB InitCompilationState
    CurrentState.noprefixDesired = 0
    CurrentState.noprefixSet = 0
    CurrentState.vwatchDesired = 0
    CurrentState.vwatchOn = 0
    CurrentState.optionExplicitDesired = 0
    CurrentState.optionExplicitSet = 0
    CurrentState.optionExplicitArrayDesired = 0
    CurrentState.optionExplicitArraySet = 0
    CurrentState.linesProcessed = 0
    CurrentState.symbolsDefined = 0
    CurrentState.referencesDeferred = 0
    CurrentState.referencesResolved = 0
END SUB

SUB UpdateCompilationState (lineNum AS LONG)
    CurrentState.linesProcessed = lineNum
    CurrentState.referencesDeferred = DeferredRefCount
END SUB

FUNCTION GetLinesProcessed&
    GetLinesProcessed& = CurrentState.linesProcessed
END FUNCTION

'-------------------------------------------------------------------------------
' UTILITY FUNCTIONS
'-------------------------------------------------------------------------------

' Print deferred reference statistics (for debugging)
SUB PrintDeferredStats
    PRINT "Deferred References: "; DeferredRefCount
    
    DIM i AS LONG, resolved AS LONG
    resolved = 0
    FOR i = 1 TO DeferredRefCount
        IF DeferredRefs(i).isResolved THEN resolved = resolved + 1
    NEXT
    
    PRINT "Resolved: "; resolved
    PRINT "Unresolved: "; DeferredRefCount - resolved
END SUB

' Cleanup
SUB CleanupDeferredReferences
    ERASE DeferredRefs
    DeferredRefCount = 0
    DeferredRefCapacity = 0
END SUB
