; QBNex Inno Setup Script
; QBasic/QuickBASIC Compiler and Interpreter

#define MyAppName "QBNex"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "thirawat27"
#define MyAppURL "https://github.com/thirawat27/QBNex"
#define MyAppExeName "qb.exe"
#define MyAppIconFile "QBNex.ico"

[Setup]
; NOTE: The value of AppId uniquely identifies this application.
AppId={{A5B6C7D8-E9F0-1234-5678-9ABCDEF01234}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
LicenseFile=..\LICENSE
InfoBeforeFile=INFO.txt
OutputDir=..\target\installer
OutputBaseFilename=QBNex-Setup-{#MyAppVersion}
SetupIconFile=..\assets\{#MyAppIconFile}
UninstallDisplayIcon={app}\{#MyAppIconFile}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
ChangesEnvironment=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "addtopath"; Description: "Add to PATH environment variable (Recommended)"; GroupDescription: "System Integration";

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\{#MyAppIconFile}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion isreadme
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\CHANGELOG.md"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\examples\*.bas"; DestDir: "{app}\examples"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\{#MyAppIconFile}"
Name: "{group}\Examples"; Filename: "{app}\examples"; IconFilename: "{app}\{#MyAppIconFile}"
Name: "{group}\README"; Filename: "{app}\README.md"
Name: "{group}\{cm:ProgramOnTheWeb,{#MyAppName}}"; Filename: "{#MyAppURL}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\{#MyAppIconFile}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Parameters: "--version"; Description: "Show QBNex version"; Flags: postinstall nowait skipifsilent unchecked
Filename: "{app}\README.md"; Description: "View README"; Flags: postinstall shellexec skipifsilent unchecked

[Code]
const
  EnvironmentKey = 'Environment';
  WM_SETTINGCHANGE = $001A;

// Broadcast environment change to all windows
procedure RefreshEnvironment;
var
  S: string;
begin
  S := 'Environment';
  SendBroadcastNotifyMessage(WM_SETTINGCHANGE, 0, CastStringToInteger(S));
end;

// Add path to user PATH environment variable
procedure EnvAddPath(Path: string);
var
  Paths: string;
  P: Integer;
begin
  // Retrieve current user path (use empty string if entry not exists)
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths) then
    Paths := '';

  // Skip if string already found in path
  P := Pos(';' + Uppercase(Path) + ';', ';' + Uppercase(Paths) + ';');
  if P > 0 then
  begin
    Log(Format('Path [%s] already exists in PATH', [Path]));
    exit;
  end;

  // Add path to the end (or beginning if empty)
  if Paths = '' then
    Paths := Path
  else if Copy(Paths, Length(Paths), 1) = ';' then
    Paths := Paths + Path
  else
    Paths := Paths + ';' + Path;

  // Write to registry
  if RegWriteStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths) then
  begin
    Log(Format('Successfully added [%s] to PATH: [%s]', [Path, Paths]));
    RefreshEnvironment;
  end
  else
    Log(Format('Error adding [%s] to PATH', [Path]));
end;

// Remove path from user PATH environment variable
procedure EnvRemovePath(Path: string);
var
  Paths: string;
  P: Integer;
  PathWithSemicolon: string;
begin
  // Skip if registry entry not exists
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths) then
    exit;

  // Try to find path with semicolon
  PathWithSemicolon := Path + ';';
  P := Pos(Uppercase(PathWithSemicolon), Uppercase(Paths));
  
  if P > 0 then
  begin
    // Remove path with semicolon
    Delete(Paths, P, Length(PathWithSemicolon));
  end
  else
  begin
    // Try to find path at the end without semicolon
    PathWithSemicolon := ';' + Path;
    P := Pos(Uppercase(PathWithSemicolon), Uppercase(Paths));
    
    if P > 0 then
      Delete(Paths, P, Length(PathWithSemicolon))
    else
    begin
      // Try to find path as the only entry
      if Uppercase(Paths) = Uppercase(Path) then
        Paths := ''
      else
        exit; // Path not found
    end;
  end;

  // Write to registry
  if RegWriteStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths) then
  begin
    Log(Format('Successfully removed [%s] from PATH', [Path]));
    RefreshEnvironment;
  end
  else
    Log(Format('Error removing [%s] from PATH', [Path]));
end;

// Called after installation
procedure CurStepChanged(CurStep: TSetupStep);
begin
  if (CurStep = ssPostInstall) and IsTaskSelected('addtopath') then
  begin
    Log('Adding QBNex to PATH...');
    EnvAddPath(ExpandConstant('{app}'));
  end;
end;

// Called during uninstallation
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
  begin
    Log('Removing QBNex from PATH...');
    EnvRemovePath(ExpandConstant('{app}'));
  end;
end;
