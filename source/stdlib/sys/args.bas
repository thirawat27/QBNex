' ============================================================================
' QBNex Standard Library - System: Command-Line Arguments
' ============================================================================
' Parse command-line arguments with flags and positionals
' ============================================================================

'$INCLUDE:'../collections/dictionary.bas'
'$INCLUDE:'../collections/list.bas'

TYPE QBNex_Args
    Flags AS QBNex_Dict
    Positionals AS QBNex_List
END TYPE

' ============================================================================
' SUB: Args_Parse
' Parse command-line arguments
' ============================================================================
SUB Args_Parse (args AS QBNex_Args, cmdLine AS STRING)
    DIM i AS LONG
    DIM token AS STRING
    DIM inQuotes AS LONG
    DIM currentToken AS STRING
    DIM flagName AS STRING
    DIM flagValue AS STRING
    DIM expectValue AS LONG
    
    Dict_Init args.Flags
    List_Init args.Positionals
    
    currentToken = ""
    inQuotes = 0
    expectValue = 0
    flagName = ""
    
    ' Tokenize respecting quotes
    FOR i = 1 TO LEN(cmdLine)
        token = MID$(cmdLine, i, 1)
        
        IF token = CHR$(34) THEN ' Quote
        inQuotes = NOT inQuotes
    ELSEIF token = " " AND NOT inQuotes THEN
        IF LEN(currentToken) > 0 THEN
            Args_ProcessToken args, currentToken, flagName, expectValue
            currentToken = ""
        END IF
    ELSE
        currentToken = currentToken + token
    END IF
NEXT i
    
' Process last token
IF LEN(currentToken) > 0 THEN
    Args_ProcessToken args, currentToken, flagName, expectValue
END IF
END SUB

' ============================================================================
' SUB: Args_ProcessToken (Internal)
' Process a single token
' ============================================================================
SUB Args_ProcessToken (args AS QBNex_Args, token AS STRING, flagName AS STRING, expectValue AS LONG)
    DIM eqPos AS LONG
    
    ' Handle --flag=value or -f=value
    eqPos = INSTR(token, "=")
    IF eqPos > 0 AND LEFT$(token, 1) = "-" THEN
        flagName = MID$(token, 1, eqPos - 1)
        IF LEFT$(flagName, 2) = "--" THEN
            flagName = MID$(flagName, 3)
        ELSEIF LEFT$(flagName, 1) = "-" THEN
            flagName = MID$(flagName, 2)
        END IF
        Dict_Set args.Flags, flagName, MID$(token, eqPos + 1)
        flagName = ""
        expectValue = 0
        EXIT SUB
    END IF
    
    ' Handle flag value
    IF expectValue THEN
        Dict_Set args.Flags, flagName, token
        flagName = ""
        expectValue = 0
        EXIT SUB
    END IF
    
    ' Handle --flag or -f
    IF LEFT$(token, 2) = "--" THEN
        flagName = MID$(token, 3)
        expectValue = -1
    ELSEIF LEFT$(token, 1) = "-" THEN
        flagName = MID$(token, 2)
        expectValue = -1
    ELSE
        ' Positional argument
        List_Add args.Positionals, token
    END IF
END SUB

' ============================================================================
' FUNCTION: Args_HasFlag
' Check if flag exists
' ============================================================================
FUNCTION Args_HasFlag& (args AS QBNex_Args, flagName AS STRING)
    Args_HasFlag = Dict_Has(args.Flags, flagName)
END FUNCTION

' ============================================================================
' FUNCTION: Args_Flag
' Get flag value
' ============================================================================
FUNCTION Args_Flag$ (args AS QBNex_Args, flagName AS STRING)
    Args_Flag = Dict_Get(args.Flags, flagName)
END FUNCTION

' ============================================================================
' FUNCTION: Args_FlagOrDefault
' Get flag value with default
' ============================================================================
FUNCTION Args_FlagOrDefault$ (args AS QBNex_Args, flagName AS STRING, defaultValue AS STRING)
    Args_FlagOrDefault = Dict_GetOrDefault(args.Flags, flagName, defaultValue)
END FUNCTION

' ============================================================================
' FUNCTION: Args_Positional
' Get positional argument by index (0-based)
' ============================================================================
FUNCTION Args_Positional$ (args AS QBNex_Args, index AS LONG)
    IF index >= 0 AND index < args.Positionals.Count THEN
        Args_Positional = List_Get(args.Positionals, index)
    ELSE
        Args_Positional = ""
    END IF
END FUNCTION

' ============================================================================
' FUNCTION: Args_PositionalCount
' Get number of positional arguments
' ============================================================================
FUNCTION Args_PositionalCount& (args AS QBNex_Args)
    Args_PositionalCount = args.Positionals.Count
END FUNCTION

' ============================================================================
' SUB: Args_Free
' Free argument structures
' ============================================================================
SUB Args_Free (args AS QBNex_Args)
    Dict_Free args.Flags
    List_Free args.Positionals
END SUB
