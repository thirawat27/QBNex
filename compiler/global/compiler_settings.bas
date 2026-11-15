DIM SHARED SaveExeWithSource AS _BYTE, IgnoreWarnings AS _BYTE
DIM SHARED qbnexversionprinted AS _BYTE
DIM SHARED generalSettingsSection$, ConfigFile$, DebugInfoIniWarning$

ConfigFile$ = "target/qb-config.ini"
generalSettingsSection$ = "COMPILER SETTINGS"
DebugInfoIniWarning$ = " 'Managed by qbnex -s"

IniSetAddQuotes 0
IniSetForceReload -1
IniSetAllowBasicComments -1
IniSetAutoCommit -1

SaveExeWithSource = ReadWriteBooleanSettingValue%(generalSettingsSection$, "SaveExeWithSource", 0)
IgnoreWarnings = ReadWriteBooleanSettingValue%(generalSettingsSection$, "IgnoreWarnings", 0)
'the file is written only when a setting is missing or invalid (-s writes the ones it changes):
'rewriting it on every start fails on a read-only install and races between parallel builds
IF TFStringToBool%(ReadSetting$(ConfigFile$, generalSettingsSection$, "DebugInfo")) = -2 THEN
    WriteConfigSetting generalSettingsSection$, "DebugInfo", "False" + DebugInfoIniWarning$
END IF
compilerdebuginfo = ReadWriteBooleanSettingValue%(generalSettingsSection$, "DebugInfo", 0)

