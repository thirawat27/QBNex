' =============================================================================
' QBNex Error Handling Library — error.bas
' =============================================================================
'
' Structured error handling that layers on top of QBNex's ON ERROR GOTO.
'
' Pattern:
'
'   '$INCLUDE:'stdlib/error/error.bas'
'
'   Err_SetHandler MyErrorHandler
'
'   ' ... code that calls Err_Raise on problems ...
'
'   SUB MyErrorHandler (e AS QBNex_Error)
'       PRINT "Error " + STR$(e.Code) + ": " + e.Message
'       IF e.Fatal THEN END 1
'   END SUB
'
'   ' Raise an error:
'   IF _FILEEXISTS("data.csv") = 0 THEN
'       Err_Raise 404, "File not found: data.csv", -1  ' -1 = fatal
'   END IF
'
' =============================================================================

CONST QBNEX_ERR_MAX_STACK = 32

TYPE QBNex_Error
    Code      AS LONG       ' custom or ERR code
    Message   AS STRING     ' human-readable description
    Source    AS STRING     ' sub/function name where error occurred
    LineNum   AS LONG       ' source line number if known
    Fatal     AS LONG       ' -1 = stop program, 0 = recoverable
    Handled   AS LONG       ' set to -1 by handler to suppress default behavior
END TYPE

' Error stack (last-in, first-out for nested error contexts)
DIM SHARED QBNEX_ErrStack(1 TO QBNEX_ERR_MAX_STACK) AS QBNex_Error
DIM SHARED QBNEX_ErrStackTop AS LONG
QBNEX_ErrStackTop = 0

DIM SHARED QBNEX_LastError AS QBNex_Error
DIM SHARED QBNEX_ErrHandlerSet AS LONG
QBNEX_ErrHandlerSet = 0

' ---------------------------------------------------------------------------
' SUB  Err_Raise(code, message$, fatal)
'   Raises a structured error. Calls the registered handler if set.
'   If no handler, prints to console. If fatal, ends with code 1.
' ---------------------------------------------------------------------------
SUB Err_Raise (code AS LONG, message$, fatal AS LONG)
    QBNEX_LastError.Code    = code
    QBNEX_LastError.Message = message$
    QBNEX_LastError.Fatal   = fatal
    QBNEX_LastError.Handled = 0

    ' Push onto stack
    IF QBNEX_ErrStackTop < QBNEX_ERR_MAX_STACK THEN
        QBNEX_ErrStackTop = QBNEX_ErrStackTop + 1
        QBNEX_ErrStack(QBNEX_ErrStackTop) = QBNEX_LastError
    END IF

    IF NOT QBNEX_ErrHandlerSet THEN
        PRINT "QBNex Error [" + STR$(code) + "]: " + message$
        IF fatal THEN END 1
    END IF
END SUB

' Convenience: raise without fatality
SUB Err_RaiseWarn (code AS LONG, message$)
    Err_Raise code, message$, 0
END SUB

' Convenience: raise fatal error
SUB Err_Fail (message$)
    Err_Raise -1, message$, -1
END SUB

' ---------------------------------------------------------------------------
' FUNCTION  Err_HasError&()   — returns -1 if there is an unhandled error
' ---------------------------------------------------------------------------
FUNCTION Err_HasError& ()
    Err_HasError& = (QBNEX_ErrStackTop > 0 AND NOT QBNEX_ErrStack(QBNEX_ErrStackTop).Handled)
END FUNCTION

' ---------------------------------------------------------------------------
' FUNCTION  Err_Last()  — returns a copy of the most recent error
' ---------------------------------------------------------------------------
FUNCTION Err_LastCode& ()
    Err_LastCode& = QBNEX_LastError.Code
END FUNCTION

FUNCTION Err_LastMessage$ ()
    Err_LastMessage$ = QBNEX_LastError.Message
END FUNCTION

FUNCTION Err_LastFatal& ()
    Err_LastFatal& = QBNEX_LastError.Fatal
END FUNCTION

' ---------------------------------------------------------------------------
' SUB  Err_Clear()   — pop the top error off the stack (mark as handled)
' ---------------------------------------------------------------------------
SUB Err_Clear ()
    IF QBNEX_ErrStackTop > 0 THEN
        QBNEX_ErrStack(QBNEX_ErrStackTop).Handled = -1
        QBNEX_ErrStackTop = QBNEX_ErrStackTop - 1
    END IF
    QBNEX_LastError.Code = 0
    QBNEX_LastError.Message = ""
    QBNEX_LastError.Fatal = 0
    QBNEX_LastError.Handled = -1
END SUB

' ---------------------------------------------------------------------------
' FUNCTION  Err_Format$(code, msg$)   — format an error for display
' ---------------------------------------------------------------------------
FUNCTION Err_Format$ (code AS LONG, msg$)
    Err_Format$ = "[E" + _TRIM$(STR$(code)) + "] " + msg$
END FUNCTION

' ---------------------------------------------------------------------------
' Guard helpers
' ---------------------------------------------------------------------------

' Assert that condition is true, else raise fatal error
SUB Err_Assert (condition AS LONG, msg$)
    IF NOT condition THEN Err_Fail msg$
END SUB

' Assert file exists, else raise error 2 (file not found)
SUB Err_AssertFile (filepath$)
    IF NOT _FILEEXISTS(filepath$) THEN
        Err_Raise 2, "File not found: " + filepath$, 0
    END IF
END SUB

' Assert directory exists
SUB Err_AssertDir (dirpath$)
    IF NOT _DIREXISTS(dirpath$) THEN
        Err_Raise 3, "Directory not found: " + dirpath$, 0
    END IF
END SUB
