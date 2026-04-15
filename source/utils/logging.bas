'===============================================================================
' QBNex Comprehensive Logging Module
'===============================================================================
' Provides detailed logging capabilities for debugging and monitoring
' the compilation process.
'
' Features:
' - Multiple log levels (DEBUG, INFO, WARNING, ERROR, FATAL)
' - Category-based logging
' - File and console output
' - Log rotation
' - Performance tracking
'===============================================================================

'-------------------------------------------------------------------------------
' LOG LEVELS
'-------------------------------------------------------------------------------

CONST LOG_LEVEL_NONE = 0
CONST LOG_LEVEL_FATAL = 1
CONST LOG_LEVEL_ERROR = 2
CONST LOG_LEVEL_WARNING = 3
CONST LOG_LEVEL_INFO = 4
CONST LOG_LEVEL_DEBUG = 5
CONST LOG_LEVEL_TRACE = 6

'-------------------------------------------------------------------------------
' LOG CATEGORIES
'-------------------------------------------------------------------------------

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

'-------------------------------------------------------------------------------
' LOG ENTRY TYPE
'-------------------------------------------------------------------------------

TYPE LogEntry
    timestamp AS STRING * 24
    level AS INTEGER
    category AS INTEGER
    sourceFile AS STRING * 64
    lineNumber AS INTEGER
    functionName AS STRING * 64
    message AS STRING * 512
    threadId AS LONG
END TYPE

'-------------------------------------------------------------------------------
' LOG CONFIGURATION
'-------------------------------------------------------------------------------

TYPE LogConfig
    'Level settings
    consoleLevel AS INTEGER
    fileLevel AS INTEGER
    
    'Category filters
    categoryEnabled(0 TO MAX_LOG_CATEGORIES - 1) AS _BYTE
    
    'Output settings
    logToConsole AS _BYTE
    logToFile AS _BYTE
    logFilePath AS STRING * 260
    logFileMaxSize AS LONG  'In KB
    logFileMaxCount AS INTEGER
    
    'Format settings
    showTimestamp AS _BYTE
    showLevel AS _BYTE
    showCategory AS _BYTE
    showSource AS _BYTE
    showThread AS _BYTE
    
    'Buffer settings
    bufferSize AS INTEGER
    flushInterval AS INTEGER  'In milliseconds
    
    'Debug settings
    enableAssertions AS _BYTE
    breakOnError AS _BYTE
END TYPE

'-------------------------------------------------------------------------------
' LOG STATISTICS
'-------------------------------------------------------------------------------

TYPE LogStats
    entriesLogged AS _UNSIGNED _INTEGER64
    entriesDropped AS _UNSIGNED _INTEGER64
    bytesWritten AS _UNSIGNED _INTEGER64
    filesCreated AS INTEGER
    filesRotated AS INTEGER
    errors AS LONG
    
    'Level breakdown
    fatalCount AS LONG
    errorCount AS LONG
    warningCount AS LONG
    infoCount AS LONG
    debugCount AS LONG
    traceCount AS LONG
END TYPE

'-------------------------------------------------------------------------------
' MODULE STATE
'-------------------------------------------------------------------------------

DIM SHARED LogConfiguration AS LogConfig
DIM SHARED LogStatistics AS LogStats
DIM SHARED LogFileNumber AS INTEGER
DIM SHARED LogFileSize AS LONG
DIM SHARED LogInitialized AS _BYTE
DIM SHARED LogBuffer() AS LogEntry
DIM SHARED LogBufferCount AS INTEGER
DIM SHARED LogBufferCapacity AS INTEGER

CONST DEFAULT_BUFFER_SIZE = 1000
CONST DEFAULT_LOG_FILE_MAX_SIZE = 10240 '10MB

'-------------------------------------------------------------------------------
' INITIALIZATION
'-------------------------------------------------------------------------------

SUB InitLogging
    'Default configuration
    WITH LogConfiguration
        .consoleLevel = LOG_LEVEL_INFO
        .fileLevel = LOG_LEVEL_DEBUG
        
        .logToConsole = -1
        .logToFile = 0
        .logFilePath = "internal/logs/qbnex.log"
        .logFileMaxSize = DEFAULT_LOG_FILE_MAX_SIZE
        .logFileMaxCount = 5
        
        .showTimestamp = -1
        .showLevel = -1
        .showCategory = 0
        .showSource = 0
        .showThread = 0
        
        .bufferSize = DEFAULT_BUFFER_SIZE
        .flushInterval = 1000
        
        .enableAssertions = -1
        .breakOnError = 0
    END WITH
    
    'Enable all categories by default
    DIM i AS INTEGER
    FOR i = 0 TO MAX_LOG_CATEGORIES - 1
        LogConfiguration.categoryEnabled(i) = -1
    NEXT
    
    'Initialize buffer
    LogBufferCapacity = DEFAULT_BUFFER_SIZE
    REDIM LogBuffer(1 TO LogBufferCapacity) AS LogEntry
    LogBufferCount = 0
    
    'Clear statistics
    WITH LogStatistics
        .entriesLogged = 0
        .entriesDropped = 0
        .bytesWritten = 0
        .filesCreated = 0
        .filesRotated = 0
        .errors = 0
        .fatalCount = 0
        .errorCount = 0
        .warningCount = 0
        .infoCount = 0
        .debugCount = 0
        .traceCount = 0
    END WITH
    
    LogFileNumber = 0
    LogFileSize = 0
    LogInitialized = -1
END SUB

SUB CleanupLogging
    FlushLogBuffer
    
    IF LogFileNumber > 0 THEN
        CLOSE #LogFileNumber
        LogFileNumber = 0
    END IF
    
    ERASE LogBuffer
    LogBufferCount = 0
    LogBufferCapacity = 0
    LogInitialized = 0
END SUB

'-------------------------------------------------------------------------------
' LOG FILE MANAGEMENT
'-------------------------------------------------------------------------------

SUB OpenLogFile
    IF NOT LogConfiguration.logToFile THEN EXIT SUB
    
    'Create log directory if needed
    DIM logDir AS STRING
    logDir = GetDirectoryFromPath$(RTRIM$(LogConfiguration.logFilePath))
    IF NOT _DIREXISTS(logDir) THEN
        MKDIR logDir
    END IF
    
    'Check if rotation needed
    IF _FILEEXISTS(RTRIM$(LogConfiguration.logFilePath)) THEN
        DIM fileSize AS LONG
        fileSize = _FILEEXISTS(RTRIM$(LogConfiguration.logFilePath)) 'Returns file size
        
        IF fileSize > LogConfiguration.logFileMaxSize * 1024 THEN
            RotateLogFiles
        END IF
    END IF
    
    'Open log file
    LogFileNumber = FREEFILE
    OPEN RTRIM$(LogConfiguration.logFilePath) FOR APPEND AS #LogFileNumber
    
    LogStatistics.filesCreated = LogStatistics.filesCreated + 1
END SUB

SUB RotateLogFiles
    DIM basePath AS STRING
    DIM ext AS STRING
    DIM i AS INTEGER
    DIM oldFile AS STRING
    DIM newFile AS STRING
    
    basePath = RTRIM$(LogConfiguration.logFilePath)
    
    'Close current file
    IF LogFileNumber > 0 THEN
        CLOSE #LogFileNumber
        LogFileNumber = 0
    END IF
    
    'Rotate existing files
    FOR i = LogConfiguration.logFileMaxCount - 1 TO 1 STEP -1
        oldFile = basePath + "." + LTRIM$(STR$(i))
        newFile = basePath + "." + LTRIM$(STR$(i + 1))
        
        IF _FILEEXISTS(oldFile) THEN
            IF _FILEEXISTS(newFile) THEN KILL newFile
            NAME oldFile AS newFile
        END IF
    NEXT
    
    'Move current file to .1
    IF _FILEEXISTS(basePath) THEN
        newFile = basePath + ".1"
        IF _FILEEXISTS(newFile) THEN KILL newFile
        NAME basePath AS newFile
    END IF
    
    LogStatistics.filesRotated = LogStatistics.filesRotated + 1
END SUB

FUNCTION GetDirectoryFromPath$ (filePath AS STRING)
    DIM i AS INTEGER
    DIM lastSlash AS INTEGER
    
    lastSlash = 0
    FOR i = LEN(filePath) TO 1 STEP -1
        IF MID$(filePath, i, 1) = "\" OR MID$(filePath, i, 1) = "/" THEN
            lastSlash = i
            EXIT FOR
        END IF
    NEXT
    
    IF lastSlash > 0 THEN
        GetDirectoryFromPath$ = LEFT$(filePath, lastSlash - 1)
    ELSE
        GetDirectoryFromPath$ = ""
    END IF
END FUNCTION

'-------------------------------------------------------------------------------
' CORE LOGGING FUNCTIONS
'-------------------------------------------------------------------------------

SUB LogMessage (level AS INTEGER, category AS INTEGER, message AS STRING, sourceFile AS STRING, lineNum AS INTEGER)
    DIM entry AS LogEntry
    
    IF NOT LogInitialized THEN InitLogging
    
    'Check level filters
    IF level > LogConfiguration.consoleLevel AND level > LogConfiguration.fileLevel THEN EXIT SUB
    
    'Check category filter
    IF category < 0 OR category >= MAX_LOG_CATEGORIES THEN
        category = LOG_CAT_GENERAL
    END IF
    IF NOT LogConfiguration.categoryEnabled(category) THEN EXIT SUB
    
    'Build log entry
    entry.timestamp = FORMAT$(DATE$, "YYYY-MM-DD") + " " + TIME$
    entry.level = level
    entry.category = category
    entry.sourceFile = sourceFile
    entry.lineNumber = lineNum
    entry.functionName = ""
    entry.message = message
    entry.threadId = 0
    
    'Add to buffer
    AddToLogBuffer entry
    
    'Update statistics
    UpdateLogStats level
END SUB

SUB AddToLogBuffer (entry AS LogEntry)
    IF LogBufferCount >= LogBufferCapacity THEN
        FlushLogBuffer
    END IF
    
    LogBufferCount = LogBufferCount + 1
    LogBuffer(LogBufferCount) = entry
END SUB

SUB UpdateLogStats (level AS INTEGER)
    LogStatistics.entriesLogged = LogStatistics.entriesLogged + 1
    
    SELECT CASE level
        CASE LOG_LEVEL_FATAL: LogStatistics.fatalCount = LogStatistics.fatalCount + 1
        CASE LOG_LEVEL_ERROR: LogStatistics.errorCount = LogStatistics.errorCount + 1
        CASE LOG_LEVEL_WARNING: LogStatistics.warningCount = LogStatistics.warningCount + 1
        CASE LOG_LEVEL_INFO: LogStatistics.infoCount = LogStatistics.infoCount + 1
        CASE LOG_LEVEL_DEBUG: LogStatistics.debugCount = LogStatistics.debugCount + 1
        CASE LOG_LEVEL_TRACE: LogStatistics.traceCount = LogStatistics.traceCount + 1
    END SELECT
END SUB

SUB FlushLogBuffer
    DIM i AS INTEGER
    DIM entry AS LogEntry
    
    IF LogBufferCount = 0 THEN EXIT SUB
    
    'Open file if needed
    IF LogConfiguration.logToFile AND LogFileNumber = 0 THEN
        OpenLogFile
    END IF
    
    'Flush each entry
    FOR i = 1 TO LogBufferCount
        entry = LogBuffer(i)
        
        'Console output
        IF LogConfiguration.logToConsole AND entry.level <= LogConfiguration.consoleLevel THEN
            PrintLogEntry entry, 0 '0 = console
        END IF
        
        'File output
        IF LogConfiguration.logToFile AND entry.level <= LogConfiguration.fileLevel THEN
            PrintLogEntry entry, LogFileNumber
        END IF
    NEXT
    
    'Clear buffer
    LogBufferCount = 0
END SUB

SUB PrintLogEntry (entry AS LogEntry, outputFile AS INTEGER)
    DIM logLine AS STRING
    
    'Build log line based on format settings
    logLine = ""
    
    IF LogConfiguration.showTimestamp THEN
        logLine = logLine + "[" + RTRIM$(entry.timestamp) + "] "
    END IF
    
    IF LogConfiguration.showLevel THEN
        logLine = logLine + "[" + GetLevelString$(entry.level) + "] "
    END IF
    
    IF LogConfiguration.showCategory THEN
        logLine = logLine + "[" + GetCategoryString$(entry.category) + "] "
    END IF
    
    IF LogConfiguration.showSource AND entry.sourceFile <> "" THEN
        logLine = logLine + RTRIM$(entry.sourceFile)
        IF entry.lineNumber > 0 THEN
            logLine = logLine + ":" + LTRIM$(STR$(entry.lineNumber))
        END IF
        logLine = logLine + " "
    END IF
    
    'Add message
    logLine = logLine + RTRIM$(entry.message)
    
    'Output
    IF outputFile = 0 THEN
        'Console output with color
        SetLogColor entry.level
        PRINT logLine
        COLOR 7 'Reset to white
    ELSE
        'File output
        PRINT #outputFile, logLine
        LogFileSize = LogFileSize + LEN(logLine) + 2 'Include CRLF
        LogStatistics.bytesWritten = LogStatistics.bytesWritten + LEN(logLine) + 2
    END IF
END SUB

SUB SetLogColor (level AS INTEGER)
    SELECT CASE level
        CASE LOG_LEVEL_FATAL: COLOR 13 'Magenta
        CASE LOG_LEVEL_ERROR: COLOR 12 'Red
        CASE LOG_LEVEL_WARNING: COLOR 14 'Yellow
        CASE LOG_LEVEL_INFO: COLOR 15 'Bright white
        CASE LOG_LEVEL_DEBUG: COLOR 8  'Gray
        CASE LOG_LEVEL_TRACE: COLOR 7  'White
    END SELECT
END SUB

'-------------------------------------------------------------------------------
' CONVENIENCE LOGGING FUNCTIONS
'-------------------------------------------------------------------------------

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

'Category-specific logging
SUB LogParser (level AS INTEGER, message AS STRING, sourceFile AS STRING, lineNum AS INTEGER)
    LogMessage level, LOG_CAT_PARSER, message, sourceFile, lineNum
END SUB

SUB LogCodegen (level AS INTEGER, message AS STRING, sourceFile AS STRING, lineNum AS INTEGER)
    LogMessage level, LOG_CAT_CODEGEN, message, sourceFile, lineNum
END SUB

SUB LogPerformance (message AS STRING, sourceFile AS STRING, lineNum AS INTEGER)
    LogMessage LOG_LEVEL_INFO, LOG_CAT_PERFORMANCE, message, sourceFile, lineNum
END SUB

'-------------------------------------------------------------------------------
' CONFIGURATION INTERFACE
'-------------------------------------------------------------------------------

SUB SetLogLevel (consoleLevel AS INTEGER, fileLevel AS INTEGER)
    LogConfiguration.consoleLevel = consoleLevel
    LogConfiguration.fileLevel = fileLevel
END SUB

SUB EnableLogCategory (category AS INTEGER)
    IF category >= 0 AND category < MAX_LOG_CATEGORIES THEN
        LogConfiguration.categoryEnabled(category) = -1
    END IF
END SUB

SUB DisableLogCategory (category AS INTEGER)
    IF category >= 0 AND category < MAX_LOG_CATEGORIES THEN
        LogConfiguration.categoryEnabled(category) = 0
    END IF
END SUB

SUB SetLogOutput (toConsole AS _BYTE, toFile AS _BYTE)
    LogConfiguration.logToConsole = toConsole
    LogConfiguration.logToFile = toFile
    
    'Open file if enabling file logging
    IF toFile AND LogFileNumber = 0 THEN
        OpenLogFile
    END IF
END SUB

SUB SetLogFilePath (filePath AS STRING)
    LogConfiguration.logFilePath = filePath
    
    'Close and reopen with new path
    IF LogFileNumber > 0 THEN
        FlushLogBuffer
        CLOSE #LogFileNumber
        LogFileNumber = 0
        OpenLogFile
    END IF
END SUB

'-------------------------------------------------------------------------------
' ASSERTIONS
'-------------------------------------------------------------------------------

SUB LogAssert (condition AS _BYTE, message AS STRING, sourceFile AS STRING, lineNum AS INTEGER)
    IF NOT LogConfiguration.enableAssertions THEN EXIT SUB
    
    IF NOT condition THEN
        LogMessage LOG_LEVEL_FATAL, LOG_CAT_GENERAL, "ASSERTION FAILED: " + message, sourceFile, lineNum
        FlushLogBuffer
        
        IF LogConfiguration.breakOnError THEN
            'Would trigger breakpoint here
            PRINT "Assertion failed at "; sourceFile; ":"; lineNum
            END 1
        END IF
    END IF
END SUB

'-------------------------------------------------------------------------------
' STRING UTILITIES
'-------------------------------------------------------------------------------

FUNCTION GetLevelString$ (level AS INTEGER)
    SELECT CASE level
        CASE LOG_LEVEL_NONE: GetLevelString$ = "NONE"
        CASE LOG_LEVEL_FATAL: GetLevelString$ = "FATAL"
        CASE LOG_LEVEL_ERROR: GetLevelString$ = "ERROR"
        CASE LOG_LEVEL_WARNING: GetLevelString$ = "WARN"
        CASE LOG_LEVEL_INFO: GetLevelString$ = "INFO"
        CASE LOG_LEVEL_DEBUG: GetLevelString$ = "DEBUG"
        CASE LOG_LEVEL_TRACE: GetLevelString$ = "TRACE"
        CASE ELSE: GetLevelString$ = "UNKNOWN"
    END SELECT
END FUNCTION

FUNCTION GetCategoryString$ (category AS INTEGER)
    SELECT CASE category
        CASE LOG_CAT_GENERAL: GetCategoryString$ = "GEN"
        CASE LOG_CAT_PARSER: GetCategoryString$ = "PARSER"
        CASE LOG_CAT_SCANNER: GetCategoryString$ = "SCAN"
        CASE LOG_CAT_SEMANTIC: GetCategoryString$ = "SEM"
        CASE LOG_CAT_CODEGEN: GetCategoryString$ = "CODE"
        CASE LOG_CAT_OPTIMIZER: GetCategoryString$ = "OPT"
        CASE LOG_CAT_LINKER: GetCategoryString$ = "LINK"
        CASE LOG_CAT_MEMORY: GetCategoryString$ = "MEM"
        CASE LOG_CAT_PERFORMANCE: GetCategoryString$ = "PERF"
        CASE LOG_CAT_IO: GetCategoryString$ = "IO"
        CASE LOG_CAT_SYSTEM: GetCategoryString$ = "SYS"
        CASE ELSE: GetCategoryString$ = "UNK"
    END SELECT
END FUNCTION

'-------------------------------------------------------------------------------
' STATISTICS AND REPORTING
'-------------------------------------------------------------------------------

SUB PrintLogStats
    PRINT "=== Logging Statistics ==="
    PRINT "Entries Logged: "; LogStatistics.entriesLogged
    PRINT "Entries Dropped: "; LogStatistics.entriesDropped
    PRINT "Bytes Written: "; LogStatistics.bytesWritten
    PRINT "Files Created: "; LogStatistics.filesCreated
    PRINT "Files Rotated: "; LogStatistics.filesRotated
    PRINT "Errors: "; LogStatistics.errors
    PRINT ""
    PRINT "By Level:"
    PRINT "  Fatal: "; LogStatistics.fatalCount
    PRINT "  Error: "; LogStatistics.errorCount
    PRINT "  Warning: "; LogStatistics.warningCount
    PRINT "  Info: "; LogStatistics.infoCount
    PRINT "  Debug: "; LogStatistics.debugCount
    PRINT "  Trace: "; LogStatistics.traceCount
    PRINT "==========================="
END SUB

FUNCTION GetLogStats () AS LogStats
    GetLogStats = LogStatistics
END FUNCTION

SUB ResetLogStats
    WITH LogStatistics
        .entriesLogged = 0
        .entriesDropped = 0
        .bytesWritten = 0
        .fatalCount = 0
        .errorCount = 0
        .warningCount = 0
        .infoCount = 0
        .debugCount = 0
        .traceCount = 0
    END WITH
END SUB

