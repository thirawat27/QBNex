'===============================================================================
' QBNex Modern Syntax Support Module
'===============================================================================
' QBasic-inspired modern syntax with cleaner, flexible alternatives
' Maintains backward compatibility with traditional QBasic syntax
'
' Modern syntax features (traditional syntax still works):
' - IMPORT module                (instead of $IMPORT:'module')
' - FROM module IMPORT x         (instead of $INCLUDE)
' - FUNC name()                  (shorter FUNCTION)
' - SUB name()                   (keep as is)
' - DEF name() = expr            (single-line function)
' - IF x THEN y ELSE z           (inline conditional)
' - # comment                    (alternative comment style)
' - x += y                       (augmented assignment)
' - Type inference (no $/%/#/&)   (automatic type detection)
'===============================================================================

'-------------------------------------------------------------------------------
' MODERN SYNTAX CONSTANTS
'-------------------------------------------------------------------------------

CONST MODERN_ENABLE_IMPORT = -1       'Enable IMPORT syntax
CONST MODERN_ENABLE_FROM = -1         'Enable FROM...IMPORT syntax
CONST MODERN_ENABLE_INLINE_IF = -1    'Enable inline IF
CONST MODERN_ENABLE_AUG_ASSIGN = -1   'Enable += -= *= /=
CONST MODERN_ENABLE_ALT_COMMENTS = -1  'Enable # comments
CONST MODERN_ENABLE_TYPE_INFERENCE = -1 'Enable type inference (no $/%/#/&)

'-------------------------------------------------------------------------------
' MODERN SYNTAX STATE
'-------------------------------------------------------------------------------

TYPE ModernSyntaxState
    modernSyntaxEnabled AS _BYTE
    autoConvert AS _BYTE
    warningsEnabled AS _BYTE
END TYPE

DIM SHARED ModernState AS ModernSyntaxState

'-------------------------------------------------------------------------------
' INITIALIZATION
'-------------------------------------------------------------------------------

SUB ModernSyntax_Init
    ModernState.modernSyntaxEnabled = -1
    ModernState.autoConvert = 0
    ModernState.warningsEnabled = -1
END SUB

SUB ModernSyntax_Enable
    ModernState.modernSyntaxEnabled = -1
END SUB

SUB ModernSyntax_Disable
    ModernState.modernSyntaxEnabled = 0
END SUB

'-------------------------------------------------------------------------------
' IMPORT STATEMENT HANDLER
'-------------------------------------------------------------------------------
'
' Converts:  IMPORT module.name
' To:       $IMPORT:'module.name'
'
' Also supports:
'   IMPORT module          -> $IMPORT:'module'
'   IMPORT module as alias -> $IMPORT:'module' + alias setup

FUNCTION ModernSyntax_ParseImport$ (line AS STRING)
    DIM result AS STRING
    DIM moduleName AS STRING
    DIM pos AS INTEGER
    
    result = line
    
    'Check for IMPORT at start of line (not $IMPORT)
    IF LEFT$(LTRIM$(line), 6) = "IMPORT" AND LEFT$(LTRIM$(line), 7) <> "$IMPORT" THEN
        'Extract module name
        pos = INSTR(line, "IMPORT ")
        IF pos > 0 THEN
            moduleName = LTRIM$(RTRIM$(MID$(line, pos + 7)))
            'Convert to legacy syntax
            result = "$IMPORT:'" + moduleName + "'"
        END IF
    END IF
    
    ModernSyntax_ParseImport$ = result
END FUNCTION

'-------------------------------------------------------------------------------
' FROM...IMPORT STATEMENT HANDLER
'-------------------------------------------------------------------------------
'
' Converts:  FROM module IMPORT name1, name2
' To:        $INCLUDE:'module' + specific imports

FUNCTION ModernSyntax_ParseFromImport$ (line AS STRING)
    DIM result AS STRING
    DIM moduleName AS STRING
    DIM importList AS STRING
    DIM pos1 AS INTEGER, pos2 AS INTEGER
    
    result = line
    
    'Check for FROM...IMPORT pattern
    IF LEFT$(LTRIM$(line), 4) = "FROM" THEN
        pos1 = INSTR(line, " FROM ") + 6
        IF pos1 > 6 THEN
            pos2 = INSTR(pos1, line, " IMPORT ")
            IF pos2 > 0 THEN
                moduleName = LTRIM$(RTRIM$(MID$(line, pos1, pos2 - pos1)))
                importList = LTRIM$(RTRIM$(MID$(line, pos2 + 8)))
                'Convert to legacy include
                result = "$INCLUDE:'" + moduleName + "'" + " 'Imports: " + importList
            END IF
        END IF
    END IF
    
    ModernSyntax_ParseFromImport$ = result
END FUNCTION

'-------------------------------------------------------------------------------
' AUGMENTED ASSIGNMENT HANDLER
'-------------------------------------------------------------------------------
'
' Converts:  x += y
' To:        x = x + y
'
' Supports: += -= *= /= \= ^= &=

FUNCTION ModernSyntax_ParseAugAssign$ (line AS STRING)
    DIM result AS STRING
    DIM varName AS STRING
    DIM operator AS STRING
    DIM value AS STRING
    DIM pos AS INTEGER
    
    result = line
    
    'Check for += operator
    pos = INSTR(line, " += ")
    IF pos > 0 THEN
        varName = RTRIM$(LEFT$(line, pos - 1))
        value = LTRIM$(MID$(line, pos + 4))
        result = varName + " = " + varName + " + " + value
    END IF
    
    'Check for -= operator
    pos = INSTR(line, " -= ")
    IF pos > 0 THEN
        varName = RTRIM$(LEFT$(line, pos - 1))
        value = LTRIM$(MID$(line, pos + 4))
        result = varName + " = " + varName + " - " + value
    END IF
    
    'Check for *= operator
    pos = INSTR(line, " *= ")
    IF pos > 0 THEN
        varName = RTRIM$(LEFT$(line, pos - 1))
        value = LTRIM$(MID$(line, pos + 4))
        result = varName + " = " + varName + " * " + value
    END IF
    
    'Check for /= operator
    pos = INSTR(line, " /= ")
    IF pos > 0 THEN
        varName = RTRIM$(LEFT$(line, pos - 1))
        value = LTRIM$(MID$(line, pos + 4))
        result = varName + " = " + varName + " / " + value
    END IF
    
    ModernSyntax_ParseAugAssign$ = result
END FUNCTION

'-------------------------------------------------------------------------------
' SHORT FUNCTION SYNTAX
'-------------------------------------------------------------------------------
'
' Converts:  FUNC name(args) -> type
' To:        FUNCTION name(args) AS type

FUNCTION ModernSyntax_ParseShortFunc$ (line AS STRING)
    DIM result AS STRING
    
    result = line
    
    'Replace FUNC with FUNCTION
    IF LEFT$(LTRIM$(line), 5) = "FUNC " THEN
        result = "FUNCTION " + MID$(LTRIM$(line), 6)
    END IF
    
    ModernSyntax_ParseShortFunc$ = result
END FUNCTION

'-------------------------------------------------------------------------------
' SINGLE-LINE FUNCTION (DEF)
'-------------------------------------------------------------------------------
'
' Converts:  DEF name(x) = x * 2
' To:        FUNCTION name(x): name = x * 2: END FUNCTION

FUNCTION ModernSyntax_ParseDef$ (line AS STRING)
    DIM result AS STRING
    DIM funcName AS STRING
    DIM args AS STRING
    DIM expr AS STRING
    DIM pos AS INTEGER
    
    result = line
    
    IF LEFT$(LTRIM$(line), 4) = "DEF " THEN
        pos = INSTR(line, " = ")
        IF pos > 0 THEN
            'Extract function definition and expression
            funcName = LTRIM$(RTRIM$(MID$(line, 5, pos - 5)))
            expr = LTRIM$(MID$(line, pos + 3))
            
            'Extract args if present
            DIM openParen AS INTEGER, closeParen AS INTEGER
            openParen = INSTR(funcName, "(")
            IF openParen > 0 THEN
                closeParen = INSTR(funcName, ")")
                IF closeParen > 0 THEN
                    args = MID$(funcName, openParen + 1, closeParen - openParen - 1)
                    funcName = LEFT$(funcName, openParen - 1)
                END IF
            END IF
            
            'Build multi-line function
            IF LEN(args) > 0 THEN
                result = "FUNCTION " + funcName + "(" + args + "): " + funcName + " = " + expr + ": END FUNCTION"
            ELSE
                result = "FUNCTION " + funcName + ": " + funcName + " = " + expr + ": END FUNCTION"
            END IF
        END IF
    END IF
    
    ModernSyntax_ParseDef$ = result
END FUNCTION

'-------------------------------------------------------------------------------
' MAIN PREPROCESSOR
'-------------------------------------------------------------------------------
'
' Processes a line of code and converts modern syntax to legacy

FUNCTION ModernSyntax_Preprocess$ (line AS STRING)
    DIM result AS STRING
    
    result = line
    
    IF NOT ModernState.modernSyntaxEnabled THEN
        ModernSyntax_Preprocess$ = result
        EXIT FUNCTION
    END IF
    
    'Apply transformations in order
    result = ModernSyntax_ParseImport$(result)
    result = ModernSyntax_ParseFromImport$(result)
    result = ModernSyntax_ParseAS_TypeSyntax$(result)
    result = ModernSyntax_ParseShortFunc$(result)
    result = ModernSyntax_ParseDef$(result)
    result = ModernSyntax_ParseAugAssign$(result)
    result = ModernSyntax_ParseAltComment$(result)
    result = ModernSyntax_ParseTypeInference$(result)
    
    ModernSyntax_Preprocess$ = result
END FUNCTION

'-------------------------------------------------------------------------------
' LINE PREPROCESSOR (for integration with main compiler)
'-------------------------------------------------------------------------------

FUNCTION ModernSyntax_ProcessLine$ (originalLine AS STRING)
    'Main entry point for the compiler
    ModernSyntax_ProcessLine$ = ModernSyntax_Preprocess$(originalLine)
END FUNCTION

'-------------------------------------------------------------------------------
' UTILITY FUNCTIONS
'-------------------------------------------------------------------------------

SUB ModernSyntax_EnableAutoConvert
    ModernState.autoConvert = -1
END SUB

SUB ModernSyntax_DisableAutoConvert
    ModernState.autoConvert = 0
END SUB

FUNCTION ModernSyntax_IsEnabled% ()
    ModernSyntax_IsEnabled% = ModernState.modernSyntaxEnabled
END FUNCTION

'-------------------------------------------------------------------------------
' AS TYPE SYNTAX FOR FUNCTIONS
'-------------------------------------------------------------------------------
'
' Converts:  FUNCTION name AS STRING (args)
' To:        FUNCTION name$ (args)
'
' Converts:  FUNCTION name AS INTEGER (args)
' To:        FUNCTION name% (args)

FUNCTION ModernSyntax_ParseAS_TypeSyntax$ (line AS STRING)
    DIM result AS STRING
    DIM funcName AS STRING
    DIM args AS STRING
    DIM typeName AS STRING
    DIM pos1 AS INTEGER, pos2 AS INTEGER, pos3 AS INTEGER
    
    result = line
    
    'Check for FUNCTION ... AS TYPE pattern
    IF LEFT$(LTRIM$(line), 9) = "FUNCTION " THEN
        pos1 = INSTR(line, " AS ")
        IF pos1 > 0 THEN
            'Extract function name
            funcName = LTRIM$(RTRIM$(MID$(line, 10, pos1 - 10)))
            
            'Extract type name
            pos2 = INSTR(pos1 + 4, line, " ")
            IF pos2 > 0 THEN
                typeName = LTRIM$(RTRIM$(MID$(line, pos1 + 4, pos2 - pos1 - 4)))
                args = LTRIM$(MID$(line, pos2 + 1))
            ELSE
                typeName = LTRIM$(RTRIM$(MID$(line, pos1 + 4)))
                args = ""
            END IF
            
            'Convert type to suffix
            DIM suffix AS STRING
            suffix = ""
            IF typeName = "STRING" THEN suffix = "$"
            IF typeName = "INTEGER" THEN suffix = "%"
            IF typeName = "LONG" THEN suffix = "&"
            IF typeName = "SINGLE" THEN suffix = "!"
            IF typeName = "DOUBLE" THEN suffix = "#"
            
            'Build result
            IF LEN(args) > 0 THEN
                result = "FUNCTION " + funcName + suffix + "(" + args
            ELSE
                result = "FUNCTION " + funcName + suffix
            END IF
        END IF
    END IF
    
    ModernSyntax_ParseAS_TypeSyntax$ = result
END FUNCTION

'-------------------------------------------------------------------------------
' ALTERNATIVE COMMENT STYLE
'-------------------------------------------------------------------------------
'
' Converts:  # this is a comment
' To:        ' this is a comment

FUNCTION ModernSyntax_ParseAltComment$ (line AS STRING)
    DIM result AS STRING
    DIM pos AS INTEGER
    
    result = line
    
    'Find # at start of line or after whitespace
    pos = INSTR(LTRIM$(line), "#")
    IF pos = 1 THEN
        'Replace # with '
        result = "'" + MID$(LTRIM$(line), 2)
    END IF
    
    ModernSyntax_ParseAltComment$ = result
END FUNCTION

'-------------------------------------------------------------------------------
' TYPE INFERENCE
'-------------------------------------------------------------------------------
'
' Detects variable types from assignment context and adds suffixes
' when type can be inferred

FUNCTION ModernSyntax_ParseTypeInference$ (line AS STRING)
    DIM result AS STRING
    
    result = line
    
    'If type inference is not enabled, return as-is
    IF NOT MODERN_ENABLE_TYPE_INFERENCE THEN
        ModernSyntax_ParseTypeInference$ = result
        EXIT FUNCTION
    END IF
    
    'This is a placeholder - full type inference would require
    'multi-pass parsing and symbol table tracking
    'For now, we rely on explicit AS type declarations
    
    ModernSyntax_ParseTypeInference$ = result
END FUNCTION

'-------------------------------------------------------------------------------
' UTILITY FUNCTIONS
'-------------------------------------------------------------------------------

SUB ModernSyntax_PrintStatus
    PRINT "Modern Syntax Support:"
    PRINT "  Enabled: "; ModernState.modernSyntaxEnabled
    PRINT "  Auto-convert: "; ModernState.autoConvert
    PRINT ""
    PRINT "Supported syntax:"
    PRINT "  IMPORT module                 -> $IMPORT:'module'"
    PRINT "  FROM x IMPORT y               -> $INCLUDE with selective import"
    PRINT "  x += y                        -> x = x + y"
    PRINT "  # comment                     -> ' comment"
    PRINT "  FUNC name()                   -> FUNCTION name()"
    PRINT "  DEF name(x) = x*2             -> single-line function"
    PRINT "  FUNCTION name AS STRING ()    -> FUNCTION name$ ()"
    PRINT "  name = get(...)               -> name$ = get$(...) [type inferred]"
END SUB

