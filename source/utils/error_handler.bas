'===============================================================================
' QBNex Error Handler Module
'===============================================================================
' Structured error handling and reporting for the compiler.
' Provides detailed error context, suggestions, and recovery options.
'===============================================================================

'-------------------------------------------------------------------------------
' ERROR SEVERITY LEVELS
'-------------------------------------------------------------------------------

CONST ERR_INFO = 1
CONST ERR_WARNING = 2
CONST ERR_ERROR = 3
CONST ERR_FATAL = 4

'-------------------------------------------------------------------------------
' ERROR CODES
'-------------------------------------------------------------------------------

' General errors (1000-1099)
CONST ERR_NO_SOURCE_FILE = 1001
CONST ERR_FILE_NOT_FOUND = 1002
CONST ERR_PERMISSION_DENIED = 1003
CONST ERR_OUT_OF_MEMORY = 1004
CONST ERR_INVALID_SYNTAX = 1005

' Parser errors (1100-1199)
CONST ERR_UNEXPECTED_TOKEN = 1101
CONST ERR_UNCLOSED_STRING = 1102
CONST ERR_UNCLOSED_COMMENT = 1103
CONST ERR_INVALID_IDENTIFIER = 1104
CONST ERR_EXPECTED_EOL = 1105
CONST ERR_EXPECTED_THEN = 1106
CONST ERR_EXPECTED_TO = 1107
CONST ERR_EXPECTED_NEXT = 1108
CONST ERR_EXPECTED_LOOP = 1109
CONST ERR_EXPECTED_WEND = 1110

' Semantic errors (1200-1299)
CONST ERR_UNDEFINED_SYMBOL = 1201
CONST ERR_REDEFINED_SYMBOL = 1202
CONST ERR_TYPE_MISMATCH = 1203
CONST ERR_WRONG_ARGUMENT_COUNT = 1204
CONST ERR_INVALID_SCOPE = 1205
CONST ERR_UNDECLARED_VARIABLE = 1206
CONST ERR_OPTION_EXPLICIT_VIOLATION = 1207
CONST ERR_INVALID_ARRAY_BOUNDS = 1208
CONST ERR_SUBSCRIPT_OUT_OF_RANGE = 1209

' Code generation errors (1300-1399)
CONST ERR_CODEGEN_FAILED = 1301
CONST ERR_UNSUPPORTED_FEATURE = 1302
CONST ERR_LINK_ERROR = 1303

'-------------------------------------------------------------------------------
' ERROR INFORMATION TYPE
'-------------------------------------------------------------------------------

TYPE ErrorInfo
    errorCode AS INTEGER
    severity AS INTEGER
    message AS STRING * 512
    fileName AS STRING * 256
    lineNumber AS LONG
    columnNumber AS INTEGER
    context AS STRING * 256
    suggestion AS STRING * 256
    recovered AS _BYTE
END TYPE

'-------------------------------------------------------------------------------
' ERROR STATISTICS
'-------------------------------------------------------------------------------

TYPE ErrorStats
    infoCount AS LONG
    warningCount AS LONG
    errorCount AS LONG
    fatalCount AS LONG
    totalCount AS LONG
    maxErrors AS LONG
    hasErrors AS _BYTE
END TYPE

'-------------------------------------------------------------------------------
' ERROR STATE
'-------------------------------------------------------------------------------

DIM SHARED Errors(1 TO 1000) AS ErrorInfo
DIM SHARED ErrorCount AS LONG
DIM SHARED Stats AS ErrorStats
DIM SHARED CurrentFile AS STRING
DIM SHARED ErrorOutputFile AS INTEGER
DIM SHARED VerboseMode AS _BYTE
DIM SHARED WarningsAsErrors AS _BYTE

'-------------------------------------------------------------------------------
' INITIALIZATION
'-------------------------------------------------------------------------------

SUB InitErrorHandler
    ErrorCount = 0
    CurrentFile = ""
    ErrorOutputFile = 0
    VerboseMode = 0
    WarningsAsErrors = 0
    
    Stats.infoCount = 0
    Stats.warningCount = 0
    Stats.errorCount = 0
    Stats.fatalCount = 0
    Stats.totalCount = 0
    Stats.maxErrors = 100
    Stats.hasErrors = 0
END SUB

SUB SetErrorOutputFile (fileNum AS INTEGER)
    ErrorOutputFile = fileNum
END SUB

SUB SetVerboseMode (enabled AS _BYTE)
    VerboseMode = enabled
END SUB

SUB SetWarningsAsErrors (enabled AS _BYTE)
    WarningsAsErrors = enabled
END SUB

SUB SetMaxErrors (maxCount AS LONG)
    Stats.maxErrors = maxCount
END SUB

SUB SetCurrentFile (fileName AS STRING)
    CurrentFile = fileName
END SUB

'-------------------------------------------------------------------------------
' ERROR REPORTING
'-------------------------------------------------------------------------------

SUB ReportError (errCode AS INTEGER, message AS STRING, lineNum AS LONG, context AS STRING)
    DIM severity AS INTEGER
    
    ' Determine severity from error code
    SELECT CASE errCode
        CASE 1000 TO 1099: severity = ERR_ERROR
        CASE 1100 TO 1199: severity = ERR_ERROR
        CASE 1200 TO 1299: severity = ERR_ERROR
        CASE 1300 TO 1399: severity = ERR_FATAL
        CASE ELSE: severity = ERR_ERROR
    END SELECT
    
    ReportErrorWithSeverity errCode, severity, message, lineNum, context
END SUB

SUB ReportErrorWithSeverity (errCode AS INTEGER, severity AS INTEGER, message AS STRING, lineNum AS LONG, context AS STRING)
    DIM errIdx AS LONG
    
    ' Check if we've reached max errors
    IF Stats.errorCount >= Stats.maxErrors THEN
        IF ErrorCount < 1000 THEN
            ErrorCount = ErrorCount + 1
            Errors(ErrorCount).errorCode = 9999
            Errors(ErrorCount).severity = ERR_FATAL
            Errors(ErrorCount).message = "Too many errors - compilation aborted"
            Errors(ErrorCount).fileName = CurrentFile
            Errors(ErrorCount).lineNumber = 0
            Errors(ErrorCount).columnNumber = 0
            Errors(ErrorCount).context = ""
            Errors(ErrorCount).suggestion = "Fix the reported errors and recompile"
        END IF
        Stats.fatalCount = Stats.fatalCount + 1
        EXIT SUB
    END IF
    
    ' Add error to list
    IF ErrorCount < 1000 THEN
        ErrorCount = ErrorCount + 1
        errIdx = ErrorCount
        
        Errors(errIdx).errorCode = errCode
        Errors(errIdx).severity = severity
        Errors(errIdx).message = message
        Errors(errIdx).fileName = CurrentFile
        Errors(errIdx).lineNumber = lineNum
        Errors(errIdx).columnNumber = 0
        Errors(errIdx).context = context
        Errors(errIdx).suggestion = GetSuggestion$(errCode)
        Errors(errIdx).recovered = 0
    END IF
    
    ' Update statistics
    Stats.totalCount = Stats.totalCount + 1
    SELECT CASE severity
        CASE ERR_INFO: Stats.infoCount = Stats.infoCount + 1
        CASE ERR_WARNING
            Stats.warningCount = Stats.warningCount + 1
            IF WarningsAsErrors THEN
                Stats.hasErrors = -1
            END IF
        CASE ERR_ERROR
            Stats.errorCount = Stats.errorCount + 1
            Stats.hasErrors = -1
        CASE ERR_FATAL
            Stats.fatalCount = Stats.fatalCount + 1
            Stats.hasErrors = -1
    END SELECT
    
    ' Print error immediately
    PrintError errIdx
END SUB

SUB ReportWarning (warnCode AS INTEGER, message AS STRING, lineNum AS LONG, context AS STRING)
    ReportErrorWithSeverity warnCode, ERR_WARNING, message, lineNum, context
END SUB

SUB ReportInfo (infoCode AS INTEGER, message AS STRING, lineNum AS LONG)
    ReportErrorWithSeverity infoCode, ERR_INFO, message, lineNum, ""
END SUB

'-------------------------------------------------------------------------------
' ERROR MESSAGE FORMATTING
'-------------------------------------------------------------------------------

FUNCTION GetErrorMessage$ (errCode AS INTEGER)
    SELECT CASE errCode
        ' General errors
        CASE ERR_NO_SOURCE_FILE: GetErrorMessage$ = "No source file specified"
        CASE ERR_FILE_NOT_FOUND: GetErrorMessage$ = "Source file not found"
        CASE ERR_PERMISSION_DENIED: GetErrorMessage$ = "Permission denied accessing file"
        CASE ERR_OUT_OF_MEMORY: GetErrorMessage$ = "Out of memory"
        CASE ERR_INVALID_SYNTAX: GetErrorMessage$ = "Invalid syntax"
        
        ' Parser errors
        CASE ERR_UNEXPECTED_TOKEN: GetErrorMessage$ = "Unexpected token"
        CASE ERR_UNCLOSED_STRING: GetErrorMessage$ = "Unclosed string literal"
        CASE ERR_UNCLOSED_COMMENT: GetErrorMessage$ = "Unclosed comment"
        CASE ERR_INVALID_IDENTIFIER: GetErrorMessage$ = "Invalid identifier"
        CASE ERR_EXPECTED_EOL: GetErrorMessage$ = "Expected end of line"
        CASE ERR_EXPECTED_THEN: GetErrorMessage$ = "Expected THEN"
        CASE ERR_EXPECTED_TO: GetErrorMessage$ = "Expected TO"
        CASE ERR_EXPECTED_NEXT: GetErrorMessage$ = "Expected NEXT"
        CASE ERR_EXPECTED_LOOP: GetErrorMessage$ = "Expected LOOP"
        CASE ERR_EXPECTED_WEND: GetErrorMessage$ = "Expected WEND"
        
        ' Semantic errors
        CASE ERR_UNDEFINED_SYMBOL: GetErrorMessage$ = "Undefined symbol"
        CASE ERR_REDEFINED_SYMBOL: GetErrorMessage$ = "Symbol already defined"
        CASE ERR_TYPE_MISMATCH: GetErrorMessage$ = "Type mismatch"
        CASE ERR_WRONG_ARGUMENT_COUNT: GetErrorMessage$ = "Wrong number of arguments"
        CASE ERR_INVALID_SCOPE: GetErrorMessage$ = "Invalid scope"
        CASE ERR_UNDECLARED_VARIABLE: GetErrorMessage$ = "Undeclared variable (Option Explicit)"
        CASE ERR_OPTION_EXPLICIT_VIOLATION: GetErrorMessage$ = "Option Explicit requires variable declaration"
        CASE ERR_INVALID_ARRAY_BOUNDS: GetErrorMessage$ = "Invalid array bounds"
        CASE ERR_SUBSCRIPT_OUT_OF_RANGE: GetErrorMessage$ = "Subscript out of range"
        
        ' Code generation errors
        CASE ERR_CODEGEN_FAILED: GetErrorMessage$ = "Code generation failed"
        CASE ERR_UNSUPPORTED_FEATURE: GetErrorMessage$ = "Unsupported feature"
        CASE ERR_LINK_ERROR: GetErrorMessage$ = "Link error"
        
        CASE ELSE: GetErrorMessage$ = "Unknown error"
    END SELECT
END FUNCTION

FUNCTION GetSuggestion$ (errCode AS INTEGER)
    SELECT CASE errCode
        CASE ERR_NO_SOURCE_FILE: GetSuggestion$ = "Specify a .bas file to compile"
        CASE ERR_FILE_NOT_FOUND: GetSuggestion$ = "Check the file path and try again"
        CASE ERR_PERMISSION_DENIED: GetSuggestion$ = "Check file permissions or run with elevated privileges"
        CASE ERR_OUT_OF_MEMORY: GetSuggestion$ = "Close other applications or use smaller source files"
        CASE ERR_UNDEFINED_SYMBOL: GetSuggestion$ = "Define the symbol or check for typos"
        CASE ERR_REDEFINED_SYMBOL: GetSuggestion$ = "Use a different name or remove duplicate definition"
        CASE ERR_TYPE_MISMATCH: GetSuggestion$ = "Convert the value to the correct type"
        CASE ERR_UNDECLARED_VARIABLE: GetSuggestion$ = "Declare the variable with DIM or turn off Option Explicit"
        CASE ERR_INVALID_ARRAY_BOUNDS: GetSuggestion$ = "Check array dimension bounds"
        CASE ERR_UNSUPPORTED_FEATURE: GetSuggestion$ = "Use an alternative syntax or update QBNex"
        CASE ELSE: GetSuggestion$ = ""
    END SELECT
END FUNCTION

FUNCTION GetSeverityString$ (severity AS INTEGER)
    SELECT CASE severity
        CASE ERR_INFO: GetSeverityString$ = "INFO"
        CASE ERR_WARNING: GetSeverityString$ = "WARNING"
        CASE ERR_ERROR: GetSeverityString$ = "ERROR"
        CASE ERR_FATAL: GetSeverityString$ = "FATAL"
        CASE ELSE: GetSeverityString$ = "UNKNOWN"
    END SELECT
END FUNCTION

'-------------------------------------------------------------------------------
' ERROR OUTPUT
'-------------------------------------------------------------------------------

SUB PrintError (errIdx AS LONG)
    IF errIdx < 1 OR errIdx > ErrorCount THEN EXIT SUB
    
    DIM err AS ErrorInfo
    DIM severityStr AS STRING
    DIM output AS STRING
    
    err = Errors(errIdx)
    severityStr = GetSeverityString$(err.severity)
    
    ' Build error message
    output = severityStr + " " + LTRIM$(STR$(err.errorCode)) + ": "
    
    IF err.fileName <> "" THEN
        output = output + RTRIM$(err.fileName)
        IF err.lineNumber > 0 THEN
            output = output + "(" + LTRIM$(STR$(err.lineNumber)) + ")"
        END IF
        output = output + ": "
    END IF
    
    output = output + RTRIM$(err.message)
    
    ' Print to console with color coding (if supported)
    SELECT CASE err.severity
        CASE ERR_INFO
            COLOR 7 ' White
        CASE ERR_WARNING
            COLOR 14 ' Yellow
        CASE ERR_ERROR, ERR_FATAL
            COLOR 12 ' Red
    END SELECT
    
    PRINT output
    
    COLOR 7 ' Reset to white
    
    ' Print context if available
    IF err.context <> "" THEN
        PRINT "    Context: "; RTRIM$(err.context)
    END IF
    
    ' Print suggestion if verbose
    IF VerboseMode AND err.suggestion <> "" THEN
        PRINT "    Suggestion: "; RTRIM$(err.suggestion)
    END IF
    
    ' Also write to error output file if set
    IF ErrorOutputFile > 0 THEN
        PRINT #ErrorOutputFile, output
        IF err.context <> "" THEN
            PRINT #ErrorOutputFile, "    Context: " + RTRIM$(err.context)
        END IF
    END IF
END SUB

SUB PrintAllErrors
    DIM i AS LONG
    
    IF ErrorCount = 0 THEN
        PRINT "No errors reported."
        EXIT SUB
    END IF
    
    PRINT ""
    PRINT "=== Error Report ==="
    PRINT ""
    
    FOR i = 1 TO ErrorCount
        PrintError i
    NEXT
    
    PrintErrorSummary
END SUB

SUB PrintErrorSummary
    IF Stats.totalCount = 0 THEN EXIT SUB
    
    PRINT ""
    PRINT "=== Summary ==="
    
    IF Stats.infoCount > 0 THEN
        PRINT "Info: "; Stats.infoCount
    END IF
    
    IF Stats.warningCount > 0 THEN
        PRINT "Warnings: "; Stats.warningCount
    END IF
    
    IF Stats.errorCount > 0 THEN
        PRINT "Errors: "; Stats.errorCount
    END IF
    
    IF Stats.fatalCount > 0 THEN
        PRINT "Fatal: "; Stats.fatalCount
    END IF
    
    IF Stats.hasErrors THEN
        PRINT ""
        PRINT "Compilation failed due to errors."
    ELSE
        PRINT ""
        PRINT "Compilation completed with warnings."
    END IF
END SUB

'-------------------------------------------------------------------------------
' ERROR RECOVERY
'-------------------------------------------------------------------------------

FUNCTION CanRecover% (errCode AS INTEGER)
    ' Determine if compilation can continue after this error
    SELECT CASE errCode
        CASE ERR_NO_SOURCE_FILE, ERR_FILE_NOT_FOUND, ERR_OUT_OF_MEMORY
            CanRecover% = 0
        CASE ERR_FATAL
            CanRecover% = 0
        CASE ELSE
            CanRecover% = -1
    END SELECT
END FUNCTION

SUB MarkRecovered (errIdx AS LONG)
    IF errIdx >= 1 AND errIdx <= ErrorCount THEN
        Errors(errIdx).recovered = -1
    END IF
END SUB

FUNCTION ShouldAbort%
    ShouldAbort% = (Stats.fatalCount > 0) OR (Stats.errorCount >= Stats.maxErrors)
END FUNCTION

'-------------------------------------------------------------------------------
' QUERY FUNCTIONS
'-------------------------------------------------------------------------------

FUNCTION HasErrors%
    HasErrors% = Stats.hasErrors
END FUNCTION

FUNCTION GetErrorCount%
    GetErrorCount% = ErrorCount
END FUNCTION

FUNCTION GetWarningCount%
    GetWarningCount% = Stats.warningCount
END FUNCTION

FUNCTION GetFatalCount%
    GetFatalCount% = Stats.fatalCount
END FUNCTION

'-------------------------------------------------------------------------------
' CLEANUP
'-------------------------------------------------------------------------------

SUB CleanupErrorHandler
    ErrorCount = 0
    CurrentFile = ""
    
    IF ErrorOutputFile > 0 THEN
        CLOSE #ErrorOutputFile
        ErrorOutputFile = 0
    END IF
END SUB
