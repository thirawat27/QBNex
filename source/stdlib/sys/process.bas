' =============================================================================
' QBNex System Integration — Process Helpers — process.bas
' =============================================================================
'
' Usage:
'
'   '$INCLUDE:'stdlib/sys/process.bas'
'
'   Proc_Run "notepad.exe"             ' launch and forget
'   DIM out$ = Proc_Capture$("dir /b")' capture stdout (Windows)
'   DIM exit% = Proc_RunWait("make")  ' (Linux) wait and get exit code
'   PRINT "exit code " + STR$(exit%)
'
' =============================================================================

'$INCLUDE:'stdlib/strings/strbuilder.bas'

' ---------------------------------------------------------------------------
' SUB  Proc_Run(cmd$)  — fire and forget
' ---------------------------------------------------------------------------
SUB Proc_Run (cmd$)
    SHELL cmd$
END SUB

' ---------------------------------------------------------------------------
' SUB  Proc_RunHidden(cmd$)  — hidden console on Windows
' ---------------------------------------------------------------------------
SUB Proc_RunHidden (cmd$)
    SHELL _HIDE cmd$
END SUB

' ---------------------------------------------------------------------------
' FUNCTION  Proc_RunWait&(cmd$)
'   Run command synchronously and return the exit/error code.
'   On Windows uses a temp batch wrapper to capture %ERRORLEVEL%.
'   On Linux/Mac uses $? via a shell script.
' ---------------------------------------------------------------------------
FUNCTION Proc_RunWait& (cmd$)
    DIM tmpFile AS STRING, fh AS LONG, result AS LONG
    tmpFile = "internal/temp/_proc_exit.tmp"
    IF INSTR(_OS$, "[WIN]") THEN
        SHELL _HIDE "cmd /c """ + cmd$ + """ & echo %ERRORLEVEL% > """ + tmpFile + """"
    ELSE
        SHELL _HIDE "sh -c """ + cmd$ + "; echo $? > " + tmpFile + """"
    END IF
    ' Read exit code
    IF _FILEEXISTS(tmpFile) THEN
        fh = FREEFILE
        OPEN tmpFile FOR INPUT AS #fh
        DIM line AS STRING
        LINE INPUT #fh, line
        CLOSE #fh
        KILL tmpFile
        result = VAL(_TRIM$(line))
    END IF
    Proc_RunWait& = result
END FUNCTION

' ---------------------------------------------------------------------------
' FUNCTION  Proc_Capture$(cmd$)
'   Runs command, captures its STDOUT, returns as a string.
'   Note: output is limited by available memory.
' ---------------------------------------------------------------------------
FUNCTION Proc_Capture$ (cmd$)
    DIM tmpFile AS STRING, fh AS LONG
    DIM sb AS QBNex_StringBuilder
    DIM line AS STRING

    tmpFile = "internal/temp/_proc_out.tmp"

    IF INSTR(_OS$, "[WIN]") THEN
        SHELL _HIDE "cmd /c """ + cmd$ + """ > """ + tmpFile + """ 2>&1"
    ELSE
        SHELL _HIDE "sh -c """ + cmd$ + " > " + tmpFile + " 2>&1"""
    END IF

    SB_Init sb
    IF _FILEEXISTS(tmpFile) THEN
        fh = FREEFILE
        OPEN tmpFile FOR INPUT AS #fh
        DO WHILE NOT EOF(fh)
            LINE INPUT #fh, line
            SB_AppendLine sb, line
        LOOP
        CLOSE #fh
        KILL tmpFile
    END IF
    Proc_Capture$ = SB_ToString$(sb)
    SB_Free sb
END FUNCTION

' ---------------------------------------------------------------------------
' FUNCTION  Proc_PID&()  — return current process ID (calls getpid via lib)
' ---------------------------------------------------------------------------
DECLARE LIBRARY
    FUNCTION getpid& ()
END DECLARE

FUNCTION Proc_PID& ()
    Proc_PID& = getpid&()
END FUNCTION

' ---------------------------------------------------------------------------
' SUB  Proc_Sleep(ms&)  — sleep for specified milliseconds
' ---------------------------------------------------------------------------
SUB Proc_Sleep (ms AS LONG)
    DIM target AS DOUBLE
    target = TIMER + ms / 1000.0
    DO: _LIMIT 1000: LOOP UNTIL TIMER >= target
END SUB
