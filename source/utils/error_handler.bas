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
CONST ERR_ENCODING_ISSUE = 1006
CONST ERR_INVALID_UTF8 = 1007

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
    suggestion AS STRING * 512
    cause AS STRING * 256
    fixExample AS STRING * 256
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
    DIM colNum AS INTEGER
    DIM causeStr AS STRING
    DIM fixStr AS STRING
    
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
            Errors(ErrorCount).cause = "Maximum error limit reached"
            Errors(ErrorCount).fixExample = ""
            Errors(ErrorCount).recovered = 0
        END IF
        Stats.fatalCount = Stats.fatalCount + 1
        EXIT SUB
    END IF
    
    ' Determine column number from context if available
    colNum = 0
    IF context <> "" THEN
        colNum = FindErrorColumn%(errCode, context)
    END IF
    
    ' Get detailed error info
    causeStr = GetErrorCause$(errCode, context)
    fixStr = GetFixExample$(errCode, context)
    
    ' Add error to list
    IF ErrorCount < 1000 THEN
        ErrorCount = ErrorCount + 1
        errIdx = ErrorCount
        
        Errors(errIdx).errorCode = errCode
        Errors(errIdx).severity = severity
        Errors(errIdx).message = message
        Errors(errIdx).fileName = CurrentFile
        Errors(errIdx).lineNumber = lineNum
        Errors(errIdx).columnNumber = colNum
        Errors(errIdx).context = context
        Errors(errIdx).suggestion = GetDetailedSuggestion$(errCode, context)
        Errors(errIdx).cause = causeStr
        Errors(errIdx).fixExample = fixStr
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
        CASE ERR_NO_SOURCE_FILE: GetSuggestion$ = "Specify a .bas file to compile: qb program.bas"
        CASE ERR_FILE_NOT_FOUND: GetSuggestion$ = "Check the file path exists and try again"
        CASE ERR_PERMISSION_DENIED: GetSuggestion$ = "Check file permissions or run with elevated privileges"
        CASE ERR_OUT_OF_MEMORY: GetSuggestion$ = "Close other applications or use smaller source files"
        CASE ERR_INVALID_SYNTAX: GetSuggestion$ = "Review the syntax and compare with QBasic documentation"
        CASE ERR_ENCODING_ISSUE: GetSuggestion$ = "Save the source file as UTF-8 encoded text"
        CASE ERR_INVALID_UTF8: GetSuggestion$ = "Remove invalid UTF-8 characters or re-encode the file"
        CASE ERR_UNEXPECTED_TOKEN: GetSuggestion$ = "Remove or replace the unexpected symbol"
        CASE ERR_UNCLOSED_STRING: GetSuggestion$ = "Add closing double quote to the string literal"
        CASE ERR_UNCLOSED_COMMENT: GetSuggestion$ = "Close the comment with appropriate delimiter"
        CASE ERR_INVALID_IDENTIFIER: GetSuggestion$ = "Use valid variable names (letters, numbers, underscores)"
        CASE ERR_EXPECTED_THEN: GetSuggestion$ = "Add THEN after the IF condition"
        CASE ERR_EXPECTED_TO: GetSuggestion$ = "Add TO in the FOR loop statement"
        CASE ERR_EXPECTED_NEXT: GetSuggestion$ = "Add NEXT to close the FOR loop"
        CASE ERR_EXPECTED_LOOP: GetSuggestion$ = "Add LOOP to close the DO loop"
        CASE ERR_EXPECTED_WEND: GetSuggestion$ = "Add WEND to close the WHILE loop"
        CASE ERR_UNDEFINED_SYMBOL: GetSuggestion$ = "Define the symbol or check for typos"
        CASE ERR_REDEFINED_SYMBOL: GetSuggestion$ = "Use a different name or remove duplicate definition"
        CASE ERR_TYPE_MISMATCH: GetSuggestion$ = "Convert the value to the correct type using type conversion functions"
        CASE ERR_WRONG_ARGUMENT_COUNT: GetSuggestion$ = "Check the function signature and provide correct arguments"
        CASE ERR_INVALID_SCOPE: GetSuggestion$ = "Move the statement to an appropriate scope"
        CASE ERR_UNDECLARED_VARIABLE: GetSuggestion$ = "Declare the variable with DIM or turn off Option Explicit"
        CASE ERR_OPTION_EXPLICIT_VIOLATION: GetSuggestion$ = "Add DIM statement before using the variable"
        CASE ERR_INVALID_ARRAY_BOUNDS: GetSuggestion$ = "Check array dimension bounds are positive integers"
        CASE ERR_SUBSCRIPT_OUT_OF_RANGE: GetSuggestion$ = "Ensure array index is within declared bounds"
        CASE ERR_CODEGEN_FAILED: GetSuggestion$ = "Check for syntax errors or unsupported language features"
        CASE ERR_UNSUPPORTED_FEATURE: GetSuggestion$ = "Use an alternative syntax or update QBNex"
        CASE ERR_LINK_ERROR: GetSuggestion$ = "Check C++ compiler installation and library paths"
        CASE ELSE: GetSuggestion$ = "Review the error context and consult the documentation"
    END SELECT
END FUNCTION

'-------------------------------------------------------------------------------
' DETAILED ERROR INFORMATION
'-------------------------------------------------------------------------------

FUNCTION GetDetailedSuggestion$ (errCode AS INTEGER, context AS STRING)
    DIM basicSuggestion AS STRING
    basicSuggestion = GetSuggestion$(errCode)
    
    SELECT CASE errCode
        CASE ERR_INVALID_SYNTAX
            IF INSTR(context, "$") THEN
                GetDetailedSuggestion$ = "Metacommands must start with $. Check for typos in " + context
            ELSEIF INSTR(context, "(") AND NOT INSTR(context, ")") THEN
                GetDetailedSuggestion$ = "Missing closing parenthesis. Add ) at the end of the expression"
            ELSEIF INSTR(context, "IF") AND NOT INSTR(context, "THEN") THEN
                GetDetailedSuggestion$ = "IF statements require THEN. Example: IF x = 1 THEN"
            ELSEIF INSTR(context, "FOR") AND NOT INSTR(context, "TO") THEN
                GetDetailedSuggestion$ = "FOR loops require TO. Example: FOR i = 1 TO 10"
            ELSE
                GetDetailedSuggestion$ = basicSuggestion
            END IF
            
        CASE ERR_UNDECLARED_VARIABLE
            IF context <> "" THEN
                GetDetailedSuggestion$ = "Variable '" + RTRIM$(context) + "' was used but not declared. Add: DIM " + RTRIM$(context) + " AS [type]"
            ELSE
                GetDetailedSuggestion$ = basicSuggestion
            END IF
            
        CASE ERR_TYPE_MISMATCH
            IF INSTR(context, "$") THEN
                GetDetailedSuggestion$ = "String variable used where numeric expected. Use VAL() to convert or remove $ from variable"
            ELSEIF INSTR(context, "!") OR INSTR(context, "#") THEN
                GetDetailedSuggestion$ = "Numeric type mismatch. Use appropriate type conversion: CSNG(), CDBL(), CINT(), CLNG()"
            ELSE
                GetDetailedSuggestion$ = basicSuggestion
            END IF
            
        CASE ERR_ENCODING_ISSUE
            GetDetailedSuggestion$ = "The source file contains characters that cannot be properly decoded. Save the file as UTF-8 without BOM using Notepad++ or VS Code. Thai characters should display correctly when properly encoded."
            
        CASE ERR_INVALID_UTF8
            GetDetailedSuggestion$ = "Invalid UTF-8 byte sequence detected. This often happens when mixing ANSI and UTF-8 encoding. Re-save the file as UTF-8."
            
        CASE ELSE
            GetDetailedSuggestion$ = basicSuggestion
    END SELECT
END FUNCTION

FUNCTION GetErrorCause$ (errCode AS INTEGER, context AS STRING)
    SELECT CASE errCode
        CASE ERR_INVALID_SYNTAX
            GetErrorCause$ = "The compiler could not parse the statement due to incorrect syntax or unexpected symbols"
        CASE ERR_UNCLOSED_STRING
            GetErrorCause$ = "A string literal was opened with " + CHR$(34) + " but never closed"
        CASE ERR_UNDEFINED_SYMBOL
            GetErrorCause$ = "A variable, function, or label was referenced that has not been defined"
        CASE ERR_REDEFINED_SYMBOL
            GetErrorCause$ = "A variable, function, or type was defined more than once in the same scope"
        CASE ERR_TYPE_MISMATCH
            GetErrorCause$ = "An operation was attempted between incompatible data types"
        CASE ERR_UNDECLARED_VARIABLE
            GetErrorCause$ = "OPTION EXPLICIT is enabled and a variable was used without being declared first"
        CASE ERR_ENCODING_ISSUE
            GetErrorCause$ = "Source file encoding does not match expected UTF-8 format"
        CASE ERR_INVALID_UTF8
            GetErrorCause$ = "Multi-byte UTF-8 character sequence is incomplete or malformed"
        CASE ERR_UNEXPECTED_TOKEN
            GetErrorCause$ = "A symbol appeared where the compiler did not expect it"
        CASE ERR_OUT_OF_MEMORY
            GetErrorCause$ = "System has insufficient memory to continue compilation"
        CASE ERR_FILE_NOT_FOUND
            GetErrorCause$ = "The specified source file or include file could not be found"
        CASE ERR_PERMISSION_DENIED
            GetErrorCause$ = "Operating system denied access to the file"
        CASE ERR_WRONG_ARGUMENT_COUNT
            GetErrorCause$ = "Function/subroutine called with wrong number of arguments"
        CASE ERR_INVALID_ARRAY_BOUNDS
            GetErrorCause$ = "Array dimensions must be positive integers"
        CASE ERR_SUBSCRIPT_OUT_OF_RANGE
            GetErrorCause$ = "Array index exceeds the declared bounds"
        CASE ERR_CODEGEN_FAILED
            GetErrorCause$ = "Internal error during C++ code generation"
        CASE ERR_UNSUPPORTED_FEATURE
            GetErrorCause$ = "Language feature not yet implemented in QBNex"
        CASE ERR_LINK_ERROR
            GetErrorCause$ = "C++ linker failed to create executable"
        CASE ELSE
            GetErrorCause$ = ""
    END SELECT
END FUNCTION

FUNCTION GetFixExample$ (errCode AS INTEGER, context AS STRING)
    SELECT CASE errCode
        CASE ERR_INVALID_SYNTAX
            IF INSTR(context, "IF") THEN
                GetFixExample$ = "IF x = 1 THEN PRINT " + CHR$(34) + "Yes" + CHR$(34) + " ELSE PRINT " + CHR$(34) + "No" + CHR$(34)
            ELSEIF INSTR(context, "FOR") THEN
                GetFixExample$ = "FOR i = 1 TO 10 STEP 2: PRINT i: NEXT"
            ELSEIF INSTR(context, "SUB") THEN
                GetFixExample$ = "SUB MySub(param AS INTEGER): 'code: END SUB"
            ELSEIF INSTR(context, "FUNCTION") THEN
                GetFixExample$ = "FUNCTION MyFunc(x) AS INTEGER: MyFunc = x * 2: END FUNCTION"
            ELSE
                GetFixExample$ = ""
            END IF
            
        CASE ERR_UNDECLARED_VARIABLE
            GetFixExample$ = "DIM myVariable AS STRING  'At top of sub/function or module"
            
        CASE ERR_TYPE_MISMATCH
            GetFixExample$ = "numValue = VAL(stringValue$)  'Convert string to number"
            
        CASE ERR_UNCLOSED_STRING
            GetFixExample$ = "text$ = " + CHR$(34) + "Complete sentence" + CHR$(34)
            
        CASE ERR_WRONG_ARGUMENT_COUNT
            GetFixExample$ = "Check function definition for required parameters"
            
        CASE ERR_INVALID_ARRAY_BOUNDS
            GetFixExample$ = "DIM arr(1 TO 100)  'or DIM arr(100) for 0-based"
            
        CASE ERR_ENCODING_ISSUE
            GetFixExample$ = "In Notepad++: Encoding > Convert to UTF-8 without BOM"
            
        CASE ERR_REDEFINED_SYMBOL
            GetFixExample$ = "Use unique names: myVar1, myVar2 instead of myVar, myVar"
            
        CASE ELSE
            GetFixExample$ = ""
    END SELECT
END FUNCTION

FUNCTION FindErrorColumn% (errCode AS INTEGER, context AS STRING)
    ' Attempt to find the column where the error likely occurred
    DIM pos AS INTEGER
    
    SELECT CASE errCode
        CASE ERR_UNCLOSED_STRING
            pos = INSTR(context, CHR$(34))
            IF pos > 0 THEN FindErrorColumn% = pos
        CASE ERR_EXPECTED_THEN
            pos = INSTR(UCASE$(context), "IF")
            IF pos > 0 THEN FindErrorColumn% = pos + 2
        CASE ERR_EXPECTED_TO
            pos = INSTR(UCASE$(context), "FOR")
            IF pos > 0 THEN FindErrorColumn% = pos + 3
        CASE ERR_UNEXPECTED_TOKEN
            ' Try to find first unusual character
            DIM i AS INTEGER
            FOR i = 1 TO LEN(context)
                SELECT CASE MID$(context, i, 1)
                    CASE "@", "#", "$", "%", "&", "*", "^", "!"
                        FindErrorColumn% = i
                        EXIT FUNCTION
                END SELECT
            NEXT
        CASE ELSE
            FindErrorColumn% = 0
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
    DIM lineStr AS STRING
    
    err = Errors(errIdx)
    severityStr = GetSeverityString$(err.severity)
    
    ' Print header separator
    PRINT STRING$(60, "=")
    
    ' Print error type and code with color coding
    SELECT CASE err.severity
        CASE ERR_INFO
            COLOR 7 ' White
            PRINT "[INFO] ";
        CASE ERR_WARNING
            COLOR 14 ' Yellow
            PRINT "[WARNING] ";
        CASE ERR_ERROR
            COLOR 12 ' Red
            PRINT "[ERROR] ";
        CASE ERR_FATAL
            COLOR 4 ' Dark Red
            PRINT "[FATAL ERROR] ";
    END SELECT
    
    COLOR 7 ' Reset
    PRINT "Code: " + LTRIM$(STR$(err.errorCode))
    
    ' Print location
    IF err.fileName <> "" THEN
        PRINT "  Location: ";
        COLOR 11 ' Cyan
        PRINT RTRIM$(err.fileName);
        COLOR 7
        IF err.lineNumber > 0 THEN
            PRINT " at line " + LTRIM$(STR$(err.lineNumber));
            IF err.columnNumber > 0 THEN
                PRINT ", column " + LTRIM$(STR$(err.columnNumber));
            END IF
        END IF
        PRINT
    END IF
    
    ' Print error message
    PRINT
    COLOR 15 ' Bright White
    PRINT "  " + RTRIM$(err.message)
    COLOR 7
    
    ' Print context with line indicator
    IF err.context <> "" THEN
        PRINT
        PRINT "  Source context:"
        COLOR 8 ' Gray
        lineStr = LTRIM$(STR$(err.lineNumber))
        PRINT "    " + STRING$(5 - LEN(lineStr), " ") + lineStr + " | ";
        COLOR 7
        PRINT RTRIM$(err.context)
        ' Print column indicator
        IF err.columnNumber > 0 THEN
            COLOR 12 ' Red
            PRINT "         | " + STRING$(err.columnNumber - 1, " ") + "^"
            COLOR 7
        END IF
    END IF
    
    ' Print cause if available
    IF err.cause <> "" THEN
        PRINT
        COLOR 13 ' Magenta
        PRINT "  Cause: " + RTRIM$(err.cause)
        COLOR 7
    END IF
    
    ' Print suggestion with fix
    IF err.suggestion <> "" THEN
        PRINT
        COLOR 10 ' Green
        PRINT "  Suggestion: " + RTRIM$(err.suggestion)
        COLOR 7
    END IF
    
    ' Print example fix if available
    IF err.fixExample <> "" THEN
        PRINT
        COLOR 14 ' Yellow
        PRINT "  Example fix:"
        PRINT "    " + RTRIM$(err.fixExample)
        COLOR 7
    END IF
    
    PRINT STRING$(60, "=")
    
    ' Also write to error output file if set
    IF ErrorOutputFile > 0 THEN
        PRINT #ErrorOutputFile, "[ERROR " + LTRIM$(STR$(err.errorCode)) + "]"
        IF err.fileName <> "" THEN
            PRINT #ErrorOutputFile, "File: " + RTRIM$(err.fileName) + " Line: " + LTRIM$(STR$(err.lineNumber))
        END IF
        PRINT #ErrorOutputFile, "Message: " + RTRIM$(err.message)
        IF err.context <> "" THEN
            PRINT #ErrorOutputFile, "Context: " + RTRIM$(err.context)
        END IF
        IF err.suggestion <> "" THEN
            PRINT #ErrorOutputFile, "Suggestion: " + RTRIM$(err.suggestion)
        END IF
        PRINT #ErrorOutputFile, ""
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
