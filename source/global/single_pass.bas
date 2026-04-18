'===============================================================================
' QBNex Single-Pass Compilation System
'===============================================================================
' Stage0-compatible deferred reference tracker.
'===============================================================================

CONST DEFREF_TYPE_LABEL = 1
CONST DEFREF_TYPE_VARIABLE = 2
CONST DEFREF_TYPE_FUNCTION = 3
CONST DEFREF_TYPE_SUB = 4
CONST DEFREF_TYPE_UDT = 5
CONST DEFREF_TYPE_CONSTANT = 6
CONST INITIAL_DEFERRED_CAPACITY = 1000

DIM SHARED DeferredRefName$(1 TO INITIAL_DEFERRED_CAPACITY)
DIM SHARED DeferredRefLine&(1 TO INITIAL_DEFERRED_CAPACITY)
DIM SHARED DeferredRefType%(1 TO INITIAL_DEFERRED_CAPACITY)
DIM SHARED DeferredRefScope&(1 TO INITIAL_DEFERRED_CAPACITY)
DIM SHARED DeferredRefFlags&(1 TO INITIAL_DEFERRED_CAPACITY)
DIM SHARED DeferredRefResolved%(1 TO INITIAL_DEFERRED_CAPACITY)
DIM SHARED DeferredRefResolvedIndex&(1 TO INITIAL_DEFERRED_CAPACITY)
DIM SHARED DeferredRefCount&
DIM SHARED DeferredRefCapacity&
DIM SHARED SinglePassMode%

DIM SHARED CompilationLinesProcessed&
DIM SHARED CompilationSymbolsDefined&
DIM SHARED CompilationReferencesDeferred&
DIM SHARED CompilationReferencesResolved&

SUB InitDeferredReferences
    DeferredRefCapacity& = INITIAL_DEFERRED_CAPACITY
    DeferredRefCount& = 0
    SinglePassMode% = -1
    ClearDeferredReferences
END SUB

SUB ClearDeferredReferences
    DIM i&

    FOR i& = 1 TO DeferredRefCapacity&
        DeferredRefName$(i&) = ""
        DeferredRefLine&(i&) = 0
        DeferredRefType%(i&) = 0
        DeferredRefScope&(i&) = 0
        DeferredRefFlags&(i&) = 0
        DeferredRefResolved%(i&) = 0
        DeferredRefResolvedIndex&(i&) = 0
    NEXT

    DeferredRefCount& = 0
END SUB

SUB AddDeferredReference (refName AS STRING, lineNum AS LONG, refType AS INTEGER, scopeId AS LONG, refFlags AS LONG)
    IF NOT SinglePassMode% THEN EXIT SUB
    IF DeferredRefCount& >= DeferredRefCapacity& THEN EXIT SUB

    DeferredRefCount& = DeferredRefCount& + 1
    DeferredRefName$(DeferredRefCount&) = refName
    DeferredRefLine&(DeferredRefCount&) = lineNum
    DeferredRefType%(DeferredRefCount&) = refType
    DeferredRefScope&(DeferredRefCount&) = scopeId
    DeferredRefFlags&(DeferredRefCount&) = refFlags
    DeferredRefResolved%(DeferredRefCount&) = 0
    DeferredRefResolvedIndex&(DeferredRefCount&) = 0
    CompilationReferencesDeferred& = DeferredRefCount&
END SUB

FUNCTION IsDeferred% (refName AS STRING, refType AS INTEGER)
    DIM i&

    FOR i& = 1 TO DeferredRefCount&
        IF DeferredRefType%(i&) = refType AND DeferredRefName$(i&) = refName THEN
            IF NOT DeferredRefResolved%(i&) THEN
                IsDeferred% = -1
                EXIT FUNCTION
            END IF
        END IF
    NEXT

    IsDeferred% = 0
END FUNCTION

SUB ResolveDeferredReferences
    DIM i&
    DIM resultFlags&
    DIM resultRef&
    DIM searchFlags&

    IF NOT SinglePassMode% THEN EXIT SUB

    FOR i& = 1 TO DeferredRefCount&
        IF DeferredRefResolved%(i&) THEN GOTO NextDeferredReference

        SELECT CASE DeferredRefType%(i&)
            CASE DEFREF_TYPE_LABEL
                searchFlags& = HASHFLAG_LABEL
            CASE DEFREF_TYPE_VARIABLE
                searchFlags& = HASHFLAG_VARIABLE
            CASE DEFREF_TYPE_FUNCTION
                searchFlags& = HASHFLAG_FUNCTION
            CASE DEFREF_TYPE_SUB
                searchFlags& = HASHFLAG_SUB
            CASE DEFREF_TYPE_UDT
                searchFlags& = HASHFLAG_UDT
            CASE DEFREF_TYPE_CONSTANT
                searchFlags& = HASHFLAG_CONSTANT
            CASE ELSE
                searchFlags& = 0
        END SELECT

        IF searchFlags& <> 0 THEN
            IF HashFind(DeferredRefName$(i&), searchFlags&, resultFlags&, resultRef&) THEN
                DeferredRefResolved%(i&) = -1
                DeferredRefResolvedIndex&(i&) = resultRef&
            END IF
        END IF

NextDeferredReference:
    NEXT

    CompilationReferencesResolved& = DeferredRefCount& - GetUnresolvedCount%
END SUB

SUB ReportUnresolvedReferences
END SUB

FUNCTION GetDeferredResolvedIndex& (refName AS STRING, refType AS INTEGER)
    DIM i&

    FOR i& = 1 TO DeferredRefCount&
        IF DeferredRefType%(i&) = refType AND DeferredRefName$(i&) = refName THEN
            IF DeferredRefResolved%(i&) THEN
                GetDeferredResolvedIndex& = DeferredRefResolvedIndex&(i&)
                EXIT FUNCTION
            END IF
        END IF
    NEXT

    GetDeferredResolvedIndex& = 0
END FUNCTION

FUNCTION CanAvoidRecompile% (triggerType AS STRING)
    IF NOT SinglePassMode% THEN
        CanAvoidRecompile% = 0
        EXIT FUNCTION
    END IF

    SELECT CASE triggerType
        CASE "LABEL", "VARIABLE", "FUNCTION", "SUB", "UDT", "CONSTANT"
            CanAvoidRecompile% = -1
        CASE ELSE
            CanAvoidRecompile% = 0
    END SELECT
END FUNCTION

SUB InitCompilationState
    CompilationLinesProcessed& = 0
    CompilationSymbolsDefined& = 0
    CompilationReferencesDeferred& = 0
    CompilationReferencesResolved& = 0
END SUB

SUB UpdateCompilationState (lineNum AS LONG)
    CompilationLinesProcessed& = lineNum
    CompilationReferencesDeferred& = DeferredRefCount&
END SUB

FUNCTION GetLinesProcessed&
    GetLinesProcessed& = CompilationLinesProcessed&
END FUNCTION

FUNCTION GetUnresolvedCount%
    DIM i&
    DIM unresolved%

    unresolved% = 0
    FOR i& = 1 TO DeferredRefCount&
        IF NOT DeferredRefResolved%(i&) THEN unresolved% = unresolved% + 1
    NEXT

    GetUnresolvedCount% = unresolved%
END FUNCTION

SUB PrintDeferredStats
    PRINT "Deferred References: "; DeferredRefCount&
    PRINT "Resolved: "; CompilationReferencesResolved&
    PRINT "Unresolved: "; GetUnresolvedCount%
END SUB

SUB CleanupDeferredReferences
    ClearDeferredReferences
    SinglePassMode% = 0
END SUB
