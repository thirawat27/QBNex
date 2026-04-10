'=============================================================================
' QBNex Settings Module
' Copyright © 2026 thirawat27
' Version: 1.0.0
' Description: Compiler settings and configuration management
'=============================================================================

DEFLNG A-Z

DIM SHARED SettingVerbose AS _BYTE
DIM SHARED SettingDebugMode AS _BYTE
DIM SHARED SettingCompileOnly AS _BYTE
DIM SHARED SettingOptimizationLevel AS INTEGER
DIM SHARED SettingShowProgress AS _BYTE
DIM SHARED SettingCleanTemp AS _BYTE

'Settings initialization
SUB init_settings
    SettingVerbose = 0
    SettingDebugMode = 0
    SettingCompileOnly = 0
    SettingOptimizationLevel = 2
    SettingShowProgress = -1
    SettingCleanTemp = -1
END SUB

'=============================================================================
' Load settings from configuration file
'=============================================================================
SUB load_settings (configFile AS STRING)
    IF _FILEEXISTS(configFile$) = 0 THEN
        CALL init_settings
        EXIT SUB
    END IF
    
    DIM fileNum AS INTEGER
    fileNum = FREEFILE
    OPEN configFile$ FOR INPUT AS #fileNum
    
    DIM line$ AS STRING
    DIM section$ AS STRING
    
    DO WHILE NOT EOF(fileNum)
        LINE INPUT #fileNum, line$
        line$ = _TRIM$(line$)
        
        'Skip empty lines and comments
        IF LEN(line$) = 0 OR LEFT$(line$, 1) = "#" OR LEFT$(line$, 1) = ";" THEN
            CONTINUE DO
        END IF
        
        'Check for section header
        IF LEFT$(line$, 1) = "[" AND INSTR(line$, "]") > 0 THEN
            section$ = MID$(line$, 2, INSTR(line$, "]") - 2)
            section$ = LCASE$(section$)
            CONTINUE DO
        END IF
        
        'Parse key=value pairs
        DIM eqPos AS INTEGER
        eqPos = INSTR(line$, "=")
        IF eqPos > 0 THEN
            DIM key$ AS STRING, value$ AS STRING
            key$ = LCASE$(_TRIM$(LEFT$(line$, eqPos - 1)))
            value$ = _TRIM$(MID$(line$, eqPos + 1))
            
            SELECT CASE section$
                CASE "compiler"
                    SELECT CASE key$
                        CASE "verbose"
                            IF LCASE$(value$) = "true" OR value$ = "-1" THEN SettingVerbose = -1 ELSE SettingVerbose = 0
                        CASE "debug_mode"
                            IF LCASE$(value$) = "true" OR value$ = "-1" THEN SettingDebugMode = -1 ELSE SettingDebugMode = 0
                        CASE "optimization_level"
                            SettingOptimizationLevel = VAL(value$)
                        CASE "show_progress"
                            IF LCASE$(value$) = "true" OR value$ = "-1" THEN SettingShowProgress = -1 ELSE SettingShowProgress = 0
                        CASE "clean_temp"
                            IF LCASE$(value$) = "true" OR value$ = "-1" THEN SettingCleanTemp = -1 ELSE SettingCleanTemp = 0
                    END SELECT
            END SELECT
        END IF
    LOOP
    
    CLOSE #fileNum
END SUB

'=============================================================================
' Save settings to configuration file
'=============================================================================
SUB save_settings (configFile AS STRING)
    DIM fileNum AS INTEGER
    fileNum = FREEFILE
    OPEN configFile$ FOR OUTPUT AS #fileNum
    
    PRINT #fileNum, "# QBNex Configuration File"
    PRINT #fileNum, "# Copyright © 2026 thirawat27"
    PRINT #fileNum, "# Version: 1.0.0"
    PRINT #fileNum, ""
    PRINT #fileNum, "[compiler]"
    PRINT #fileNum, "verbose = "; IIF$(SettingVerbose, "true", "false")
    PRINT #fileNum, "debug_mode = "; IIF$(SettingDebugMode, "true", "false")
    PRINT #fileNum, "optimization_level = "; SettingOptimizationLevel
    PRINT #fileNum, "show_progress = "; IIF$(SettingShowProgress, "true", "false")
    PRINT #fileNum, "clean_temp = "; IIF$(SettingCleanTemp, "true", "false")
    
    CLOSE #fileNum
END SUB

'=============================================================================
' Helper function for IIF
'=============================================================================
FUNCTION IIF$ (condition AS _BYTE, trueVal AS STRING, falseVal AS STRING)
    IF condition THEN
        IIF$ = trueVal$
    ELSE
        IIF$ = falseVal$
    END IF
END FUNCTION

'Initialize on module load
CALL init_settings
