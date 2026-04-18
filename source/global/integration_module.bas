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
    generatedCodePath AS STRING * 256
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
    Integration.useParallelProcessing = 0
    Integration.useSinglePass = -1
    Integration.useOptimizations = 0
    Integration.useCaching = -1
    
    Integration.totalLinesProcessed = 0
    Integration.totalSymbolsResolved = 0
    Integration.totalPhasesCompleted = 0
    Integration.generatedCodePath = ""
    
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
        PopErrorContext
        IntegratedAnalyze% = 0
        EXIT FUNCTION
    END IF
    
    Integration.totalSymbolsResolved = SymbolCount
    
    PopErrorContext
    IntegratedAnalyze% = -1
END FUNCTION

FUNCTION IntegratedGenerateCode%
    DIM targetOutput AS STRING
    DIM astRoot AS LONG

    PushErrorContext "code generation"

    astRoot = GetASTRoot%
    IF astRoot = 0 THEN
        ReportError ERR_CODEGEN_FAILED, "Cannot generate code without a parsed AST", 0, ""
        PopErrorContext
        IntegratedGenerateCode% = 0
        EXIT FUNCTION
    END IF

    targetOutput = RTRIM$(Integration.outputFile)
    IF targetOutput = "" THEN
        targetOutput = "internal/temp/integrated_output.cpp"
    END IF

    GenerateCodeFromAST astRoot, targetOutput
    Integration.generatedCodePath = targetOutput

    IF GetOutputLineCount% <= 0 THEN
        ReportError ERR_CODEGEN_FAILED, "Code generator produced no output", 0, ""
        PopErrorContext
        IntegratedGenerateCode% = 0
        EXIT FUNCTION
    END IF
    
    PopErrorContext
    IntegratedGenerateCode% = -1
END FUNCTION

FUNCTION IntegratedOptimize%
    ' Run optimization passes
    IF Integration.useOptimizations THEN
        ReportError ERR_UNSUPPORTED_FEATURE, "Integrated optimization pass is not implemented yet", 0, ""
        IntegratedOptimize% = 0
        EXIT FUNCTION
    END IF
    
    IntegratedOptimize% = -1
END FUNCTION

FUNCTION IntegratedLink%
    DIM targetOutput AS STRING
    DIM lowerOutput AS STRING

    targetOutput = RTRIM$(Integration.outputFile)
    IF targetOutput = "" THEN targetOutput = RTRIM$(Integration.generatedCodePath)
    lowerOutput = LCASE$(targetOutput)

    IF lowerOutput = "" THEN
        ReportError ERR_LINK_ERROR, "No output path available for integrated link step", 0, ""
        IntegratedLink% = 0
        EXIT FUNCTION
    END IF

    IF RIGHT$(lowerOutput, 4) = ".cpp" OR RIGHT$(lowerOutput, 2) = ".c" OR RIGHT$(lowerOutput, 4) = ".cxx" OR RIGHT$(lowerOutput, 3) = ".cc" THEN
        IntegratedLink% = -1
        EXIT FUNCTION
    END IF

    ReportError ERR_UNSUPPORTED_FEATURE, "Integrated native linking is not implemented yet; use a .cpp output path or the legacy compiler path", 0, ""
    IntegratedLink% = 0
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
    PRINT "  - Parallel Processing: "; EnabledStatus$(Integration.useParallelProcessing)
    PRINT "  - Single-Pass Mode: "; EnabledStatus$(Integration.useSinglePass)
    PRINT "  - Optimizations: "; EnabledStatus$(Integration.useOptimizations)
    PRINT "  - Caching: "; EnabledStatus$(Integration.useCaching)
    
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

FUNCTION EnabledStatus$ (enabled AS _BYTE)
    IF enabled THEN
        EnabledStatus$ = "YES"
    ELSE
        EnabledStatus$ = "NO"
    END IF
END FUNCTION
