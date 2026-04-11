' ============================================================================
' QBNex Standard Library - Error Handling
' ============================================================================
' Structured error handling with error stack
' ============================================================================

TYPE QBNex_Error
    Code AS LONG
    Message AS STRING * 256
    Fatal AS LONG
    Handled AS LONG
END TYPE

' Error stack (max 32 errors)
DIM SHARED QBNEX_ErrorStack(1 TO 32) AS QBNex_Error
DIM SHARED QBNEX_ErrorStackTop AS LONG

' ============================================================================
' SUB: Err_Raise
' Raise an error
' ============================================================================
SUB Err_Raise (code AS LONG, message AS STRING, fatal AS LONG)
    IF QBNEX_ErrorStackTop >= 32 THEN
        PRINT "FATAL: Error stack overflow"
        SYSTEM
    END IF
    
    QBNEX_ErrorStackTop = QBNEX_ErrorStackTop + 1
    QBNEX_ErrorStack(QBNEX_ErrorStackTop).Code = code
    QBNEX_ErrorStack(QBNEX_ErrorStackTop).Message = message
    QBNEX_ErrorStack(QBNEX_ErrorStackTop).Fatal = fatal
    QBNEX_ErrorStack(QBNEX_ErrorStackTop).Handled = 0
    
    IF fatal THEN
        PRINT "FATAL ERROR ["; LTRIM$(STR$(code)); "]: "; message
        SYSTEM
    END IF
END SUB

' ============================================================================
' SUB: Err_RaiseWarn
' Raise a non-fatal warning
' ============================================================================
SUB Err_RaiseWarn (code AS LONG, message AS STRING)
    Err_Raise code, message, 0
END SUB

' ============================================================================
' SUB: Err_Fail
' Raise a fatal error
' ============================================================================
SUB Err_Fail (message AS STRING)
    Err_Raise -1, message, -1
END SUB

' ============================================================================
' FUNCTION: Err_HasError
' Check if there are unhandled errors
' ============================================================================
FUNCTION Err_HasError& ()
    DIM i AS LONG
    FOR i = 1 TO QBNEX_ErrorStackTop
        IF QBNEX_ErrorStack(i).Handled = 0 THEN
            Err_HasError = -1
            EXIT FUNCTION
        END IF
    NEXT i
    Err_HasError = 0
END FUNCTION

' ============================================================================
' SUB: Err_Clear
' Clear the most recent unhandled error
' ============================================================================
SUB Err_Clear ()
    DIM i AS LONG
    FOR i = QBNEX_ErrorStackTop TO 1 STEP -1
        IF QBNEX_ErrorStack(i).Handled = 0 THEN
            QBNEX_ErrorStack(i).Handled = -1
            EXIT SUB
        END IF
    NEXT i
END SUB

' ============================================================================
' SUB: Err_ClearAll
' Clear all errors
' ============================================================================
SUB Err_ClearAll ()
    QBNEX_ErrorStackTop = 0
END SUB

' ============================================================================
' FUNCTION: Err_GetLast
' Get the most recent error
' ============================================================================
FUNCTION Err_GetLast$ ()
    DIM i AS LONG
    FOR i = QBNEX_ErrorStackTop TO 1 STEP -1
        IF QBNEX_ErrorStack(i).Handled = 0 THEN
            Err_GetLast = RTRIM$(QBNEX_ErrorStack(i).Message)
            EXIT FUNCTION
        END IF
    NEXT i
    Err_GetLast = ""
END FUNCTION

' ============================================================================
' FUNCTION: Err_GetLastCode
' Get the most recent error code
' ============================================================================
FUNCTION Err_GetLastCode& ()
    DIM i AS LONG
    FOR i = QBNEX_ErrorStackTop TO 1 STEP -1
        IF QBNEX_ErrorStack(i).Handled = 0 THEN
            Err_GetLastCode = QBNEX_ErrorStack(i).Code
            EXIT FUNCTION
        END IF
    NEXT i
    Err_GetLastCode = 0
END FUNCTION

' ============================================================================
' SUB: Err_Assert
' Assert a condition
' ============================================================================
SUB Err_Assert (condition AS LONG, message AS STRING)
    IF NOT condition THEN
        Err_Fail "Assertion failed: " + message
    END IF
END SUB

' ============================================================================
' SUB: Err_AssertFile
' Assert file exists
' ============================================================================
SUB Err_AssertFile (filePath AS STRING)
    DIM fileNum AS LONG
    DIM exists AS LONG
    
    exists = 0
    fileNum = FREEFILE
    
    ON ERROR GOTO FileNotFound
    OPEN filePath FOR INPUT AS #fileNum
    CLOSE #fileNum
    exists = -1
    
    FileNotFound:
    ON ERROR GOTO 0
    
    IF NOT exists THEN
        Err_Fail "File not found: " + filePath
    END IF
END SUB

' ============================================================================
' SUB: Err_AssertDir
' Assert directory exists
' ============================================================================
SUB Err_AssertDir (dirPath AS STRING)
    ' Simplified - full implementation would use platform-specific checks
    IF LEN(dirPath) = 0 THEN
        Err_Fail "Directory path is empty"
    END IF
END SUB

' ============================================================================
' FUNCTION: Err_Format
' Format error message with code
' ============================================================================
FUNCTION Err_Format$ (code AS LONG, message AS STRING)
    Err_Format = "[E" + LTRIM$(STR$(code)) + "] " + message
END FUNCTION
