' =============================================================================
' QBNex I/O Library — Cross-Platform Path Manipulation — path.bas
' =============================================================================
'
' Usage:
'
'   '$INCLUDE:'stdlib/io/path.bas'
'
'   PRINT Path_Join$("C:\Users\foo", "docs\readme.txt")   ' C:\Users\foo\docs\readme.txt
'   PRINT Path_Dir$("C:\Users\foo\readme.txt")            ' C:\Users\foo
'   PRINT Path_Filename$("C:\Users\foo\readme.txt")       ' readme.txt
'   PRINT Path_Basename$("readme.txt")                    ' readme
'   PRINT Path_Extension$("readme.txt")                   ' .txt
'   PRINT Path_Normalize$("C:/Users/foo/../bar")          ' C:\Users\bar  (Win)
'   PRINT Path_IsAbsolute$("C:\foo")                      ' -1 (TRUE)
'
' =============================================================================

' Current platform path separator — set once at startup
DIM SHARED QBNEX_PathSep AS STRING * 1
IF INSTR(_OS$, "[LINUX]") OR INSTR(_OS$, "[MACOSX]") THEN
    QBNEX_PathSep = "/"
ELSE
    QBNEX_PathSep = "\"
END IF

' Normalises any slash to the platform separator
FUNCTION Path_NormSlash$ (p$)
    DIM result AS STRING, i AS LONG, ch AS STRING
    result = ""
    FOR i = 1 TO LEN(p$)
        ch = MID$(p$, i, 1)
        IF ch = "/" OR ch = "\" THEN
            result = result + QBNEX_PathSep
        ELSE
            result = result + ch
        END IF
    NEXT i
    Path_NormSlash$ = result
END FUNCTION

' Join two path segments with a single separator
FUNCTION Path_Join$ (base$, part$)
    DIM b AS STRING, p AS STRING
    b = Path_NormSlash$(base$)
    p = Path_NormSlash$(part$)
    ' remove trailing sep from b
    DO WHILE LEN(b) > 0 AND RIGHT$(b, 1) = QBNEX_PathSep
        b = LEFT$(b, LEN(b) - 1)
    LOOP
    ' remove leading sep from p
    DO WHILE LEN(p) > 0 AND LEFT$(p, 1) = QBNEX_PathSep
        p = MID$(p, 2)
    LOOP
    IF LEN(b) = 0 THEN Path_Join$ = p: EXIT FUNCTION
    IF LEN(p) = 0 THEN Path_Join$ = b: EXIT FUNCTION
    Path_Join$ = b + QBNEX_PathSep + p
END FUNCTION

' Directory part (everything before the last separator)
FUNCTION Path_Dir$ (p$)
    DIM nrm AS STRING, i AS LONG
    nrm = Path_NormSlash$(p$)
    FOR i = LEN(nrm) TO 1 STEP -1
        IF MID$(nrm, i, 1) = QBNEX_PathSep THEN
            Path_Dir$ = LEFT$(nrm, i - 1): EXIT FUNCTION
        END IF
    NEXT i
    Path_Dir$ = ""
END FUNCTION

' Filename with extension (everything after last separator)
FUNCTION Path_Filename$ (p$)
    DIM nrm AS STRING, i AS LONG
    nrm = Path_NormSlash$(p$)
    FOR i = LEN(nrm) TO 1 STEP -1
        IF MID$(nrm, i, 1) = QBNEX_PathSep THEN
            Path_Filename$ = MID$(nrm, i + 1): EXIT FUNCTION
        END IF
    NEXT i
    Path_Filename$ = nrm
END FUNCTION

' Filename without extension
FUNCTION Path_Basename$ (p$)
    DIM fname AS STRING, i AS LONG
    fname = Path_Filename$(p$)
    FOR i = LEN(fname) TO 1 STEP -1
        IF MID$(fname, i, 1) = "." THEN
            Path_Basename$ = LEFT$(fname, i - 1): EXIT FUNCTION
        END IF
    NEXT i
    Path_Basename$ = fname
END FUNCTION

' Extension including the dot (e.g. ".bas"), "" if none
FUNCTION Path_Extension$ (p$)
    DIM fname AS STRING, i AS LONG
    fname = Path_Filename$(p$)
    FOR i = LEN(fname) TO 1 STEP -1
        IF MID$(fname, i, 1) = "." THEN
            Path_Extension$ = MID$(fname, i): EXIT FUNCTION
        END IF
    NEXT i
    Path_Extension$ = ""
END FUNCTION

' Returns -1 if the path is absolute
FUNCTION Path_IsAbsolute& (p$)
    DIM nrm AS STRING
    nrm = _TRIM$(p$)
    IF LEN(nrm) = 0 THEN Path_IsAbsolute& = 0: EXIT FUNCTION
    ' Unix: starts with /
    IF LEFT$(nrm, 1) = "/" THEN Path_IsAbsolute& = -1: EXIT FUNCTION
    ' Windows: drive letter like C:\
    IF LEN(nrm) >= 3 AND MID$(nrm, 2, 1) = ":" THEN Path_IsAbsolute& = -1: EXIT FUNCTION
    Path_IsAbsolute& = 0
END FUNCTION

' Collapse ".." and "." segments (simplistic — not resolving symlinks)
FUNCTION Path_Normalize$ (p$)
    DIM nrm AS STRING, parts() AS STRING, result() AS STRING
    DIM n AS LONG, i AS LONG, seg AS STRING, ri AS LONG
    nrm = Path_NormSlash$(p$)

    ' split on separator
    n = 0
    REDIM parts(1 TO LEN(nrm) + 1) AS STRING
    DIM s AS LONG, pos AS LONG
    s = 1
    DO
        pos = INSTR(s, nrm, QBNEX_PathSep)
        n = n + 1
        IF pos = 0 THEN
            parts(n) = MID$(nrm, s): EXIT DO
        END IF
        parts(n) = MID$(nrm, s, pos - s)
        s = pos + 1
    LOOP

    REDIM result(1 TO n + 1) AS STRING
    ri = 0
    FOR i = 1 TO n
        seg = parts(i)
        IF seg = ".." THEN
            IF ri > 0 THEN ri = ri - 1
        ELSEIF seg = "." OR seg = "" THEN
            ' skip
        ELSE
            ri = ri + 1: result(ri) = seg
        END IF
    NEXT i

    DIM out AS STRING
    out = ""
    FOR i = 1 TO ri
        IF i > 1 THEN out = out + QBNEX_PathSep
        out = out + result(i)
    NEXT i
    ' re-attach leading sep for absolute paths
    IF LEFT$(nrm, 1) = QBNEX_PathSep THEN out = QBNEX_PathSep + out
    Path_Normalize$ = out
END FUNCTION

' Change extension of a file path
FUNCTION Path_ChangeExt$ (p$, newExt$)
    DIM dir AS STRING, base AS STRING
    dir  = Path_Dir$(p$)
    base = Path_Basename$(p$)
    DIM ext AS STRING
    ext = newExt$
    IF LEN(ext) > 0 AND LEFT$(ext, 1) <> "." THEN ext = "." + ext
    IF LEN(dir) > 0 THEN
        Path_ChangeExt$ = Path_Join$(dir, base + ext)
    ELSE
        Path_ChangeExt$ = base + ext
    END IF
END FUNCTION
