' ============================================================================
' QBNex Standard Library - I/O: Path Helpers
' ============================================================================

FUNCTION Path_Separator$ ()
    IF INSTR(_OS$, "WIN") THEN
        Path_Separator = "\"
    ELSE
        Path_Separator = "/"
    END IF
END FUNCTION

FUNCTION Path_Normalize$ (rawPath AS STRING)
    DIM i AS LONG
    DIM separator AS STRING
    DIM ch AS STRING
    DIM previousWasSeparator AS LONG
    DIM normalizedPath AS STRING

    separator = Path_Separator$
    FOR i = 1 TO LEN(rawPath)
        ch = MID$(rawPath, i, 1)
        IF ch = "\" OR ch = "/" THEN
            IF previousWasSeparator = 0 THEN
                normalizedPath = normalizedPath + separator
                previousWasSeparator = -1
            END IF
        ELSE
            normalizedPath = normalizedPath + ch
            previousWasSeparator = 0
        END IF
    NEXT

    Path_Normalize = normalizedPath
END FUNCTION

FUNCTION Path_Join$ (basePath AS STRING, leafPath AS STRING)
    DIM separator AS STRING
    DIM baseText AS STRING
    DIM leafText AS STRING

    ' Work on copies: the arguments are passed by reference.
    separator = Path_Separator$
    baseText = Path_Normalize$(basePath)
    leafText = Path_Normalize$(leafPath)

    IF LEN(baseText) = 0 THEN
        Path_Join = leafText
        EXIT FUNCTION
    END IF
    IF LEN(leafText) = 0 THEN
        Path_Join = baseText
        EXIT FUNCTION
    END IF

    IF LEFT$(leafText, 1) = separator THEN leafText = MID$(leafText, 2)
    IF RIGHT$(baseText, 1) = separator THEN
        Path_Join = baseText + leafText
    ELSE
        Path_Join = baseText + separator + leafText
    END IF
END FUNCTION

FUNCTION Path_FileName$ (rawPath AS STRING)
    DIM i AS LONG
    DIM normalized AS STRING
    DIM separator AS STRING
    DIM position AS LONG

    normalized = Path_Normalize$(rawPath)
    separator = Path_Separator$
    position = 0

    FOR i = 1 TO LEN(normalized)
        IF MID$(normalized, i, 1) = separator THEN position = i
    NEXT

    IF position = 0 THEN
        Path_FileName = normalized
    ELSE
        Path_FileName = MID$(normalized, position + 1)
    END IF
END FUNCTION

FUNCTION Path_DirName$ (rawPath AS STRING)
    DIM i AS LONG
    DIM normalized AS STRING
    DIM separator AS STRING
    DIM position AS LONG

    normalized = Path_Normalize$(rawPath)
    separator = Path_Separator$
    position = 0
    FOR i = 1 TO LEN(normalized)
        IF MID$(normalized, i, 1) = separator THEN position = i
    NEXT

    IF position = 0 THEN
        Path_DirName = ""
    ELSEIF position = 1 THEN
        Path_DirName = separator ' the root itself
    ELSE
        Path_DirName = LEFT$(normalized, position - 1)
    END IF
END FUNCTION

FUNCTION Path_Extension$ (rawPath AS STRING)
    DIM i AS LONG
    DIM filename AS STRING
    DIM position AS LONG

    filename = Path_FileName$(rawPath)
    position = 0

    FOR i = 1 TO LEN(filename)
        IF MID$(filename, i, 1) = "." THEN position = i
    NEXT

    IF position = 0 THEN
        Path_Extension = ""
    ELSE
        Path_Extension = MID$(filename, position)
    END IF
END FUNCTION
