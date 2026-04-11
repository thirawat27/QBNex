' ============================================================================
' QBNex Standard Library - I/O: Path Manipulation
' ============================================================================
' Cross-platform path operations
' ============================================================================

DIM SHARED QBNEX_PathSep AS STRING

' Initialize path separator based on platform
SUB Path_InitSeparator
    IF INSTR(_OS$, "WIN") > 0 THEN
        QBNEX_PathSep = "\"
    ELSE
        QBNEX_PathSep = "/"
    END IF
END SUB

' Auto-initialize on first use
IF QBNEX_PathSep = "" THEN Path_InitSeparator

' ============================================================================
' FUNCTION: Path_Join
' Join path components
' ============================================================================
FUNCTION Path_Join$ (a AS STRING, b AS STRING)
    DIM result AS STRING
    
    result = a
    IF RIGHT$(result, 1) <> QBNEX_PathSep AND RIGHT$(result, 1) <> "/" AND RIGHT$(result, 1) <> "\" THEN
        result = result + QBNEX_PathSep
    END IF
    
    ' Remove leading separator from b
    IF LEFT$(b, 1) = "/" OR LEFT$(b, 1) = "\" THEN
        b = MID$(b, 2)
    END IF
    
    Path_Join = result + b
END FUNCTION

' ============================================================================
' FUNCTION: Path_Dir
' Get directory part of path
' ============================================================================
FUNCTION Path_Dir$ (path AS STRING)
    DIM POS AS LONG
    DIM i AS LONG
    
    POS = 0
    FOR i = LEN(path) TO 1 STEP -1
        IF MID$(path, i, 1) = "/" OR MID$(path, i, 1) = "\" THEN
            POS = i
            EXIT FOR
        END IF
    NEXT i
    
    IF POS > 0 THEN
        Path_Dir = LEFT$(path, POS - 1)
    ELSE
        Path_Dir = ""
    END IF
END FUNCTION

' ============================================================================
' FUNCTION: Path_Filename
' Get filename part (with extension)
' ============================================================================
FUNCTION Path_Filename$ (path AS STRING)
    DIM POS AS LONG
    DIM i AS LONG
    
    POS = 0
    FOR i = LEN(path) TO 1 STEP -1
        IF MID$(path, i, 1) = "/" OR MID$(path, i, 1) = "\" THEN
            POS = i
            EXIT FOR
        END IF
    NEXT i
    
    IF POS > 0 THEN
        Path_Filename = MID$(path, POS + 1)
    ELSE
        Path_Filename = path
    END IF
END FUNCTION

' ============================================================================
' FUNCTION: Path_Basename
' Get filename without extension
' ============================================================================
FUNCTION Path_Basename$ (path AS STRING)
    DIM filename AS STRING
    DIM POS AS LONG
    DIM i AS LONG
    
    filename = Path_Filename(path)
    
    ' Find last dot
    POS = 0
    FOR i = LEN(filename) TO 1 STEP -1
        IF MID$(filename, i, 1) = "." THEN
            POS = i
            EXIT FOR
        END IF
    NEXT i
    
    IF POS > 1 THEN
        Path_Basename = LEFT$(filename, POS - 1)
    ELSE
        Path_Basename = filename
    END IF
END FUNCTION

' ============================================================================
' FUNCTION: Path_Extension
' Get file extension (with dot)
' ============================================================================
FUNCTION Path_Extension$ (path AS STRING)
    DIM filename AS STRING
    DIM POS AS LONG
    DIM i AS LONG
    
    filename = Path_Filename(path)
    
    ' Find last dot
    POS = 0
    FOR i = LEN(filename) TO 1 STEP -1
        IF MID$(filename, i, 1) = "." THEN
            POS = i
            EXIT FOR
        END IF
    NEXT i
    
    IF POS > 1 THEN
        Path_Extension = MID$(filename, POS)
    ELSE
        Path_Extension = ""
    END IF
END FUNCTION

' ============================================================================
' FUNCTION: Path_ChangeExt
' Change file extension
' ============================================================================
FUNCTION Path_ChangeExt$ (path AS STRING, newExt AS STRING)
    DIM dir AS STRING
    DIM BASE AS STRING
    
    dir = Path_Dir(path)
    BASE = Path_Basename(path)
    
    IF LEFT$(newExt, 1) <> "." AND LEN(newExt) > 0 THEN
        newExt = "." + newExt
    END IF
    
    IF LEN(dir) > 0 THEN
        Path_ChangeExt = Path_Join(dir, BASE + newExt)
    ELSE
        Path_ChangeExt = BASE + newExt
    END IF
END FUNCTION

' ============================================================================
' FUNCTION: Path_IsAbsolute
' Check if path is absolute
' ============================================================================
FUNCTION Path_IsAbsolute& (path AS STRING)
    IF LEN(path) = 0 THEN
        Path_IsAbsolute = 0
        EXIT FUNCTION
    END IF
    
    ' Unix absolute path
    IF LEFT$(path, 1) = "/" THEN
        Path_IsAbsolute = -1
        EXIT FUNCTION
    END IF
    
    ' Windows absolute path (C:\ or \\)
    IF LEN(path) >= 2 THEN
        IF MID$(path, 2, 1) = ":" THEN
            Path_IsAbsolute = -1
            EXIT FUNCTION
        END IF
        IF LEFT$(path, 2) = "\\" THEN
            Path_IsAbsolute = -1
            EXIT FUNCTION
        END IF
    END IF
    
    Path_IsAbsolute = 0
END FUNCTION

' ============================================================================
' FUNCTION: Path_Normalize
' Normalize path (resolve .. and .)
' ============================================================================
FUNCTION Path_Normalize$ (path AS STRING)
    DIM parts() AS STRING
    DIM result() AS STRING
    DIM partCount AS LONG
    DIM resultCount AS LONG
    DIM i AS LONG
    DIM POS AS LONG
    DIM lastPos AS LONG
    DIM part AS STRING
    
    ' Split path by separators
    REDIM parts(1 TO 100) AS STRING
    partCount = 0
    lastPos = 1
    
    FOR i = 1 TO LEN(path)
        IF MID$(path, i, 1) = "/" OR MID$(path, i, 1) = "\" THEN
            IF i > lastPos THEN
                partCount = partCount + 1
                parts(partCount) = MID$(path, lastPos, i - lastPos)
            END IF
            lastPos = i + 1
        END IF
    NEXT i
    
    ' Add last part
    IF lastPos <= LEN(path) THEN
        partCount = partCount + 1
        parts(partCount) = MID$(path, lastPos)
    END IF
    
    ' Process parts
    REDIM result(1 TO 100) AS STRING
    resultCount = 0
    
    FOR i = 1 TO partCount
        part = parts(i)
        IF part = ".." THEN
            IF resultCount > 0 THEN resultCount = resultCount - 1
        ELSEIF part <> "." AND LEN(part) > 0 THEN
            resultCount = resultCount + 1
            result(resultCount) = part
        END IF
    NEXT i
    
    ' Rebuild path
    IF resultCount = 0 THEN
        Path_Normalize = "."
    ELSE
        Path_Normalize = result(1)
        FOR i = 2 TO resultCount
            Path_Normalize = Path_Normalize + QBNEX_PathSep + result(i)
        NEXT i
    END IF
END FUNCTION
