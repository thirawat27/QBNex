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
