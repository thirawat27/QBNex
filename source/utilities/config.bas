SUB WriteConfigSetting (section$, item$, value$)
    WriteSetting ConfigFile$, section$, item$, value$
END SUB

FUNCTION ReadConfigSetting (section$, item$, value$)
    value$ = ReadSetting$(ConfigFile$, section$, item$)
    ReadConfigSetting = (LEN(value$) > 0)
END FUNCTION

' Convert a boolean value to 'True' or 'False'.
FUNCTION BoolToTFString$ (b AS LONG)
    IF b THEN BoolToTFString$ = "True" ELSE BoolToTFString$ = "False"
END FUNCTION

' Convert 'True' or 'False' to a boolean value.
' Any string not 'True' or 'False' is returned as -2.
FUNCTION TFStringToBool% (s AS STRING)
    SELECT CASE _TRIM$(UCASE$(s))
      CASE "TRUE": TFStringToBool% = -1
      CASE "FALSE": TFStringToBool% = 0
      CASE ELSE: TFStringToBool% = -2
    END SELECT
END FUNCTION

' Reads the bool setting at section:setting. 
' If it is not there or invalid, writes the default value to it.
FUNCTION ReadWriteBooleanSettingValue% (section AS STRING, setting AS STRING, default AS INTEGER)
    
    DIM checkResult AS INTEGER
    DIM value AS STRING
    DIM result AS INTEGER

    result = ReadConfigSetting(section, setting, value)

    checkResult = TFStringToBool%(value)

    IF checkResult = -2 THEN
        WriteConfigSetting section, setting, BoolToTFString$(default)
        ReadWriteBooleanSettingValue% = default
    ELSE
        ReadWriteBooleanSettingValue% = checkResult
    END IF

END FUNCTION

' Reads the string setting at section:setting. 
' If it is not there or invalid, writes the default value to it.
FUNCTION ReadWriteStringSettingValue$ (section AS STRING, setting AS STRING, default AS STRING)

    DIM value AS STRING
    DIM result AS INTEGER

    result = ReadConfigSetting(section, setting, value)

    IF result = 0 THEN
        WriteConfigSetting section, setting, default
        ReadWriteStringSettingValue$ = default
    ELSE
        ReadWriteStringSettingValue$ = value
    END IF

END FUNCTION

' Reads the integer setting at section:setting. 
' If it is not there or invalid, writes the default value to it.
' Verifies the value is positive and non-zero.
FUNCTION ReadWriteLongSettingValue& (section AS STRING, setting AS STRING, default AS LONG)

    DIM value AS STRING
    DIM result AS INTEGER
    DIM checkResult AS LONG

    result = ReadConfigSetting(section, setting, value)

    checkResult = VAL(value)

    IF result = 0 OR checkResult <= 0 THEN
        WriteConfigSetting section, setting, str2$(default)
        ReadWriteLongSettingValue& = default
    ELSE
        ReadWriteLongSettingValue& = checkResult
    END IF

END FUNCTION