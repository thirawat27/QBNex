'===============================================================================
' QBNex Symbol Table Module
'===============================================================================
' Symbol table management for variable, function, subroutine, and type tracking.
' Provides efficient symbol lookup and scope management.
'===============================================================================

'-------------------------------------------------------------------------------
' SYMBOL TYPES AND FLAGS
'-------------------------------------------------------------------------------

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

'-------------------------------------------------------------------------------
' DATA TYPE DEFINITIONS
'-------------------------------------------------------------------------------

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

TYPE DataTypeInfo
    typeCode AS INTEGER
    typeName AS STRING * 32
    sizeBytes AS INTEGER
    isUnsigned AS _BYTE
    udtIndex AS INTEGER
END TYPE

'-------------------------------------------------------------------------------
' SYMBOL DEFINITION
'-------------------------------------------------------------------------------

TYPE Symbol
    name AS STRING * 256
    symbolType AS INTEGER
    dataType AS DataTypeInfo
    scopeLevel AS INTEGER
    scopeID AS LONG
    flags AS LONG
    
    ' For variables/arrays
    arrayDims AS INTEGER
    arrayBounds(1 TO 8) AS LONG ' Lower, Upper pairs
    isArray AS _BYTE
    
    ' For functions/subs
    paramCount AS INTEGER
    returnType AS DataTypeInfo
    isDefined AS _BYTE
    
    ' For types
    memberCount AS INTEGER
    parentUDT AS INTEGER
    
    ' Source location
    declaredLine AS LONG
    declaredCol AS INTEGER
    declaredFile AS STRING * 256
    
    ' Memory/offset information
    stackOffset AS LONG
    globalOffset AS LONG
    
    ' Reference tracking
    refCount AS LONG
    defCount AS LONG
END TYPE

'-------------------------------------------------------------------------------
' SCOPE DEFINITION
'-------------------------------------------------------------------------------

TYPE ScopeInfo
    scopeID AS LONG
    parentScope AS LONG
    scopeType AS INTEGER
    startLine AS LONG
    endLine AS LONG
    symbolStart AS LONG
    symbolCount AS LONG
END TYPE

CONST SCOPE_GLOBAL = 1
CONST SCOPE_FUNCTION = 2
CONST SCOPE_SUB = 3
CONST SCOPE_TYPE = 4
CONST SCOPE_IF = 5
CONST SCOPE_FOR = 6
CONST SCOPE_DO = 7
CONST SCOPE_SELECT = 8

'-------------------------------------------------------------------------------
' SYMBOL TABLE STATE
'-------------------------------------------------------------------------------

DIM SHARED Symbols(1 TO 1024) AS Symbol
DIM SHARED SymbolCount AS LONG
DIM SHARED SymbolCapacity AS LONG

DIM SHARED Scopes(1 TO 100) AS ScopeInfo
DIM SHARED ScopeCount AS INTEGER
DIM SHARED CurrentScope AS LONG
DIM SHARED GlobalScope AS LONG

DIM SHARED UDTs(1 TO 100) AS Symbol
DIM SHARED UDTCount AS INTEGER

' Built-in type mapping
DIM SHARED BuiltinTypes(1 TO 10) AS DataTypeInfo

'-------------------------------------------------------------------------------
' INITIALIZATION
'-------------------------------------------------------------------------------

SUB InitSymbolTable
    SymbolCount = 0
    SymbolCapacity = 1024
    
    ScopeCount = 0
    CurrentScope = 0
    
    UDTCount = 0
    
    ' Initialize built-in types
    InitBuiltinTypes
    
    ' Create global scope
    GlobalScope = CreateScope%(SCOPE_GLOBAL, 0, 1, 0)
    CurrentScope = GlobalScope
END SUB

SUB InitBuiltinTypes
    ' INTEGER (%)
    BuiltinTypes(TYPE_INTEGER).typeCode = TYPE_INTEGER
    BuiltinTypes(TYPE_INTEGER).typeName = "INTEGER"
    BuiltinTypes(TYPE_INTEGER).sizeBytes = 2
    BuiltinTypes(TYPE_INTEGER).isUnsigned = 0
    
    ' LONG (&)
    BuiltinTypes(TYPE_LONG).typeCode = TYPE_LONG
    BuiltinTypes(TYPE_LONG).typeName = "LONG"
    BuiltinTypes(TYPE_LONG).sizeBytes = 4
    BuiltinTypes(TYPE_LONG).isUnsigned = 0
    
    ' SINGLE (!)
    BuiltinTypes(TYPE_SINGLE).typeCode = TYPE_SINGLE
    BuiltinTypes(TYPE_SINGLE).typeName = "SINGLE"
    BuiltinTypes(TYPE_SINGLE).sizeBytes = 4
    BuiltinTypes(TYPE_SINGLE).isUnsigned = 0
    
    ' DOUBLE (#)
    BuiltinTypes(TYPE_DOUBLE).typeCode = TYPE_DOUBLE
    BuiltinTypes(TYPE_DOUBLE).typeName = "DOUBLE"
    BuiltinTypes(TYPE_DOUBLE).sizeBytes = 8
    BuiltinTypes(TYPE_DOUBLE).isUnsigned = 0
    
    ' STRING ($)
    BuiltinTypes(TYPE_STRING).typeCode = TYPE_STRING
    BuiltinTypes(TYPE_STRING).typeName = "STRING"
    BuiltinTypes(TYPE_STRING).sizeBytes = 8 ' Pointer + length
    BuiltinTypes(TYPE_STRING).isUnsigned = 0
    
    ' VARIANT
    BuiltinTypes(TYPE_VARIANT).typeCode = TYPE_VARIANT
    BuiltinTypes(TYPE_VARIANT).typeName = "VARIANT"
    BuiltinTypes(TYPE_VARIANT).sizeBytes = 16
    BuiltinTypes(TYPE_VARIANT).isUnsigned = 0
    
    ' _BYTE (%%)
    BuiltinTypes(TYPE_BYTE).typeCode = TYPE_BYTE
    BuiltinTypes(TYPE_BYTE).typeName = "_BYTE"
    BuiltinTypes(TYPE_BYTE).sizeBytes = 1
    BuiltinTypes(TYPE_BYTE).isUnsigned = 0
    
    ' _INTEGER64 (&&)
    BuiltinTypes(TYPE_INTEGER64).typeCode = TYPE_INTEGER64
    BuiltinTypes(TYPE_INTEGER64).typeName = "_INTEGER64"
    BuiltinTypes(TYPE_INTEGER64).sizeBytes = 8
    BuiltinTypes(TYPE_INTEGER64).isUnsigned = 0
    
    ' _FLOAT (##)
    BuiltinTypes(TYPE_FLOAT).typeCode = TYPE_FLOAT
    BuiltinTypes(TYPE_FLOAT).typeName = "_FLOAT"
    BuiltinTypes(TYPE_FLOAT).sizeBytes = 16
    BuiltinTypes(TYPE_FLOAT).isUnsigned = 0
END SUB

SUB CleanupSymbolTable
    SymbolCount = 0
    ScopeCount = 0
    CurrentScope = 0
    UDTCount = 0
END SUB

'-------------------------------------------------------------------------------
' SCOPE MANAGEMENT
'-------------------------------------------------------------------------------

FUNCTION CreateScope% (scopeType AS INTEGER, parent AS LONG, startLine AS LONG, startCol AS INTEGER)
    IF ScopeCount >= 100 THEN
        CreateScope% = 0
        EXIT FUNCTION
    END IF
    
    ScopeCount = ScopeCount + 1
    
    Scopes(ScopeCount).scopeID = ScopeCount
    Scopes(ScopeCount).parentScope = parent
    Scopes(ScopeCount).scopeType = scopeType
    Scopes(ScopeCount).startLine = startLine
    Scopes(ScopeCount).endLine = 0
    Scopes(ScopeCount).symbolStart = SymbolCount + 1
    Scopes(ScopeCount).symbolCount = 0
    
    CreateScope% = ScopeCount
END FUNCTION

SUB CloseScope (scopeID AS LONG)
    IF scopeID < 1 OR scopeID > ScopeCount THEN EXIT SUB
    Scopes(scopeID).endLine = 0 ' Would be set to current line
END SUB

SUB EnterScope (scopeID AS LONG)
    IF scopeID >= 1 AND scopeID <= ScopeCount THEN
        CurrentScope = scopeID
    END IF
END SUB

SUB ExitScope
    IF CurrentScope > 0 THEN
        CloseScope CurrentScope
        IF Scopes(CurrentScope).parentScope > 0 THEN
            CurrentScope = Scopes(CurrentScope).parentScope
        ELSE
            CurrentScope = GlobalScope
        END IF
    END IF
END SUB

FUNCTION GetCurrentScope%
    GetCurrentScope% = CurrentScope
END FUNCTION

FUNCTION GetGlobalScope%
    GetGlobalScope% = GlobalScope
END FUNCTION

FUNCTION GetScopeCount%
    GetScopeCount% = ScopeCount
END FUNCTION

FUNCTION GetScopeParent% (scopeID AS LONG)
    IF scopeID >= 1 AND scopeID <= ScopeCount THEN
        GetScopeParent% = Scopes(scopeID).parentScope
    ELSE
        GetScopeParent% = 0
    END IF
END FUNCTION

'-------------------------------------------------------------------------------
' SYMBOL CREATION
'-------------------------------------------------------------------------------

FUNCTION AddSymbol% (name AS STRING, symType AS INTEGER, dType AS DataTypeInfo, flags AS LONG)
    IF SymbolCount >= SymbolCapacity THEN
        ' Expand symbol table
        SymbolCapacity = SymbolCapacity + 1024
        REDIM _PRESERVE Symbols(1 TO SymbolCapacity) AS Symbol
    END IF
    
    SymbolCount = SymbolCount + 1
    
    Symbols(SymbolCount).name = name
    Symbols(SymbolCount).symbolType = symType
    Symbols(SymbolCount).dataType = dType
    Symbols(SymbolCount).scopeLevel = GetScopeLevel(CurrentScope)
    Symbols(SymbolCount).scopeID = CurrentScope
    Symbols(SymbolCount).flags = flags
    
    Symbols(SymbolCount).isArray = 0
    Symbols(SymbolCount).arrayDims = 0
    Symbols(SymbolCount).paramCount = 0
    Symbols(SymbolCount).isDefined = 0
    Symbols(SymbolCount).memberCount = 0
    Symbols(SymbolCount).parentUDT = 0
    
    Symbols(SymbolCount).declaredLine = 0
    Symbols(SymbolCount).declaredCol = 0
    Symbols(SymbolCount).declaredFile = ""
    
    Symbols(SymbolCount).stackOffset = 0
    Symbols(SymbolCount).globalOffset = 0
    Symbols(SymbolCount).refCount = 0
    Symbols(SymbolCount).defCount = 0
    
    ' Update scope symbol count
    IF CurrentScope >= 1 AND CurrentScope <= ScopeCount THEN
        Scopes(CurrentScope).symbolCount = Scopes(CurrentScope).symbolCount + 1
    END IF
    
    ' Add to hash table for fast lookup
    ' (Integration with optimization.bas hash table)
    DIM ignore AS LONG, flagsOut AS LONG
    ' HashInsert name, SymbolCount, HashTypeFromSymbolType(symType)
    
    AddSymbol% = SymbolCount
END FUNCTION

FUNCTION AddVariable% (name AS STRING, typeCode AS INTEGER, isArray AS _BYTE, flags AS LONG)
    DIM dType AS DataTypeInfo
    dType = BuiltinTypes(typeCode)
    AddVariable% = AddSymbol%(name, IIF(isArray, SYM_ARRAY, SYM_VARIABLE), dType, flags)
END FUNCTION

FUNCTION AddFunction% (name AS STRING, returnTypeCode AS INTEGER, flags AS LONG)
    DIM dType AS DataTypeInfo
    dType = BuiltinTypes(TYPE_VARIANT) ' Functions are callable, not data
    
    DIM symIdx AS LONG
    symIdx = AddSymbol%(name, SYM_FUNCTION, dType, flags)
    
    IF symIdx > 0 THEN
        Symbols(symIdx).returnType = BuiltinTypes(returnTypeCode)
    END IF
    
    AddFunction% = symIdx
END FUNCTION

FUNCTION AddSub% (name AS STRING, flags AS LONG)
    DIM dType AS DataTypeInfo
    dType = BuiltinTypes(TYPE_VARIANT)
    AddSub% = AddSymbol%(name, SYM_SUB, dType, flags)
END FUNCTION

FUNCTION AddConstant% (name AS STRING, typeCode AS INTEGER, flags AS LONG)
    DIM dType AS DataTypeInfo
    dType = BuiltinTypes(typeCode)
    AddConstant% = AddSymbol%(name, SYM_CONSTANT, dType, flags OR FLAG_CONST)
END FUNCTION

FUNCTION AddUDT% (name AS STRING)
    DIM dType AS DataTypeInfo
    dType.typeCode = TYPE_UDT
    dType.typeName = name
    dType.sizeBytes = 0
    dType.isUnsigned = 0
    dType.udtIndex = UDTCount + 1
    
    UDTCount = UDTCount + 1
    IF UDTCount <= 100 THEN
        UDTs(UDTCount) = Symbols(AddSymbol%(name, SYM_TYPE, dType, 0))
    END IF
    
    AddUDT% = UDTCount
END FUNCTION

FUNCTION AddLabel% (name AS STRING, lineNum AS LONG)
    DIM dType AS DataTypeInfo
    dType = BuiltinTypes(TYPE_VARIANT)
    
    DIM symIdx AS LONG
    symIdx = AddSymbol%(name, SYM_LABEL, dType, 0)
    
    IF symIdx > 0 THEN
        Symbols(symIdx).declaredLine = lineNum
    END IF
    
    AddLabel% = symIdx
END FUNCTION

'-------------------------------------------------------------------------------
' SYMBOL LOOKUP
'-------------------------------------------------------------------------------

FUNCTION FindSymbol% (name AS STRING, symType AS INTEGER)
    DIM i AS LONG
    
    ' Search in current scope first, then parent scopes
    DIM scope AS LONG
    scope = CurrentScope
    
    DO WHILE scope > 0
        ' Search symbols in this scope
        FOR i = 1 TO SymbolCount
            IF Symbols(i).scopeID = scope THEN
                IF UCASE$(RTRIM$(Symbols(i).name)) = UCASE$(name) THEN
                    IF symType = 0 OR Symbols(i).symbolType = symType THEN
                        FindSymbol% = i
                        EXIT FUNCTION
                    END IF
                END IF
            END IF
        NEXT
        
        ' Move to parent scope
        IF scope <= ScopeCount THEN
            scope = Scopes(scope).parentScope
        ELSE
            scope = 0
        END IF
    LOOP
    
    FindSymbol% = 0
END FUNCTION

FUNCTION FindGlobalSymbol% (name AS STRING, symType AS INTEGER)
    DIM i AS LONG
    
    FOR i = 1 TO SymbolCount
        IF Symbols(i).scopeID = GlobalScope THEN
            IF UCASE$(RTRIM$(Symbols(i).name)) = UCASE$(name) THEN
                IF symType = 0 OR Symbols(i).symbolType = symType THEN
                    FindGlobalSymbol% = i
                    EXIT FUNCTION
                END IF
            END IF
        END IF
    NEXT
    
    FindGlobalSymbol% = 0
END FUNCTION

FUNCTION FindSymbolInScope% (name AS STRING, scopeID AS LONG, symType AS INTEGER)
    DIM i AS LONG
    
    FOR i = 1 TO SymbolCount
        IF Symbols(i).scopeID = scopeID THEN
            IF UCASE$(RTRIM$(Symbols(i).name)) = UCASE$(name) THEN
                IF symType = 0 OR Symbols(i).symbolType = symType THEN
                    FindSymbolInScope% = i
                    EXIT FUNCTION
                END IF
            END IF
        END IF
    NEXT
    
    FindSymbolInScope% = 0
END FUNCTION

'-------------------------------------------------------------------------------
' SYMBOL MODIFICATION
'-------------------------------------------------------------------------------

SUB MarkSymbolDefined (symIndex AS LONG)
    IF symIndex >= 1 AND symIndex <= SymbolCount THEN
        Symbols(symIndex).isDefined = -1
        Symbols(symIndex).defCount = Symbols(symIndex).defCount + 1
    END IF
END SUB

SUB IncrementRefCount (symIndex AS LONG)
    IF symIndex >= 1 AND symIndex <= SymbolCount THEN
        Symbols(symIndex).refCount = Symbols(symIndex).refCount + 1
    END IF
END SUB

SUB SetArrayDimensions (symIndex AS LONG, dims AS INTEGER, bounds() AS LONG)
    DIM i AS INTEGER
    
    IF symIndex >= 1 AND symIndex <= SymbolCount THEN
        Symbols(symIndex).isArray = -1
        Symbols(symIndex).arrayDims = dims
        
        FOR i = 1 TO dims * 2
            IF i <= 8 THEN
                Symbols(symIndex).arrayBounds(i) = bounds(i)
            END IF
        NEXT
    END IF
END SUB

SUB SetParameterCount (symIndex AS LONG, count AS INTEGER)
    IF symIndex >= 1 AND symIndex <= SymbolCount THEN
        IF Symbols(symIndex).symbolType = SYM_FUNCTION OR Symbols(symIndex).symbolType = SYM_SUB THEN
            Symbols(symIndex).paramCount = count
        END IF
    END IF
END SUB

'-------------------------------------------------------------------------------
' UTILITY FUNCTIONS
'-------------------------------------------------------------------------------

FUNCTION GetScopeLevel (scopeID AS LONG)
    IF scopeID = GlobalScope THEN
        GetScopeLevel = 0
    ELSEIF scopeID > 0 AND scopeID <= ScopeCount THEN
        ' Calculate depth from global
        DIM level AS INTEGER
        DIM parent AS LONG
        
        level = 1
        parent = Scopes(scopeID).parentScope
        
        DO WHILE parent > 0 AND parent <> GlobalScope
            level = level + 1
            IF parent <= ScopeCount THEN
                parent = Scopes(parent).parentScope
            ELSE
                parent = 0
            END IF
        LOOP
        
        GetScopeLevel = level
    ELSE
        GetScopeLevel = 0
    END IF
END FUNCTION

FUNCTION GetSymbolName$ (symIndex AS LONG)
    IF symIndex >= 1 AND symIndex <= SymbolCount THEN
        GetSymbolName$ = RTRIM$(Symbols(symIndex).name)
    ELSE
        GetSymbolName$ = ""
    END IF
END FUNCTION

FUNCTION GetSymbolType% (symIndex AS LONG)
    IF symIndex >= 1 AND symIndex <= SymbolCount THEN
        GetSymbolType% = Symbols(symIndex).symbolType
    ELSE
        GetSymbolType% = 0
    END IF
END FUNCTION

FUNCTION GetSymbolDataType% (symIndex AS LONG)
    IF symIndex >= 1 AND symIndex <= SymbolCount THEN
        GetSymbolDataType% = Symbols(symIndex).dataType.typeCode
    ELSE
        GetSymbolDataType% = 0
    END IF
END FUNCTION

FUNCTION GetSymbolScope% (symIndex AS LONG)
    IF symIndex >= 1 AND symIndex <= SymbolCount THEN
        GetSymbolScope% = Symbols(symIndex).scopeID
    ELSE
        GetSymbolScope% = 0
    END IF
END FUNCTION

FUNCTION IsSymbolDefined% (symIndex AS LONG)
    IF symIndex >= 1 AND symIndex <= SymbolCount THEN
        IsSymbolDefined% = Symbols(symIndex).isDefined
    ELSE
        IsSymbolDefined% = 0
    END IF
END FUNCTION

FUNCTION IsSymbolArray% (symIndex AS LONG)
    IF symIndex >= 1 AND symIndex <= SymbolCount THEN
        IsSymbolArray% = Symbols(symIndex).isArray
    ELSE
        IsSymbolArray% = 0
    END IF
END FUNCTION

'-------------------------------------------------------------------------------
' SYMBOL TABLE STATISTICS
'-------------------------------------------------------------------------------

SUB PrintSymbolTableStats
    DIM i AS LONG
    DIM varCount AS LONG, funcCount AS LONG, subCount AS LONG
    DIM constCount AS LONG, typeCount AS LONG, labelCount AS LONG
    
    varCount = 0: funcCount = 0: subCount = 0
    constCount = 0: typeCount = 0: labelCount = 0
    
    FOR i = 1 TO SymbolCount
        SELECT CASE Symbols(i).symbolType
            CASE SYM_VARIABLE, SYM_ARRAY: varCount = varCount + 1
            CASE SYM_FUNCTION: funcCount = funcCount + 1
            CASE SYM_SUB: subCount = subCount + 1
            CASE SYM_CONSTANT: constCount = constCount + 1
            CASE SYM_TYPE: typeCount = typeCount + 1
            CASE SYM_LABEL: labelCount = labelCount + 1
        END SELECT
    NEXT
    
    PRINT "=== Symbol Table Statistics ==="
    PRINT "Total Symbols: "; SymbolCount
    PRINT "Variables/Arrays: "; varCount
    PRINT "Functions: "; funcCount
    PRINT "Subroutines: "; subCount
    PRINT "Constants: "; constCount
    PRINT "Types/UDTs: "; typeCount
    PRINT "Labels: "; labelCount
    PRINT "Scopes: "; ScopeCount
    PRINT "================================"
END SUB

FUNCTION GetSymbolCount%
    GetSymbolCount% = SymbolCount
END FUNCTION
