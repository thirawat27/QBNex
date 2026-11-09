''' Splits the filename from its path and returns the path.
''' Returns: The path or empty if no path.
FUNCTION GetFilePath$ (f$)
    DIM normalizedPath AS STRING
    DIM directoryPath AS STRING
    DIM resultPath AS STRING
    DIM separator AS STRING

    normalizedPath = Path_Normalize$(f$)
    directoryPath = Path_DirName$(normalizedPath)
    IF directoryPath = "" THEN EXIT FUNCTION

    separator = Path_Separator$
    resultPath = directoryPath
    IF RIGHT$(resultPath, 1) <> separator THEN resultPath = resultPath + separator
    GetFilePath$ = resultPath
END FUNCTION

''' Checks if a filename has an extension on the end.
''' Returns: True if provided filename has an extension.
FUNCTION FileHasExtension (f$)
    IF Path_Extension$(f$) <> "" THEN FileHasExtension = -1
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

''' Fixes the provided filename and path to use the correct path separator.
SUB PATH_SLASH_CORRECT (a$)
    a$ = Path_Normalize$(a$)
END SUB
