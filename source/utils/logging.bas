'===============================================================================
' QBNex Logging Compatibility Module
'===============================================================================
' Stage0-compatible lightweight logger.
'===============================================================================

CONST LOG_LEVEL_NONE = 0
CONST LOG_LEVEL_FATAL = 1
CONST LOG_LEVEL_ERROR = 2
CONST LOG_LEVEL_WARNING = 3
CONST LOG_LEVEL_INFO = 4
CONST LOG_LEVEL_DEBUG = 5
CONST LOG_LEVEL_TRACE = 6

CONST LOG_CAT_GENERAL = 0
CONST LOG_CAT_PARSER = 1
CONST LOG_CAT_SCANNER = 2
CONST LOG_CAT_SEMANTIC = 3
CONST LOG_CAT_CODEGEN = 4
CONST LOG_CAT_OPTIMIZER = 5
CONST LOG_CAT_LINKER = 6
CONST LOG_CAT_MEMORY = 7
CONST LOG_CAT_PERFORMANCE = 8
CONST LOG_CAT_IO = 9
CONST LOG_CAT_SYSTEM = 10
CONST MAX_LOG_CATEGORIES = 11

DIM SHARED LogInitialized AS _BYTE
DIM SHARED LogConsoleLevel AS INTEGER
DIM SHARED LogFileLevel AS INTEGER
DIM SHARED LogCategoryEnabled(0 TO MAX_LOG_CATEGORIES - 1) AS INTEGER

SUB InitLogging
    DIM i AS INTEGER

    LogConsoleLevel = LOG_LEVEL_INFO
    LogFileLevel = LOG_LEVEL_DEBUG
    FOR i = 0 TO MAX_LOG_CATEGORIES - 1
        LogCategoryEnabled(i) = -1
    NEXT
    LogInitialized = -1
END SUB

SUB CleanupLogging
    LogInitialized = 0
END SUB

SUB FlushLogBuffer
END SUB

SUB LogMessage (level AS INTEGER, category AS INTEGER, message AS STRING, sourceFile AS STRING, lineNum AS INTEGER)
    IF NOT LogInitialized THEN InitLogging
    IF category >= 0 AND category < MAX_LOG_CATEGORIES THEN
        IF NOT LogCategoryEnabled(category) THEN EXIT SUB
    END IF
END SUB

SUB LogFatal (message AS STRING, sourceFile AS STRING, lineNum AS INTEGER)
    LogMessage LOG_LEVEL_FATAL, LOG_CAT_GENERAL, message, sourceFile, lineNum
END SUB

SUB LogError (message AS STRING, sourceFile AS STRING, lineNum AS INTEGER)
    LogMessage LOG_LEVEL_ERROR, LOG_CAT_GENERAL, message, sourceFile, lineNum
END SUB

SUB LogWarning (message AS STRING, sourceFile AS STRING, lineNum AS INTEGER)
    LogMessage LOG_LEVEL_WARNING, LOG_CAT_GENERAL, message, sourceFile, lineNum
END SUB

SUB LogInfo (message AS STRING, sourceFile AS STRING, lineNum AS INTEGER)
    LogMessage LOG_LEVEL_INFO, LOG_CAT_GENERAL, message, sourceFile, lineNum
END SUB

SUB LogDebug (message AS STRING, sourceFile AS STRING, lineNum AS INTEGER)
    LogMessage LOG_LEVEL_DEBUG, LOG_CAT_GENERAL, message, sourceFile, lineNum
END SUB

SUB LogTrace (message AS STRING, sourceFile AS STRING, lineNum AS INTEGER)
    LogMessage LOG_LEVEL_TRACE, LOG_CAT_GENERAL, message, sourceFile, lineNum
END SUB

SUB LogCompilerPhase (phaseName AS STRING, message AS STRING)
    LogMessage LOG_LEVEL_INFO, LOG_CAT_GENERAL, phaseName + ": " + message, "", 0
END SUB

SUB LogPerformance (message AS STRING, sourceFile AS STRING, lineNum AS INTEGER)
    LogMessage LOG_LEVEL_INFO, LOG_CAT_PERFORMANCE, message, sourceFile, lineNum
END SUB

SUB SetLogLevel (consoleLevel AS INTEGER, fileLevel AS INTEGER)
    LogConsoleLevel = consoleLevel
    LogFileLevel = fileLevel
END SUB

SUB EnableLogCategory (category AS INTEGER)
    IF category >= 0 AND category < MAX_LOG_CATEGORIES THEN LogCategoryEnabled(category) = -1
END SUB

SUB DisableLogCategory (category AS INTEGER)
    IF category >= 0 AND category < MAX_LOG_CATEGORIES THEN LogCategoryEnabled(category) = 0
END SUB

FUNCTION IsLogCategoryEnabled% (category AS INTEGER)
    IF category >= 0 AND category < MAX_LOG_CATEGORIES THEN
        IsLogCategoryEnabled% = LogCategoryEnabled(category)
    ELSE
        IsLogCategoryEnabled% = 0
    END IF
END FUNCTION
