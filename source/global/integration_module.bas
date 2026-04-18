'===============================================================================
' QBNex Integration Module
'===============================================================================
' Central integration point that coordinates all compiler modules and provides
' a unified API for the main compiler loop.
'===============================================================================

'-------------------------------------------------------------------------------
' INTEGRATION STATE
'-------------------------------------------------------------------------------

TYPE IntegrationState
    isInitialized AS _BYTE
    isCompiling AS _BYTE
    sourceFile AS STRING * 256
    outputFile AS STRING * 256
    compileStartTime AS SINGLE
    compileEndTime AS SINGLE
    
    ' Feature flags
    useParallelProcessing AS _BYTE
    useSinglePass AS _BYTE
    useOptimizations AS _BYTE
    useCaching AS _BYTE
    
    ' Statistics
    totalLinesProcessed AS LONG
    totalSymbolsResolved AS LONG
    totalPhasesCompleted AS INTEGER
END TYPE

DIM SHARED Integration AS IntegrationState

'-------------------------------------------------------------------------------
' INITIALIZATION
'-------------------------------------------------------------------------------

SUB InitIntegrationModule
    Integration.isInitialized = -1
    Integration.isCompiling = 0
    Integration.sourceFile = ""
    Integration.outputFile = ""
    Integration.compileStartTime = 0
    Integration.compileEndTime = 0
    
    ' Enable all optimizations by default
    Integration.useParallelProcessing = -1
    Integration.useSinglePass = -1
    Integration.useOptimizations = -1
    Integration.useCaching = -1
    
    Integration.totalLinesProcessed = 0
    Integration.totalSymbolsResolved = 0
    Integration.totalPhasesCompleted = 0
    
    ' Initialize all subsystems in correct order
    InitErrorHandler
    InitOptimizationModule
    InitDeferredReferences
    InitCompilationState
    InitCompilerPhases
    InitParallelProcessing
    InitParser
    InitSymbolTable
    InitCodeGenerator
    
    ' Set error handler options
    SetVerboseMode -1
    SetMaxErrors 100
END SUB

SUB CleanupIntegrationModule
    Integration.isCompiling = 0
    
    ' Cleanup in reverse order
    CleanupCodeGenerator
    CleanupSymbolTable
    CleanupParser
    CleanupParallelProcessing
    CleanupDeferredReferences
    CleanupOptimizationModule
    CleanupErrorHandler
    
    Integration.isInitialized = 0
END SUB

'-------------------------------------------------------------------------------
' COMPILATION COORDINATION
'-------------------------------------------------------------------------------

FUNCTION IntegratedCompile% (sourcePath AS STRING, outputPath AS STRING)
    DIM success AS _BYTE
    
    success = -1
    Integration.isCompiling = -1
    Integration.sourceFile = sourcePath
    Integration.outputFile = outputPath
    Integration.compileStartTime = TIMER
    
    ' Phase 1: Initialization
    StartPhase PHASE_INITIALIZATION
    IF NOT IntegratedInitializeCompilation% THEN
        success = 0
        GOTO IntegratedCompileExit
    END IF
    EndPhase PHASE_INITIALIZATION
    Integration.totalPhasesCompleted = Integration.totalPhasesCompleted + 1
    
    ' Phase 2: Preprocessing
    StartPhase PHASE_PREPROCESSING
    IF NOT IntegratedPreprocess% THEN
        success = 0
        GOTO IntegratedCompileExit
    END IF
    EndPhase PHASE_PREPROCESSING
    Integration.totalPhasesCompleted = Integration.totalPhasesCompleted + 1
    
    ' Phase 3: Parsing (with caching)
    StartPhase PHASE_PARSING
    IF NOT IntegratedParse% THEN
        success = 0
        GOTO IntegratedCompileExit
    END IF
    EndPhase PHASE_PARSING
    Integration.totalPhasesCompleted = Integration.totalPhasesCompleted + 1
    
    ' Phase 4: Semantic Analysis (with parallel processing)
    StartPhase PHASE_SEMANTIC_ANALYSIS
    IF NOT IntegratedAnalyze% THEN
        success = 0
        GOTO IntegratedCompileExit
    END IF
    EndPhase PHASE_SEMANTIC_ANALYSIS
    Integration.totalPhasesCompleted = Integration.totalPhasesCompleted + 1
    
    ' Phase 5: Code Generation
    StartPhase PHASE_CODE_GENERATION
    IF NOT IntegratedGenerateCode% THEN
        success = 0
        GOTO IntegratedCompileExit
    END IF
    EndPhase PHASE_CODE_GENERATION
    Integration.totalPhasesCompleted = Integration.totalPhasesCompleted + 1
    
    ' Phase 6: Optimization
    StartPhase PHASE_OPTIMIZATION
    IF NOT IntegratedOptimize% THEN
        success = 0
        GOTO IntegratedCompileExit
    END IF
    EndPhase PHASE_OPTIMIZATION
    Integration.totalPhasesCompleted = Integration.totalPhasesCompleted + 1
    
    ' Phase 7: Linking
    StartPhase PHASE_LINKING
    IF NOT IntegratedLink% THEN
        success = 0
        GOTO IntegratedCompileExit
    END IF
    EndPhase PHASE_LINKING
    Integration.totalPhasesCompleted = Integration.totalPhasesCompleted + 1
    
    ' Phase 8: Finalization
    StartPhase PHASE_FINALIZATION
    IntegratedFinalize
    EndPhase PHASE_FINALIZATION
    Integration.totalPhasesCompleted = Integration.totalPhasesCompleted + 1
    
    IntegratedCompileExit:
    Integration.compileEndTime = TIMER
    Integration.isCompiling = 0
    IntegratedCompile% = success
END FUNCTION

'-------------------------------------------------------------------------------
' INTEGRATED PHASE FUNCTIONS
'-------------------------------------------------------------------------------

FUNCTION IntegratedInitializeCompilation%
    PushErrorContext "initialize compilation"

    ' Validate source file
    IF RTRIM$(Integration.sourceFile) = "" THEN
        ReportError ERR_NO_SOURCE_FILE, "No source file specified", 0, ""
        PopErrorContext
        IntegratedInitializeCompilation% = 0
        EXIT FUNCTION
    END IF
    
    IF NOT _FILEEXISTS(RTRIM$(Integration.sourceFile)) THEN
        ReportError ERR_FILE_NOT_FOUND, "Source file not found: " + RTRIM$(Integration.sourceFile), 0, ""
        PopErrorContext
        IntegratedInitializeCompilation% = 0
        EXIT FUNCTION
    END IF
    
    ' Set current file for error reporting
    SetCurrentFile RTRIM$(Integration.sourceFile)
    
    PopErrorContext
    IntegratedInitializeCompilation% = -1
END FUNCTION

FUNCTION IntegratedPreprocess%
    ' Preprocessing is handled by the main compiler loop
    ' This is a placeholder for future expansion
    IntegratedPreprocess% = -1
END FUNCTION

FUNCTION IntegratedParse%
    DIM astRoot AS LONG

    PushErrorContext "parse source file"
    
    ' Use parser module to parse source
    astRoot = ParseSourceFile%(RTRIM$(Integration.sourceFile))
    
    IF astRoot = 0 THEN
        ReportError ERR_INVALID_SYNTAX, "Failed to parse source file", 0, ""
        PopErrorContext
        IntegratedParse% = 0
        EXIT FUNCTION
    END IF
    
    ' Update statistics
    Integration.totalLinesProcessed = GetASTNodeCount%
    
    PopErrorContext
    IntegratedParse% = -1
END FUNCTION

FUNCTION IntegratedAnalyze%
    PushErrorContext "semantic analysis"

    ' Use parallel processing if enabled and beneficial
    IF Integration.useParallelProcessing AND IsParallelEnabled% THEN
        ' Use parallel symbol resolution
        ResolveSymbolsParallel
    END IF
    
    ' Always resolve deferred references
    ResolveDeferredReferences
    ReportUnresolvedReferences
    
    ' Check for unresolved symbols
    IF GetUnresolvedCount% > 0 THEN
        DIM i AS LONG
        FOR i = 1 TO GetUnresolvedCount%
            ReportError ERR_UNDEFINED_SYMBOL, "Unresolved symbol", 0, ""
        NEXT
        ' Don't fail immediately - let user see all errors
    END IF
    
    Integration.totalSymbolsResolved = SymbolCount
    
    PopErrorContext
    IntegratedAnalyze% = -1
END FUNCTION

FUNCTION IntegratedGenerateCode%
    ' Use parallel code generation if enabled
    IF Integration.useParallelProcessing AND IsParallelEnabled% THEN
        GenerateCodeParallel
    END IF
    
    IntegratedGenerateCode% = -1
END FUNCTION

FUNCTION IntegratedOptimize%
    ' Run optimization passes
    IF Integration.useOptimizations THEN
        IF Integration.useParallelProcessing AND IsParallelEnabled% THEN
            OptimizeParallel
        END IF
    END IF
    
    IntegratedOptimize% = -1
END FUNCTION

FUNCTION IntegratedLink%
    ' Linking is handled by the main compiler loop
    ' This is a placeholder for future expansion
    IntegratedLink% = -1
END FUNCTION

SUB IntegratedFinalize
    ' Print all accumulated reports
    IF HasErrors% THEN
        PrintAllErrors
    END IF
    
    ' Print performance reports
    IF Integration.useOptimizations THEN
        PrintPerformanceReport
        PrintCacheStats
    END IF
    
    IF Integration.useSinglePass THEN
        PrintDeferredStats
    END IF
    
    IF Integration.useParallelProcessing THEN
        PrintParallelMetrics
    END IF
    
    PrintPhaseReport
END SUB

'-------------------------------------------------------------------------------
' CONFIGURATION FUNCTIONS
'-------------------------------------------------------------------------------

SUB SetParallelProcessingEnabled (enabled AS _BYTE)
    Integration.useParallelProcessing = enabled
    SetParallelEnabled enabled
END SUB

SUB SetSinglePassEnabled (enabled AS _BYTE)
    Integration.useSinglePass = enabled
END SUB

SUB SetOptimizationsEnabled (enabled AS _BYTE)
    Integration.useOptimizations = enabled
END SUB

SUB SetCachingEnabled (enabled AS _BYTE)
    Integration.useCaching = enabled
END SUB

FUNCTION IsParallelProcessingEnabled%
    IsParallelProcessingEnabled% = Integration.useParallelProcessing
END FUNCTION

FUNCTION IsSinglePassEnabled%
    IsSinglePassEnabled% = Integration.useSinglePass
END FUNCTION

FUNCTION IsOptimizationsEnabled%
    IsOptimizationsEnabled% = Integration.useOptimizations
END FUNCTION

FUNCTION IsCachingEnabled%
    IsCachingEnabled% = Integration.useCaching
END FUNCTION

'-------------------------------------------------------------------------------
' STATISTICS FUNCTIONS
'-------------------------------------------------------------------------------

FUNCTION GetCompileTime!
    GetCompileTime! = Integration.compileEndTime - Integration.compileStartTime
END FUNCTION

FUNCTION GetTotalLinesProcessed%
    GetTotalLinesProcessed% = Integration.totalLinesProcessed
END FUNCTION

FUNCTION GetTotalSymbolsResolved%
    GetTotalSymbolsResolved% = Integration.totalSymbolsResolved
END FUNCTION

FUNCTION GetTotalPhasesCompleted%
    GetTotalPhasesCompleted% = Integration.totalPhasesCompleted
END FUNCTION

SUB PrintIntegrationSummary
    DIM elapsed AS SINGLE
    elapsed = GetCompileTime!
    
    PRINT ""
    PRINT "========================================"
    PRINT "   Integrated Compilation Summary"
    PRINT "========================================"
    PRINT "Source: "; RTRIM$(Integration.sourceFile)
    PRINT "Output: "; RTRIM$(Integration.outputFile)
    PRINT "Time: ";
    PRINT USING "###.###"; elapsed;
    PRINT " seconds"
    PRINT ""
    PRINT "Lines Processed: "; Integration.totalLinesProcessed
    PRINT "Symbols Resolved: "; Integration.totalSymbolsResolved
    PRINT "Phases Completed: "; Integration.totalPhasesCompleted; " / 8"
    PRINT ""
    PRINT "Features Enabled:"
    PRINT "  - Parallel Processing: "; IIF$(Integration.useParallelProcessing, "YES", "NO")
    PRINT "  - Single-Pass Mode: "; IIF$(Integration.useSinglePass, "YES", "NO")
    PRINT "  - Optimizations: "; IIF$(Integration.useOptimizations, "YES", "NO")
    PRINT "  - Caching: "; IIF$(Integration.useCaching, "YES", "NO")
    
    IF HasErrors% THEN
        PRINT ""
        PRINT "Status: FAILED ("; GetErrorCount%; " errors)"
    ELSEIF GetWarningCount% > 0 THEN
        PRINT ""
        PRINT "Status: SUCCESS ("; GetWarningCount%; " warnings)"
    ELSE
        PRINT ""
        PRINT "Status: SUCCESS"
    END IF
    
    PRINT "========================================"
END SUB
