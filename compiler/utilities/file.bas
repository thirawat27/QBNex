''' Splits the filename from its path and returns the path.
''' Returns: The path or empty if no path.
FUNCTION GetFilePath$ (f$)
    DIM normalizedPath AS STRING
    DIM directoryPath AS STRING
    DIM resultPath AS STRING
    DIM separator AS STRING
    'the compiler asks for the folder of the main source and of the current $INCLUDE
    'file at every statement: the last two answers are kept
    STATIC cacheIn1 AS STRING, cacheOut1 AS STRING, cacheUsed1 AS LONG
    STATIC cacheIn2 AS STRING, cacheOut2 AS STRING, cacheUsed2 AS LONG

    IF cacheUsed1 <> 0 AND f$ = cacheIn1 THEN GetFilePath$ = cacheOut1: EXIT FUNCTION
    IF cacheUsed2 <> 0 AND f$ = cacheIn2 THEN GetFilePath$ = cacheOut2: EXIT FUNCTION

    normalizedPath = Path_Normalize$(f$)
    directoryPath = Path_Dir$(normalizedPath)
    IF directoryPath <> "" THEN
        separator = Path_Sep$
        resultPath = directoryPath
        IF RIGHT$(resultPath, 1) <> separator THEN resultPath = resultPath + separator
    END IF

    cacheIn2 = cacheIn1: cacheOut2 = cacheOut1: cacheUsed2 = cacheUsed1
    cacheIn1 = f$: cacheOut1 = resultPath: cacheUsed1 = -1
    GetFilePath$ = resultPath
END FUNCTION

''' Checks if p$ names a file from a root: /x, \x, \\server\share\x, or C:\x on Windows.
''' Returns: True for such a path; a relative one is relative to some folder.
FUNCTION IsAbsolutePath% (p$)
    IF LEFT$(p$, 1) = "/" OR LEFT$(p$, 1) = "\" THEN IsAbsolutePath% = -1: EXIT FUNCTION
    IF os$ = "WIN" AND MID$(p$, 2, 1) = ":" THEN IsAbsolutePath% = -1
END FUNCTION

''' Joins a folder and a name with the system's separator: D:\x\y.bas, not D:\x/y.bas,
''' and D:\y.bas for the drive's root folder D:\.
''' Returns: folder$ + name$ with one separator between them.
FUNCTION JoinPath$ (folder$, name$)
    IF LEN(folder$) = 0 THEN JoinPath$ = name$: EXIT FUNCTION
    IF RIGHT$(folder$, 1) = "/" OR RIGHT$(folder$, 1) = "\" THEN JoinPath$ = folder$ + name$ ELSE JoinPath$ = folder$ + pathsep$ + name$
END FUNCTION

''' Checks if a filename has an extension on the end.
''' Returns: True if provided filename has an extension.
FUNCTION FileHasExtension (f$)
    IF Path_Ext$(f$) <> "" THEN FileHasExtension = -1
END FUNCTION

''' Removes the extension off of a filename.
''' Returns: Provided filename without extension on the end.
FUNCTION RemoveFileExtension$ (f$)
    DIM i AS LONG
    DIM a AS LONG
    
    FOR i = LEN(f$) TO 1 STEP -1
        a = ASC(f$, i)
        IF a = 47 OR a = 92 THEN EXIT FOR
        IF a = 46 THEN RemoveFileExtension$ = LEFT$(f$, i - 1): EXIT FUNCTION
    NEXT
    RemoveFileExtension$ = f$
END FUNCTION

''' The file path$ names, found the way DOS and Windows find it: path$ itself when it
''' exists; otherwise, on Linux and macOS, the existing file whose path differs from path$
''' only in letter case ($INCLUDE:'SUB\Inc.BI' for sub/inc.bi). The path is resolved folder
''' by folder: a part that exists as written is kept, and of several names that differ only
''' in case the first in byte order is taken, so the choice does not depend on the folder.
''' Returns: the path to open, or "" when there is no such file.
FUNCTION ResolvePathCase$ (path$)
    DIM p AS STRING, done AS STRING, part AS STRING, cand AS STRING
    DIM i AS LONG, j AS LONG, isLast AS LONG

    IF _FILEEXISTS(path$) THEN ResolvePathCase$ = path$: EXIT FUNCTION
    IF os$ = "WIN" OR LEN(path$) = 0 THEN EXIT FUNCTION

    p = StrReplace$(path$, "\", "/")
    IF LEFT$(p, 1) = "/" THEN done = "/": i = 2 ELSE i = 1
    DO WHILE i <= LEN(p)
        j = INSTR(i, p, "/")
        IF j = 0 THEN j = LEN(p) + 1: isLast = -1
        part = MID$(p, i, j - i)
        i = j + 1
        IF isLast THEN
            cand = done + part
            IF _FILEEXISTS(cand) = 0 THEN cand = PathCaseMatch$(done, part, 0): IF LEN(cand) = 0 THEN EXIT FUNCTION
            ResolvePathCase$ = cand
            EXIT FUNCTION
        END IF
        IF LEN(part) THEN
            cand = done + part
            IF part <> "." AND part <> ".." THEN
                IF _DIREXISTS(cand) = 0 THEN cand = PathCaseMatch$(done, part, -1): IF LEN(cand) = 0 THEN EXIT FUNCTION
            END IF
            done = cand + "/"
        END IF
    LOOP
END FUNCTION

''' The entry of folder$ ("" is the current folder; else it ends in /) whose name is name$
''' in another letter case, a folder when wantDir, else a file. Returns: folder$ + the name,
''' or "" when there is none.
FUNCTION PathCaseMatch$ (folder$, name$, wantDir AS LONG)
    DIM listing AS STRING, dirKey AS STRING, entry AS STRING, best AS STRING, un AS STRING
    DIM p AS LONG, q AS LONG, fh AS LONG, lst AS STRING, x AS STRING

    dirKey = CHR$(0) + folder$ + CHR$(1)
    p = INSTR(pathCaseListings, dirKey)
    IF p THEN
        p = p + LEN(dirKey)
        q = INSTR(p, pathCaseListings, CHR$(0))
        IF q = 0 THEN q = LEN(pathCaseListings) + 1
        listing = MID$(pathCaseListings, p, q - p)
    ELSE
        'the shell lists the folder (the runtime has no function that does): its names, one
        'per line, without the quoting ls may add
        lst = tmpdir$ + "pathcase.txt"
        x = folder$: IF LEN(x) = 0 THEN x = "."
        SHELL _HIDE "(cd -- " + QuotedFilename$(x) + " 2>/dev/null && for f in .* *; do printf '%s\n' " + CHR$(34) + "$f" + CHR$(34) + "; done) > " + QuotedFilename$(lst) + " 2>/dev/null"
        IF _FILEEXISTS(lst) THEN
            fh = FREEFILE
            OPEN lst FOR BINARY AS #fh
            IF LOF(fh) THEN listing = SPACE$(LOF(fh)): GET #fh, , listing
            CLOSE #fh
            KILL lst
        END IF
        pathCaseListings = pathCaseListings + dirKey + listing
    END IF

    un = UCASE$(name$)
    p = 1
    DO WHILE p <= LEN(listing)
        q = INSTR(p, listing, CHR$(10))
        IF q = 0 THEN q = LEN(listing) + 1
        entry = MID$(listing, p, q - p)
        p = q + 1
        IF UCASE$(entry) = un AND entry <> "." AND entry <> ".." THEN
            IF LEN(best) = 0 OR entry < best THEN
                IF wantDir THEN
                    IF _DIREXISTS(folder$ + entry) THEN best = entry
                ELSE
                    IF _FILEEXISTS(folder$ + entry) THEN best = entry
                END IF
            END IF
        END IF
    LOOP
    IF LEN(best) THEN PathCaseMatch$ = folder$ + best
END FUNCTION
