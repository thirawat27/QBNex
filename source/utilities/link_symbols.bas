SUB RegisterResolveStaticFunction (libFile AS STRING, aliasName AS STRING, method AS LONG)
    ResolveStaticFunctions = ResolveStaticFunctions + 1

    IF ResolveStaticFunctions > UBOUND(ResolveStaticFunction_Name) THEN
        REDIM _PRESERVE ResolveStaticFunction_Name(1 TO ResolveStaticFunctions + 100) AS STRING
        REDIM _PRESERVE ResolveStaticFunction_File(1 TO ResolveStaticFunctions + 100) AS STRING
        REDIM _PRESERVE ResolveStaticFunction_Method(1 TO ResolveStaticFunctions + 100) AS LONG
    END IF

    ResolveStaticFunction_File(ResolveStaticFunctions) = libFile
    ResolveStaticFunction_Name(ResolveStaticFunctions) = aliasName
    ResolveStaticFunction_Method(ResolveStaticFunctions) = method
END SUB

FUNCTION ReadCachedFirstLine$ (path AS STRING)
    CONST READ_CACHE_SLOTS = 16
    STATIC cachePath(1 TO READ_CACHE_SLOTS) AS STRING
    STATIC cacheValue(1 TO READ_CACHE_SLOTS) AS STRING
    STATIC nextSlot AS LONG

    FOR i = 1 TO READ_CACHE_SLOTS
        IF cachePath(i) = path THEN
            ReadCachedFirstLine$ = cacheValue(i)
            EXIT FUNCTION
        END IF
    NEXT

    fh = FREEFILE
    OPEN path FOR BINARY AS #fh
    LINE INPUT #fh, cachedLine$
    CLOSE #fh

    nextSlot = nextSlot + 1
    IF nextSlot > READ_CACHE_SLOTS THEN nextSlot = 1

    cachePath(nextSlot) = path
    cacheValue(nextSlot) = cachedLine$
    ReadCachedFirstLine$ = cachedLine$
END FUNCTION
