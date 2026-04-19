'===============================================================================
' QBNex Error Handler Declarations
'===============================================================================
' This file stays separate from error_handler.bas because QB64 requires
' CONST/TYPE/DIM declarations to appear before later SUB/FUNCTION bodies.
' Structured compiler diagnostics that explain:
' - what happened
' - where it happened
' - why the compiler stopped
' - how to fix it
'===============================================================================

'-------------------------------------------------------------------------------
' ERROR SEVERITY LEVELS
'-------------------------------------------------------------------------------

CONST ERR_INFO = 1
CONST ERR_WARNING = 2
CONST MAX_ERROR_CONTEXTS = 16

'-------------------------------------------------------------------------------
' ERROR CODES
'-------------------------------------------------------------------------------

' General errors (1000-1099)
CONST ERR_NO_SOURCE_FILE = 1001
CONST ERR_FILE_NOT_FOUND = 1002
CONST ERR_PERMISSION_DENIED = 1003
CONST ERR_OUT_OF_MEMORY = 1004
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
    secondaryContext AS STRING * 256
    locationNote AS STRING * 256
    suggestion AS STRING * 512
    cause AS STRING * 256
    fixExample AS STRING * 256
    phaseName AS STRING * 64
    contextTrace AS STRING * 512
    fingerprint AS STRING * 512
    duplicateCount AS LONG
    recovered AS _BYTE
    wasPrinted AS _BYTE
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
    suppressedDuplicateCount AS LONG
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
DIM SHARED ErrorOutputFileEnabled AS _BYTE
DIM SHARED VerboseMode AS _BYTE
DIM SHARED WarningsAsErrors AS _BYTE
DIM SHARED ErrorsFlushed AS _BYTE
DIM SHARED LastRenderedFingerprint AS STRING * 512
DIM SHARED CurrentErrorPhase AS STRING * 64
DIM SHARED ErrorContextDepth AS INTEGER
DIM SHARED ErrorContextStack(1 TO MAX_ERROR_CONTEXTS) AS STRING * 128
