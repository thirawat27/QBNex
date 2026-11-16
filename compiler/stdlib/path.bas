'===============================================================================
' QBNex Standard Library - path ('$IMPORT:'path')
'===============================================================================
' File paths, like Python's os.path. "/" and "\" are both accepted; results
' use the separator of the computer (Path_Sep$).
'
'   Path_Join$("dir", "sub", "f.txt")   "dir/sub/f.txt" (up to 4 parts)
'   Path_Base$("a/b/c.txt")             "c.txt"
'   Path_Dir$("a/b/c.txt")              "a/b"
'   Path_Ext$("a/b/c.txt")              ".txt"
'   Path_Stem$("a/b/c.txt")             "c"
'   Path_Exists&(p$)                    -1 for a file or a folder
'===============================================================================

' "\" on Windows, "/" elsewhere.
FUNCTION Path_Sep$ ()
    IF INSTR(_OS$, "WIN") THEN Path_Sep$ = "\" ELSE Path_Sep$ = "/"
END FUNCTION

' p with the computer's separator and no doubled separators.
FUNCTION Path_Normalize$ (p AS STRING)
    DIM i AS LONG, c AS STRING, r AS STRING, sep AS STRING, last AS LONG
    sep = Path_Sep$
    FOR i = 1 TO LEN(p)
        c = MID$(p, i, 1)
        IF c = "\" OR c = "/" THEN
            IF last = 0 THEN r = r + sep
            last = -1
        ELSE
            r = r + c
            last = 0
        END IF
    NEXT
    Path_Normalize$ = r
END FUNCTION

' The parts joined with the separator (Python os.path.join).
FUNCTION Path_Join$ (a AS STRING, b AS STRING, OPTIONAL c AS STRING = "", OPTIONAL d AS STRING = "")
    Path_Join$ = Path_Join2$(Path_Join2$(Path_Join2$(a, b), c), d)
END FUNCTION

FUNCTION Path_Join2$ (a AS STRING, b AS STRING)
    DIM x AS STRING, y AS STRING, sep AS STRING
    sep = Path_Sep$
    x = Path_Normalize$(a)
    y = Path_Normalize$(b)
    IF LEN(x) = 0 THEN Path_Join2$ = y: EXIT FUNCTION
    IF LEN(y) = 0 THEN Path_Join2$ = x: EXIT FUNCTION
    IF LEFT$(y, 1) = sep THEN y = MID$(y, 2)
    IF RIGHT$(x, 1) = sep THEN Path_Join2$ = x + y ELSE Path_Join2$ = x + sep + y
END FUNCTION

' Position of the last separator in p (0 when there is none).
FUNCTION Path_LastSep& (p AS STRING)
    DIM i AS LONG
    FOR i = LEN(p) TO 1 STEP -1
        IF MID$(p, i, 1) = "\" OR MID$(p, i, 1) = "/" THEN Path_LastSep& = i: EXIT FUNCTION
    NEXT
END FUNCTION

' The last part of p: the file name (Python basename).
FUNCTION Path_Base$ (p AS STRING)
    Path_Base$ = MID$(p, Path_LastSep&(p) + 1)
END FUNCTION

' p without its last part: the folder (Python dirname).
FUNCTION Path_Dir$ (p AS STRING)
    DIM n AS LONG
    n = Path_LastSep&(p)
    IF n = 0 THEN EXIT FUNCTION
    IF n = 1 THEN Path_Dir$ = Path_Sep$ ELSE Path_Dir$ = Path_Normalize$(LEFT$(p, n - 1))
END FUNCTION

' The extension of the file name with its dot (".txt"), "" when there is none.
FUNCTION Path_Ext$ (p AS STRING)
    DIM b AS STRING, i AS LONG
    b = Path_Base$(p)
    FOR i = LEN(b) TO 2 STEP -1
        IF MID$(b, i, 1) = "." THEN Path_Ext$ = MID$(b, i): EXIT FUNCTION
    NEXT
END FUNCTION

' The file name without its extension.
FUNCTION Path_Stem$ (p AS STRING)
    DIM b AS STRING
    b = Path_Base$(p)
    Path_Stem$ = LEFT$(b, LEN(b) - LEN(Path_Ext$(p)))
END FUNCTION

' -1 when p is a file or a folder.
FUNCTION Path_Exists& (p AS STRING)
    IF _FILEEXISTS(p) THEN Path_Exists& = -1 ELSE Path_Exists& = _DIREXISTS(p)
END FUNCTION
