' =============================================================================
' QBNex System Integration — Command-Line Argument Parser — args.bas
' =============================================================================
'
' Usage:
'
'   '$INCLUDE:'stdlib/sys/args.bas'
'
'   DIM args AS QBNex_Args
'   Args_Parse args, COMMAND$
'
'   PRINT "positional[1]: "; Args_Positional$(args, 1)
'   PRINT "--output val:  "; Args_Flag$(args, "output")
'   PRINT "-v present:    "; Args_HasFlag(args, "v")
'
' =============================================================================

'$INCLUDE:'stdlib/collections/list.bas'
'$INCLUDE:'stdlib/collections/dictionary.bas'

TYPE QBNex_Args
    _positional AS QBNex_List    ' non-flag arguments
    _flags      AS QBNex_Dict    ' flag -> value (empty string if boolean flag)
    _raw        AS STRING        ' original command string
END TYPE

SUB Args_Init (a AS QBNex_Args)
    List_Init a._positional
    Dict_Init a._flags
END SUB

SUB Args_Free (a AS QBNex_Args)
    List_Free a._positional
    Dict_Free a._flags
END SUB

' ---------------------------------------------------------------------------
' SUB  Args_Parse(a, cmdLine$)
'
'   Parses a COMMAND$ style string. Supported syntax:
'     positional      — bare words
'     -flag           — boolean flag
'     --flag          — boolean flag
'     --flag value    — key-value flag
'     -flag=value     — key-value flag (also: --flag=value)
'     "quoted words"  — treated as a single token
' ---------------------------------------------------------------------------
SUB Args_Parse (a AS QBNex_Args, cmdLine$)
    DIM tokens AS QBNex_List
    List_Init tokens

    ' Tokenise respecting quoted strings
    DIM i AS LONG, n AS LONG, tok AS STRING, ch AS STRING, inQ AS LONG
    n = LEN(cmdLine$): tok = "": inQ = 0
    FOR i = 1 TO n
        ch = MID$(cmdLine$, i, 1)
        IF ch = """" THEN
            inQ = NOT inQ
        ELSEIF ch = " " AND NOT inQ THEN
            IF LEN(tok) > 0 THEN List_Add tokens, tok: tok = ""
        ELSE
            tok = tok + ch
        END IF
    NEXT i
    IF LEN(tok) > 0 THEN List_Add tokens, tok

    ' Parse tokens
    DIM t AS LONG, total AS LONG
    total = List_Count&(tokens)
    t = 1
    DO WHILE t <= total
        tok = List_Get$(tokens, t)
        IF LEFT$(tok, 2) = "--" THEN
            DIM longFlag AS STRING
            longFlag = MID$(tok, 3)
            DIM eqPos AS LONG
            eqPos = INSTR(longFlag, "=")
            IF eqPos > 0 THEN
                Dict_Set a._flags, LEFT$(longFlag, eqPos - 1), MID$(longFlag, eqPos + 1)
            ELSEIF t + 1 <= total AND LEFT$(List_Get$(tokens, t + 1), 1) <> "-" THEN
                Dict_Set a._flags, longFlag, List_Get$(tokens, t + 1)
                t = t + 1
            ELSE
                Dict_Set a._flags, longFlag, ""
            END IF
        ELSEIF LEFT$(tok, 1) = "-" AND LEN(tok) > 1 THEN
            DIM shortFlag AS STRING
            shortFlag = MID$(tok, 2)
            eqPos = INSTR(shortFlag, "=")
            IF eqPos > 0 THEN
                Dict_Set a._flags, LEFT$(shortFlag, eqPos - 1), MID$(shortFlag, eqPos + 1)
            ELSEIF t + 1 <= total AND LEFT$(List_Get$(tokens, t + 1), 1) <> "-" THEN
                Dict_Set a._flags, shortFlag, List_Get$(tokens, t + 1)
                t = t + 1
            ELSE
                Dict_Set a._flags, shortFlag, ""
            END IF
        ELSE
            List_Add a._positional, tok
        END IF
        t = t + 1
    LOOP

    List_Free tokens
END SUB

FUNCTION Args_HasFlag& (a AS QBNex_Args, flagName$)
    Args_HasFlag& = Dict_Has&(a._flags, flagName$)
END FUNCTION

FUNCTION Args_Flag$ (a AS QBNex_Args, flagName$)
    Args_Flag$ = Dict_Get$(a._flags, flagName$)
END FUNCTION

FUNCTION Args_FlagOrDefault$ (a AS QBNex_Args, flagName$, default$)
    IF Dict_Has&(a._flags, flagName$) THEN
        Args_FlagOrDefault$ = Dict_Get$(a._flags, flagName$)
    ELSE
        Args_FlagOrDefault$ = default$
    END IF
END FUNCTION

FUNCTION Args_Positional$ (a AS QBNex_Args, idx AS LONG)
    Args_Positional$ = List_Get$(a._positional, idx)
END FUNCTION

FUNCTION Args_PositionalCount& (a AS QBNex_Args)
    Args_PositionalCount& = List_Count&(a._positional)
END FUNCTION

SUB Args_Print (a AS QBNex_Args)
    DIM i AS LONG
    PRINT "Args positional (" + STR$(List_Count&(a._positional)) + "):"
    FOR i = 1 TO List_Count&(a._positional)
        PRINT "  [" + STR$(i) + "] " + List_Get$(a._positional, i)
    NEXT i
    DIM keys AS QBNex_List
    List_Init keys
    Dict_Keys a._flags, keys
    PRINT "Args flags (" + STR$(List_Count&(keys)) + "):"
    FOR i = 1 TO List_Count&(keys)
        DIM k AS STRING
        k = List_Get$(keys, i)
        PRINT "  --" + k + " = [" + Dict_Get$(a._flags, k) + "]"
    NEXT i
    List_Free keys
END SUB
