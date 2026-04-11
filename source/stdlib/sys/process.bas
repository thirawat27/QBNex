' ============================================================================
' QBNex Standard Library - System: Process Management
' ============================================================================
' Process execution and control
' ============================================================================

' ============================================================================
' SUB: Proc_Run
' Execute command (fire-and-forget)
' ============================================================================
SUB Proc_Run (cmd AS STRING)
    SHELL cmd
END SUB

' ============================================================================
' SUB: Proc_RunHidden
' Execute command hidden (no window)
' ============================================================================
SUB Proc_RunHidden (cmd AS STRING)
    SHELL _HIDE cmd
END SUB

' ============================================================================
' FUNCTION: Proc_RunWait
' Execute command and wait for completion, return exit code
' ============================================================================
FUNCTION Proc_RunWait& (cmd AS STRING)
    DIM tempFile AS STRING
    DIM exitCode AS LONG
    DIM fileNum AS LONG
    
    ' Create temp file for exit code
    tempFile = "qbnex_exit_" + LTRIM$(STR$(INT(RND * 10000))) + ".tmp"
    
    ' Run command and capture exit code
    IF INSTR(_OS$, "WIN") > 0 THEN
        SHELL _HIDE cmd + " & echo %ERRORLEVEL% > " + tempFile
    ELSE
        SHELL _HIDE cmd + "; echo $? > " + tempFile
    END IF
    
    ' Read exit code
    exitCode = 0
    fileNum = FREEFILE
    
    ON ERROR GOTO ReadError
    OPEN tempFile FOR INPUT AS #fileNum
    INPUT #fileNum, exitCode
    CLOSE #fileNum
    
    ReadError:
    ON ERROR GOTO 0
    
    ' Clean up
    KILL tempFile
    
    Proc_RunWait = exitCode
END FUNCTION

' ============================================================================
' FUNCTION: Proc_Capture
' Execute command and capture stdout
' ============================================================================
FUNCTION Proc_Capture$ (cmd AS STRING)
    DIM tempFile AS STRING
    DIM result AS STRING
    DIM fileNum AS LONG
    DIM LINE AS STRING
    
    ' Create temp file for output
    tempFile = "qbnex_out_" + LTRIM$(STR$(INT(RND * 10000))) + ".tmp"
    
    ' Run command and redirect output
    IF INSTR(_OS$, "WIN") > 0 THEN
        SHELL _HIDE cmd + " > " + tempFile + " 2>&1"
    ELSE
        SHELL _HIDE cmd + " > " + tempFile + " 2>&1"
    END IF
    
    ' Read output
    result = ""
    fileNum = FREEFILE
    
    ON ERROR GOTO CaptureError
    OPEN tempFile FOR INPUT AS #fileNum
    DO WHILE NOT EOF(fileNum)
        LINE INPUT #fileNum, LINE
        IF LEN(result) > 0 THEN result = result + CHR$(10)
        result = result + LINE
    LOOP
    CLOSE #fileNum
    
    CaptureError:
    ON ERROR GOTO 0
    
    ' Clean up
    KILL tempFile
    
    Proc_Capture = result
END FUNCTION

' ============================================================================
' FUNCTION: Proc_PID
' Get current process ID
' ============================================================================
FUNCTION Proc_PID& ()
    ' Platform-specific implementation
    DIM pid AS STRING
    
    IF INSTR(_OS$, "WIN") > 0 THEN
        pid = Proc_Capture("echo %RANDOM%")
    ELSE
        pid = Proc_Capture("echo $$")
    END IF
    
    Proc_PID = VAL(pid)
END FUNCTION

' ============================================================================
' SUB: Proc_Sleep
' Sleep for specified milliseconds
' ============================================================================
SUB Proc_Sleep (milliseconds AS LONG)
    DIM startTime AS DOUBLE
    startTime = TIMER
    
    DO WHILE (TIMER - startTime) * 1000 < milliseconds
        _LIMIT 100
    LOOP
END SUB

' ============================================================================
' FUNCTION: Proc_Exists
' Check if a process exists by PID
' ============================================================================
FUNCTION Proc_Exists& (pid AS LONG)
    DIM result AS STRING
    
    IF INSTR(_OS$, "WIN") > 0 THEN
        result = Proc_Capture("tasklist /FI " + CHR$(34) + "PID eq " + LTRIM$(STR$(pid)) + CHR$(34))
        Proc_Exists = INSTR(result, LTRIM$(STR$(pid))) > 0
    ELSE
        result = Proc_Capture("ps -p " + LTRIM$(STR$(pid)))
        Proc_Exists = INSTR(result, LTRIM$(STR$(pid))) > 0
    END IF
END FUNCTION

' ============================================================================
' SUB: Proc_Kill
' Terminate a process by PID
' ============================================================================
SUB Proc_Kill (pid AS LONG)
    IF INSTR(_OS$, "WIN") > 0 THEN
        Proc_RunHidden "taskkill /F /PID " + LTRIM$(STR$(pid))
    ELSE
        Proc_RunHidden "kill -9 " + LTRIM$(STR$(pid))
    END IF
END SUB
