DIM SHARED SaveExeWithSource AS _BYTE, IgnoreWarnings AS _BYTE
DIM SHARED qb64versionprinted AS _BYTE
DIM SHARED generalSettingsSection$, ConfigFile$, DebugInfoIniWarning$

ConfigFile$ = "internal/config.ini"
generalSettingsSection$ = "COMPILER SETTINGS"
DebugInfoIniWarning$ = " 'Managed by qb64 -s"

IniSetAddQuotes 0
IniSetForceReload -1
IniSetAllowBasicComments -1
IniSetAutoCommit -1

SaveExeWithSource = ReadWriteBooleanSettingValue%(generalSettingsSection$, "SaveExeWithSource", 0)
IgnoreWarnings = ReadWriteBooleanSettingValue%(generalSettingsSection$, "IgnoreWarnings", 0)
compilerdebuginfo = ReadWriteBooleanSettingValue%(generalSettingsSection$, "DebugInfo", 0)

IF compilerdebuginfo THEN
    WriteConfigSetting generalSettingsSection$, "DebugInfo", "True" + DebugInfoIniWarning$
ELSE
    WriteConfigSetting generalSettingsSection$, "DebugInfo", "False" + DebugInfoIniWarning$
END IF

Include_GDB_Debugging_Info = compilerdebuginfo
