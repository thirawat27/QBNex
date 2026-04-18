'===============================================================================
' QBNex Incremental Compilation Compatibility Module
'===============================================================================
' Stage0-compatible dependency tracker. Keeps the public API stable while using
' a conservative implementation that avoids advanced UDT features during
' bootstrap/self-host builds.
'===============================================================================

CONST DEP_TYPE_INCLUDE = 1
CONST DEP_TYPE_IMPORT = 2
CONST DEP_TYPE_LIBRARY = 3
CONST DEP_TYPE_RESOURCE = 4

CONST INITIAL_DEP_CAPACITY = 256
CONST MAX_DEPENDENCIES_PER_FILE = 32

DIM SHARED IncrementalEnabled AS _BYTE
DIM SHARED DepGraphBuilt AS _BYTE
DIM SHARED FileDepCount%
DIM SHARED FileDepPath$(1 TO INITIAL_DEP_CAPACITY)
DIM SHARED FileDepLastModified#(1 TO INITIAL_DEP_CAPACITY)
DIM SHARED FileDepSize&(1 TO INITIAL_DEP_CAPACITY)
DIM SHARED FileDepModified%(1 TO INITIAL_DEP_CAPACITY)
DIM SHARED FileDepNeedsRecompile%(1 TO INITIAL_DEP_CAPACITY)
DIM SHARED FileDepCompiled%(1 TO INITIAL_DEP_CAPACITY)
DIM SHARED FileDepDependencyCount%(1 TO INITIAL_DEP_CAPACITY)
DIM SHARED FileDepDependency$(1 TO INITIAL_DEP_CAPACITY, 1 TO MAX_DEPENDENCIES_PER_FILE)
DIM SHARED FileDepDependencyType%(1 TO INITIAL_DEP_CAPACITY, 1 TO MAX_DEPENDENCIES_PER_FILE)

SUB InitIncrementalCompilation
    IncrementalEnabled = -1
    DepGraphBuilt = 0
    FileDepCount% = 0
    CleanupIncrementalState
END SUB

SUB CleanupIncrementalCompilation
    CleanupIncrementalState
    FileDepCount% = 0
    DepGraphBuilt = 0
END SUB

SUB CleanupIncrementalState
    DIM i%
    DIM j%

    FOR i% = 1 TO INITIAL_DEP_CAPACITY
        FileDepPath$(i%) = ""
        FileDepLastModified#(i%) = 0
        FileDepSize&(i%) = 0
        FileDepModified%(i%) = 0
        FileDepNeedsRecompile%(i%) = 0
        FileDepCompiled%(i%) = 0
        FileDepDependencyCount%(i%) = 0
        FOR j% = 1 TO MAX_DEPENDENCIES_PER_FILE
            FileDepDependency$(i%, j%) = ""
            FileDepDependencyType%(i%, j%) = 0
        NEXT
    NEXT
END SUB

FUNCTION GetFileDepIndex% (filePath AS STRING)
    DIM existing%

    existing% = FindFileDepIndex%(filePath)
    IF existing% > 0 THEN
        GetFileDepIndex% = existing%
        EXIT FUNCTION
    END IF

    IF FileDepCount% >= INITIAL_DEP_CAPACITY THEN
        GetFileDepIndex% = 0
        EXIT FUNCTION
    END IF

    FileDepCount% = FileDepCount% + 1
    FileDepPath$(FileDepCount%) = filePath
    FileDepNeedsRecompile%(FileDepCount%) = -1
    GetFileDepIndex% = FileDepCount%
END FUNCTION

FUNCTION FindFileDepIndex% (filePath AS STRING)
    DIM i%

    FOR i% = 1 TO FileDepCount%
        IF RTRIM$(FileDepPath$(i%)) = filePath THEN
            FindFileDepIndex% = i%
            EXIT FUNCTION
        END IF
    NEXT

    FindFileDepIndex% = 0
END FUNCTION

SUB AddFileDependency (sourceFile AS STRING, depFile AS STRING, depType AS INTEGER)
    DIM sourceIdx%
    DIM i%

    sourceIdx% = GetFileDepIndex%(sourceFile)
    IF sourceIdx% = 0 THEN EXIT SUB

    FOR i% = 1 TO FileDepDependencyCount%(sourceIdx%)
        IF RTRIM$(FileDepDependency$(sourceIdx%, i%)) = depFile THEN EXIT SUB
    NEXT

    IF FileDepDependencyCount%(sourceIdx%) >= MAX_DEPENDENCIES_PER_FILE THEN EXIT SUB

    FileDepDependencyCount%(sourceIdx%) = FileDepDependencyCount%(sourceIdx%) + 1
    FileDepDependency$(sourceIdx%, FileDepDependencyCount%(sourceIdx%)) = depFile
    FileDepDependencyType%(sourceIdx%, FileDepDependencyCount%(sourceIdx%)) = depType
END SUB

SUB BuildDependencyGraph (sourceFile AS STRING)
    DIM fileNo%
    DIM lineText AS STRING
    DIM upperLine AS STRING
    DIM depFile AS STRING

    IF NOT IncrementalEnabled THEN EXIT SUB
    IF NOT _FILEEXISTS(sourceFile) THEN EXIT SUB

    GetFileDepIndex% sourceFile
    fileNo% = FREEFILE
    OPEN sourceFile FOR INPUT AS #fileNo%

    DO WHILE NOT EOF(fileNo%)
        LINE INPUT #fileNo%, lineText
        upperLine = UCASE$(LTRIM$(RTRIM$(lineText)))

        IF LEFT$(upperLine, 8) = "$INCLUDE" THEN
            depFile = ExtractIncludeFile$(lineText)
            IF LEN(depFile) > 0 THEN AddFileDependency sourceFile, depFile, DEP_TYPE_INCLUDE
        END IF

        IF LEFT$(upperLine, 7) = "$IMPORT" THEN
            depFile = ExtractImportFile$(lineText)
            IF LEN(depFile) > 0 THEN AddFileDependency sourceFile, depFile, DEP_TYPE_IMPORT
        END IF
    LOOP

    CLOSE #fileNo%
END SUB

FUNCTION ExtractIncludeFile$ (lineText AS STRING)
    DIM startPos%
    DIM endPos%

    startPos% = INSTR(lineText, CHR$(34))
    IF startPos% > 0 THEN
        endPos% = INSTR(startPos% + 1, lineText, CHR$(34))
        IF endPos% > startPos% THEN
            ExtractIncludeFile$ = MID$(lineText, startPos% + 1, endPos% - startPos% - 1)
            EXIT FUNCTION
        END IF
    END IF

    ExtractIncludeFile$ = ""
END FUNCTION

FUNCTION ExtractImportFile$ (lineText AS STRING)
    DIM startPos%
    DIM endPos%

    startPos% = INSTR(lineText, CHR$(34))
    IF startPos% > 0 THEN
        endPos% = INSTR(startPos% + 1, lineText, CHR$(34))
        IF endPos% > startPos% THEN
            ExtractImportFile$ = MID$(lineText, startPos% + 1, endPos% - startPos% - 1)
            EXIT FUNCTION
        END IF
    END IF

    lineText = LTRIM$(RTRIM$(lineText))
    endPos% = LEN(lineText)
    DO WHILE endPos% > 0
        IF MID$(lineText, endPos%, 1) = " " THEN EXIT DO
        endPos% = endPos% - 1
    LOOP
    IF endPos% > 0 THEN
        ExtractImportFile$ = LTRIM$(MID$(lineText, endPos% + 1))
    ELSE
        ExtractImportFile$ = ""
    END IF
END FUNCTION

FUNCTION NeedsRecompile% (fileIdx AS INTEGER)
    DIM i%
    DIM depIdx%
    DIM sourceFile AS STRING
    DIM currentMod#
    DIM currentSize&

    IF NOT IncrementalEnabled THEN
        NeedsRecompile% = -1
        EXIT FUNCTION
    END IF

    IF fileIdx < 1 OR fileIdx > FileDepCount% THEN
        NeedsRecompile% = -1
        EXIT FUNCTION
    END IF

    sourceFile = RTRIM$(FileDepPath$(fileIdx))
    IF LEN(sourceFile) = 0 THEN
        NeedsRecompile% = -1
        EXIT FUNCTION
    END IF

    currentMod# = _FILEDATETIME(sourceFile)
    currentSize& = _FILEEXISTS(sourceFile)

    IF currentMod# > FileDepLastModified#(fileIdx) OR currentSize& <> FileDepSize&(fileIdx) THEN
        FileDepModified%(fileIdx) = -1
        FileDepNeedsRecompile%(fileIdx) = -1
        NeedsRecompile% = -1
        EXIT FUNCTION
    END IF

    FOR i% = 1 TO FileDepDependencyCount%(fileIdx)
        depIdx% = FindFileDepIndex%(RTRIM$(FileDepDependency$(fileIdx, i%)))
        IF depIdx% > 0 THEN
            IF NeedsRecompile%(depIdx%) THEN
                FileDepNeedsRecompile%(fileIdx) = -1
                NeedsRecompile% = -1
                EXIT FUNCTION
            END IF
        END IF
    NEXT

    FileDepNeedsRecompile%(fileIdx) = 0
    NeedsRecompile% = 0
END FUNCTION

SUB AnalyzeIncrementalCompilation (mainSource AS STRING)
    DIM mainIdx%

    IF NOT IncrementalEnabled THEN EXIT SUB

    BuildDependencyGraph mainSource
    mainIdx% = GetFileDepIndex%(mainSource)
    IF mainIdx% = 0 THEN EXIT SUB

    IF NeedsRecompile%(mainIdx%) THEN PropagateRecompileStatus mainIdx%
    DepGraphBuilt = -1
END SUB

SUB PropagateRecompileStatus (modifiedIdx AS INTEGER)
    DIM i%
    DIM j%
    DIM modifiedFile AS STRING

    IF modifiedIdx < 1 OR modifiedIdx > FileDepCount% THEN EXIT SUB
    modifiedFile = RTRIM$(FileDepPath$(modifiedIdx))

    FOR i% = 1 TO FileDepCount%
        FOR j% = 1 TO FileDepDependencyCount%(i%)
            IF RTRIM$(FileDepDependency$(i%, j%)) = modifiedFile THEN
                FileDepNeedsRecompile%(i%) = -1
            END IF
        NEXT
    NEXT
END SUB

SUB GetRecompileList (fileList() AS STRING, count AS INTEGER)
    DIM i%

    count = 0
    FOR i% = 1 TO FileDepCount%
        IF FileDepNeedsRecompile%(i%) THEN
            count = count + 1
            fileList(count) = RTRIM$(FileDepPath$(i%))
        END IF
    NEXT
END SUB

SUB SaveDependencyGraph (cacheFile AS STRING)
    DIM fileNo%
    DIM i%
    DIM j%

    fileNo% = FREEFILE
    OPEN cacheFile FOR OUTPUT AS #fileNo%
    PRINT #fileNo%, "QBNEX_DEP_GRAPH_V1"
    PRINT #fileNo%, FileDepCount%

    FOR i% = 1 TO FileDepCount%
        PRINT #fileNo%, RTRIM$(FileDepPath$(i%))
        PRINT #fileNo%, FileDepLastModified#(i%)
        PRINT #fileNo%, FileDepSize&(i%)
        PRINT #fileNo%, FileDepDependencyCount%(i%)
        FOR j% = 1 TO FileDepDependencyCount%(i%)
            PRINT #fileNo%, RTRIM$(FileDepDependency$(i%, j%))
            PRINT #fileNo%, FileDepDependencyType%(i%, j%)
        NEXT
    NEXT

    CLOSE #fileNo%
END SUB

FUNCTION LoadDependencyGraph% (cacheFile AS STRING)
    DIM fileNo%
    DIM header AS STRING
    DIM i%
    DIM j%
    DIM depCount%

    IF NOT _FILEEXISTS(cacheFile) THEN
        LoadDependencyGraph% = 0
        EXIT FUNCTION
    END IF

    CleanupIncrementalState

    fileNo% = FREEFILE
    OPEN cacheFile FOR INPUT AS #fileNo%
    LINE INPUT #fileNo%, header
    IF header <> "QBNEX_DEP_GRAPH_V1" THEN
        CLOSE #fileNo%
        LoadDependencyGraph% = 0
        EXIT FUNCTION
    END IF

    INPUT #fileNo%, FileDepCount%
    IF FileDepCount% < 0 THEN FileDepCount% = 0
    IF FileDepCount% > INITIAL_DEP_CAPACITY THEN FileDepCount% = INITIAL_DEP_CAPACITY

    FOR i% = 1 TO FileDepCount%
        LINE INPUT #fileNo%, FileDepPath$(i%)
        INPUT #fileNo%, FileDepLastModified#(i%)
        INPUT #fileNo%, FileDepSize&(i%)
        INPUT #fileNo%, depCount%
        IF depCount% < 0 THEN depCount% = 0
        IF depCount% > MAX_DEPENDENCIES_PER_FILE THEN depCount% = MAX_DEPENDENCIES_PER_FILE
        FileDepDependencyCount%(i%) = depCount%

        FOR j% = 1 TO depCount%
            LINE INPUT #fileNo%, FileDepDependency$(i%, j%)
            INPUT #fileNo%, FileDepDependencyType%(i%, j%)
        NEXT
    NEXT

    CLOSE #fileNo%
    LoadDependencyGraph% = -1
END FUNCTION

SUB UpdateFileTimestamps
    DIM i%
    DIM sourceFile AS STRING

    FOR i% = 1 TO FileDepCount%
        IF FileDepNeedsRecompile%(i%) OR FileDepModified%(i%) THEN
            sourceFile = RTRIM$(FileDepPath$(i%))
            FileDepLastModified#(i%) = _FILEDATETIME(sourceFile)
            FileDepSize&(i%) = _FILEEXISTS(sourceFile)
            FileDepModified%(i%) = 0
            FileDepNeedsRecompile%(i%) = 0
        END IF
    NEXT
END SUB

SUB PrintIncrementalStats
    DIM i%
    DIM recompileCount%

    FOR i% = 1 TO FileDepCount%
        IF FileDepNeedsRecompile%(i%) THEN recompileCount% = recompileCount% + 1
    NEXT

    PRINT "=== Incremental Compilation Stats ==="
    PRINT "Tracked Files: "; FileDepCount%
    PRINT "Files to Recompile: "; recompileCount%
    PRINT "====================================="
END SUB

SUB SetIncrementalEnabled (enabled AS _BYTE)
    IncrementalEnabled = enabled
END SUB

FUNCTION IsIncrementalEnabled%
    IsIncrementalEnabled% = IncrementalEnabled
END FUNCTION

SUB ForceFullRebuild
    DIM i%

    FOR i% = 1 TO FileDepCount%
        FileDepNeedsRecompile%(i%) = -1
        FileDepCompiled%(i%) = 0
    NEXT
END SUB
