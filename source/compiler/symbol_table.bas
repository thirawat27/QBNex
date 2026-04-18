'===============================================================================
' QBNex Symbol Table Compatibility Module
'===============================================================================
' Stage0-compatible symbol table. Retains the modular compiler API while using
' simpler record layouts during bootstrap/self-host builds.
'===============================================================================

CONST SYM_VARIABLE = 1
CONST SYM_CONSTANT = 2
CONST SYM_FUNCTION = 3
CONST SYM_SUB = 4
CONST SYM_TYPE = 5
CONST SYM_TYPE_MEMBER = 6
CONST SYM_LABEL = 7
CONST SYM_ARRAY = 8

CONST FLAG_SHARED = 1
CONST FLAG_STATIC = 2
CONST FLAG_CONST = 4
CONST FLAG_EXTERNAL = 8
CONST FLAG_PUBLIC = 16
CONST FLAG_PRIVATE = 32
CONST FLAG_REFERENCE = 64
CONST FLAG_OPTIONAL = 128

CONST TYPE_INTEGER = 1
CONST TYPE_LONG = 2
CONST TYPE_SINGLE = 3
CONST TYPE_DOUBLE = 4
CONST TYPE_STRING = 5
CONST TYPE_VARIANT = 6
CONST TYPE_BYTE = 7
CONST TYPE_INTEGER64 = 8
CONST TYPE_FLOAT = 9
CONST TYPE_UDT = 10

CONST SYMTAB_SCOPE_GLOBAL = 1
CONST SYMTAB_SCOPE_FUNCTION = 2
CONST SYMTAB_SCOPE_SUB = 3
CONST SYMTAB_SCOPE_TYPE = 4
CONST SYMTAB_SCOPE_IF = 5
CONST SYMTAB_SCOPE_FOR = 6
CONST SYMTAB_SCOPE_DO = 7
CONST SYMTAB_SCOPE_SELECT = 8

TYPE DataTypeInfo
    typeCode AS INTEGER
    typeName AS STRING * 32
    sizeBytes AS INTEGER
    isUnsigned AS _BYTE
    udtIndex AS INTEGER
END TYPE

TYPE Symbol
    name AS STRING * 256
    symbolType AS INTEGER
    dataType AS DataTypeInfo
    scopeLevel AS INTEGER
    scopeID AS LONG
    flags AS LONG
    arrayDims AS INTEGER
    isArray AS _BYTE
    paramCount AS INTEGER
    returnType AS DataTypeInfo
    isDefined AS _BYTE
    memberCount AS INTEGER
    parentUDT AS INTEGER
    declaredLine AS LONG
    declaredCol AS INTEGER
    declaredFile AS STRING * 256
    stackOffset AS LONG
    globalOffset AS LONG
    refCount AS LONG
    defCount AS LONG
END TYPE

TYPE ScopeInfo
    scopeID AS LONG
    parentScope AS LONG
    scopeType AS INTEGER
    startLine AS LONG
    endLine AS LONG
    symbolStart AS LONG
    symbolCount AS LONG
END TYPE

DIM SHARED Symbols(1 TO 1024) AS Symbol
DIM SHARED SymbolCount&
DIM SHARED SymbolCapacity&
DIM SHARED Scopes(1 TO 100) AS ScopeInfo
DIM SHARED ScopeCount%
DIM SHARED CurrentScope&
DIM SHARED GlobalScope&
DIM SHARED UDTs(1 TO 100) AS Symbol
DIM SHARED UDTCount%
DIM SHARED BuiltinTypes(1 TO 10) AS DataTypeInfo

SUB InitSymbolTable
    SymbolCount& = 0
    SymbolCapacity& = 1024
    ScopeCount% = 0
    CurrentScope& = 0
    GlobalScope& = 0
    UDTCount% = 0
    InitBuiltinTypes
    GlobalScope& = CreateScope%(SYMTAB_SCOPE_GLOBAL, 0, 1, 0)
    CurrentScope& = GlobalScope&
END SUB

SUB InitBuiltinTypes
    BuiltinTypes(TYPE_INTEGER).typeCode = TYPE_INTEGER
    BuiltinTypes(TYPE_INTEGER).typeName = "INTEGER"
    BuiltinTypes(TYPE_INTEGER).sizeBytes = 2

    BuiltinTypes(TYPE_LONG).typeCode = TYPE_LONG
    BuiltinTypes(TYPE_LONG).typeName = "LONG"
    BuiltinTypes(TYPE_LONG).sizeBytes = 4

    BuiltinTypes(TYPE_SINGLE).typeCode = TYPE_SINGLE
    BuiltinTypes(TYPE_SINGLE).typeName = "SINGLE"
    BuiltinTypes(TYPE_SINGLE).sizeBytes = 4

    BuiltinTypes(TYPE_DOUBLE).typeCode = TYPE_DOUBLE
    BuiltinTypes(TYPE_DOUBLE).typeName = "DOUBLE"
    BuiltinTypes(TYPE_DOUBLE).sizeBytes = 8

    BuiltinTypes(TYPE_STRING).typeCode = TYPE_STRING
    BuiltinTypes(TYPE_STRING).typeName = "STRING"
    BuiltinTypes(TYPE_STRING).sizeBytes = 8

    BuiltinTypes(TYPE_VARIANT).typeCode = TYPE_VARIANT
    BuiltinTypes(TYPE_VARIANT).typeName = "VARIANT"
    BuiltinTypes(TYPE_VARIANT).sizeBytes = 16

    BuiltinTypes(TYPE_BYTE).typeCode = TYPE_BYTE
    BuiltinTypes(TYPE_BYTE).typeName = "_BYTE"
    BuiltinTypes(TYPE_BYTE).sizeBytes = 1

    BuiltinTypes(TYPE_INTEGER64).typeCode = TYPE_INTEGER64
    BuiltinTypes(TYPE_INTEGER64).typeName = "_INTEGER64"
    BuiltinTypes(TYPE_INTEGER64).sizeBytes = 8

    BuiltinTypes(TYPE_FLOAT).typeCode = TYPE_FLOAT
    BuiltinTypes(TYPE_FLOAT).typeName = "_FLOAT"
    BuiltinTypes(TYPE_FLOAT).sizeBytes = 16

    BuiltinTypes(TYPE_UDT).typeCode = TYPE_UDT
    BuiltinTypes(TYPE_UDT).typeName = "UDT"
    BuiltinTypes(TYPE_UDT).sizeBytes = 0
END SUB

SUB CleanupSymbolTable
    SymbolCount& = 0
    ScopeCount% = 0
    CurrentScope& = 0
    GlobalScope& = 0
    UDTCount% = 0
END SUB

FUNCTION CreateScope% (scopeType AS INTEGER, parent AS LONG, startLine AS LONG, startCol AS INTEGER)
    IF ScopeCount% >= 100 THEN
        CreateScope% = 0
        EXIT FUNCTION
    END IF

    ScopeCount% = ScopeCount% + 1
    Scopes(ScopeCount%).scopeID = ScopeCount%
    Scopes(ScopeCount%).parentScope = parent
    Scopes(ScopeCount%).scopeType = scopeType
    Scopes(ScopeCount%).startLine = startLine
    Scopes(ScopeCount%).endLine = 0
    Scopes(ScopeCount%).symbolStart = SymbolCount& + 1
    Scopes(ScopeCount%).symbolCount = 0
    CreateScope% = ScopeCount%
END FUNCTION

SUB CloseScope (scopeID AS LONG)
    IF scopeID >= 1 AND scopeID <= ScopeCount% THEN Scopes(scopeID).endLine = 0
END SUB

SUB EnterScope (scopeID AS LONG)
    IF scopeID >= 1 AND scopeID <= ScopeCount% THEN CurrentScope& = scopeID
END SUB

SUB ExitScope
    IF CurrentScope& > 0 THEN
        CloseScope CurrentScope&
        IF Scopes(CurrentScope&).parentScope > 0 THEN
            CurrentScope& = Scopes(CurrentScope&).parentScope
        ELSE
            CurrentScope& = GlobalScope&
        END IF
    END IF
END SUB

FUNCTION GetCurrentScope%
    GetCurrentScope% = CurrentScope&
END FUNCTION

FUNCTION GetGlobalScope%
    GetGlobalScope% = GlobalScope&
END FUNCTION

FUNCTION GetScopeCount%
    GetScopeCount% = ScopeCount%
END FUNCTION

FUNCTION GetScopeParent% (scopeID AS LONG)
    IF scopeID >= 1 AND scopeID <= ScopeCount% THEN
        GetScopeParent% = Scopes(scopeID).parentScope
    ELSE
        GetScopeParent% = 0
    END IF
END FUNCTION

FUNCTION AddSymbol% (symbolName AS STRING, symType AS INTEGER, dType AS DataTypeInfo, symbolFlags AS LONG)
    IF SymbolCount& >= SymbolCapacity& THEN
        IF SymbolCapacity& >= 8192 THEN
            AddSymbol% = 0
            EXIT FUNCTION
        END IF
        SymbolCapacity& = SymbolCapacity& * 2
        REDIM _PRESERVE Symbols(1 TO SymbolCapacity&) AS Symbol
    END IF

    SymbolCount& = SymbolCount& + 1
    Symbols(SymbolCount&).name = symbolName
    Symbols(SymbolCount&).symbolType = symType
    Symbols(SymbolCount&).dataType = dType
    Symbols(SymbolCount&).scopeLevel = GetScopeLevel(CurrentScope&)
    Symbols(SymbolCount&).scopeID = CurrentScope&
    Symbols(SymbolCount&).flags = symbolFlags
    Symbols(SymbolCount&).arrayDims = 0
    Symbols(SymbolCount&).isArray = 0
    Symbols(SymbolCount&).paramCount = 0
    Symbols(SymbolCount&).returnType = dType
    Symbols(SymbolCount&).isDefined = 0
    Symbols(SymbolCount&).memberCount = 0
    Symbols(SymbolCount&).parentUDT = 0
    Symbols(SymbolCount&).declaredLine = 0
    Symbols(SymbolCount&).declaredCol = 0
    Symbols(SymbolCount&).declaredFile = ""
    Symbols(SymbolCount&).stackOffset = 0
    Symbols(SymbolCount&).globalOffset = 0
    Symbols(SymbolCount&).refCount = 0
    Symbols(SymbolCount&).defCount = 0

    IF CurrentScope& >= 1 AND CurrentScope& <= ScopeCount% THEN
        Scopes(CurrentScope&).symbolCount = Scopes(CurrentScope&).symbolCount + 1
    END IF

    AddSymbol% = SymbolCount&
END FUNCTION

FUNCTION AddVariable% (symbolName AS STRING, typeCode AS INTEGER, isArray AS _BYTE, symbolFlags AS LONG)
    DIM dType AS DataTypeInfo
    DIM symIndex AS LONG

    dType = BuiltinTypes(typeCode)
    symIndex = AddSymbol%(symbolName, IIFSymbolType%(isArray), dType, symbolFlags)
    IF symIndex > 0 THEN
        Symbols(symIndex).isArray = isArray
    END IF
    AddVariable% = symIndex
END FUNCTION

FUNCTION IIFSymbolType% (isArray AS _BYTE)
    IF isArray THEN IIFSymbolType% = SYM_ARRAY ELSE IIFSymbolType% = SYM_VARIABLE
END FUNCTION

FUNCTION AddFunction% (symbolName AS STRING, returnTypeCode AS INTEGER, symbolFlags AS LONG)
    DIM dType AS DataTypeInfo
    DIM symIdx%

    dType = BuiltinTypes(TYPE_VARIANT)
    symIdx% = AddSymbol%(symbolName, SYM_FUNCTION, dType, symbolFlags)
    IF symIdx% > 0 THEN Symbols(symIdx%).returnType = BuiltinTypes(returnTypeCode)
    AddFunction% = symIdx%
END FUNCTION

FUNCTION AddSub% (symbolName AS STRING, symbolFlags AS LONG)
    DIM dType AS DataTypeInfo
    dType = BuiltinTypes(TYPE_VARIANT)
    AddSub% = AddSymbol%(symbolName, SYM_SUB, dType, symbolFlags)
END FUNCTION

FUNCTION AddConstant% (symbolName AS STRING, typeCode AS INTEGER, symbolFlags AS LONG)
    DIM dType AS DataTypeInfo
    dType = BuiltinTypes(typeCode)
    AddConstant% = AddSymbol%(symbolName, SYM_CONSTANT, dType, symbolFlags OR FLAG_CONST)
END FUNCTION

FUNCTION AddUDT% (symbolName AS STRING)
    DIM dType AS DataTypeInfo
    DIM symIdx%

    dType = BuiltinTypes(TYPE_UDT)
    symIdx% = AddSymbol%(symbolName, SYM_TYPE, dType, 0)
    IF symIdx% > 0 AND UDTCount% < 100 THEN
        UDTCount% = UDTCount% + 1
        UDTs(UDTCount%) = Symbols(symIdx%)
        Symbols(symIdx%).isDefined = -1
    END IF
    AddUDT% = symIdx%
END FUNCTION

FUNCTION AddLabel% (symbolName AS STRING, lineNum AS LONG)
    DIM dType AS DataTypeInfo
    DIM symIndex AS LONG

    dType = BuiltinTypes(TYPE_VARIANT)
    symIndex = AddSymbol%(symbolName, SYM_LABEL, dType, 0)
    IF symIndex > 0 THEN
        Symbols(symIndex).declaredLine = lineNum
        Symbols(symIndex).isDefined = -1
    END IF
    AddLabel% = symIndex
END FUNCTION

FUNCTION FindSymbol% (symbolName AS STRING, symType AS INTEGER)
    DIM i&

    FOR i& = SymbolCount& TO 1 STEP -1
        IF RTRIM$(Symbols(i&).name) = symbolName THEN
            IF symType = 0 OR Symbols(i&).symbolType = symType THEN
                FindSymbol% = i&
                EXIT FUNCTION
            END IF
        END IF
    NEXT

    FindSymbol% = 0
END FUNCTION

FUNCTION FindGlobalSymbol% (symbolName AS STRING, symType AS INTEGER)
    DIM i&

    FOR i& = 1 TO SymbolCount&
        IF Symbols(i&).scopeID = GlobalScope& AND RTRIM$(Symbols(i&).name) = symbolName THEN
            IF symType = 0 OR Symbols(i&).symbolType = symType THEN
                FindGlobalSymbol% = i&
                EXIT FUNCTION
            END IF
        END IF
    NEXT

    FindGlobalSymbol% = 0
END FUNCTION

FUNCTION FindSymbolInScope% (symbolName AS STRING, scopeID AS LONG, symType AS INTEGER)
    DIM i&

    FOR i& = SymbolCount& TO 1 STEP -1
        IF Symbols(i&).scopeID = scopeID AND RTRIM$(Symbols(i&).name) = symbolName THEN
            IF symType = 0 OR Symbols(i&).symbolType = symType THEN
                FindSymbolInScope% = i&
                EXIT FUNCTION
            END IF
        END IF
    NEXT

    FindSymbolInScope% = 0
END FUNCTION

SUB MarkSymbolDefined (symIndex AS LONG)
    IF symIndex >= 1 AND symIndex <= SymbolCount& THEN
        Symbols(symIndex).isDefined = -1
        Symbols(symIndex).defCount = Symbols(symIndex).defCount + 1
    END IF
END SUB

SUB IncrementRefCount (symIndex AS LONG)
    IF symIndex >= 1 AND symIndex <= SymbolCount& THEN Symbols(symIndex).refCount = Symbols(symIndex).refCount + 1
END SUB

SUB SetArrayDimensions (symIndex AS LONG, dims AS INTEGER, bounds() AS LONG)
    IF symIndex >= 1 AND symIndex <= SymbolCount& THEN
        Symbols(symIndex).arrayDims = dims
        Symbols(symIndex).isArray = -1
    END IF
END SUB

SUB SetParameterCount (symIndex AS LONG, count AS INTEGER)
    IF symIndex >= 1 AND symIndex <= SymbolCount& THEN Symbols(symIndex).paramCount = count
END SUB

FUNCTION GetScopeLevel (scopeID AS LONG)
    DIM level%
    DIM cursor&

    level% = 0
    cursor& = scopeID
    DO WHILE cursor& > 0 AND cursor& <= ScopeCount%
        level% = level% + 1
        cursor& = Scopes(cursor&).parentScope
    LOOP

    GetScopeLevel = level%
END FUNCTION

FUNCTION GetSymbolName$ (symIndex AS LONG)
    IF symIndex >= 1 AND symIndex <= SymbolCount& THEN
        GetSymbolName$ = RTRIM$(Symbols(symIndex).name)
    ELSE
        GetSymbolName$ = ""
    END IF
END FUNCTION

FUNCTION GetSymbolType% (symIndex AS LONG)
    IF symIndex >= 1 AND symIndex <= SymbolCount& THEN
        GetSymbolType% = Symbols(symIndex).symbolType
    ELSE
        GetSymbolType% = 0
    END IF
END FUNCTION

FUNCTION GetSymbolDataType% (symIndex AS LONG)
    IF symIndex >= 1 AND symIndex <= SymbolCount& THEN
        GetSymbolDataType% = Symbols(symIndex).dataType.typeCode
    ELSE
        GetSymbolDataType% = 0
    END IF
END FUNCTION

FUNCTION GetSymbolScope% (symIndex AS LONG)
    IF symIndex >= 1 AND symIndex <= SymbolCount& THEN
        GetSymbolScope% = Symbols(symIndex).scopeID
    ELSE
        GetSymbolScope% = 0
    END IF
END FUNCTION

FUNCTION IsSymbolDefined% (symIndex AS LONG)
    IF symIndex >= 1 AND symIndex <= SymbolCount& THEN
        IsSymbolDefined% = Symbols(symIndex).isDefined
    ELSE
        IsSymbolDefined% = 0
    END IF
END FUNCTION

FUNCTION IsSymbolArray% (symIndex AS LONG)
    IF symIndex >= 1 AND symIndex <= SymbolCount& THEN
        IsSymbolArray% = Symbols(symIndex).isArray
    ELSE
        IsSymbolArray% = 0
    END IF
END FUNCTION

SUB PrintSymbolTableStats
    PRINT "=== Symbol Table Statistics ==="
    PRINT "Symbols: "; SymbolCount&
    PRINT "Scopes: "; ScopeCount%
    PRINT "UDTs: "; UDTCount%
    PRINT "==============================="
END SUB

FUNCTION GetSymbolCount%
    GetSymbolCount% = SymbolCount&
END FUNCTION
