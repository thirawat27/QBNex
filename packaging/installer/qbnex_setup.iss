; This script lives in packaging\installer\; the repository root is two levels up.
#define RepoRoot SourcePath + "..\\..\\"

#ifnexist RepoRoot + "qb.exe"
  #error The file "qb.exe" was not found. Build QBNex first (run scripts\setup_win.cmd).
#endif

#ifnexist RepoRoot + "LICENSE"
  #error The file "LICENSE" was not found.
#endif

#define MyAppName "QBNex"
#define MyAppVersion "1.0.3"
#define MyAppPublisher "thirawat27"
#define MyAppURL "https://github.com/thirawat27/QBNex"
#define MyAppExeName "qb.exe"

[Setup]
AppId={{D3DA0999-AD6A-4A2F-AF8A-C4ED13D5D95D}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
AppCopyright=Copyright (c) 2026 Thirawat Sinlapasomsak
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
LicenseFile={#RepoRoot}LICENSE
OutputDir={#RepoRoot}dist
OutputBaseFilename=QBNex-Setup-{#MyAppVersion}
SetupIconFile={#RepoRoot}compiler\qbnex.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x86compatible
ArchitecturesInstallIn64BitMode=x64compatible
ChangesEnvironment=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "addtopath"; Description: "Add QBNex install folder to the system PATH"; Flags: unchecked

[Files]
Source: "{#RepoRoot}qb.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RepoRoot}qb.cmd"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RepoRoot}qbnex.cmd"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RepoRoot}README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RepoRoot}CHANGELOG.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RepoRoot}LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RepoRoot}licenses\*"; DestDir: "{app}\licenses"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#RepoRoot}compiler\*"; DestDir: "{app}\compiler"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#RepoRoot}internal\*"; DestDir: "{app}\internal"; Flags: ignoreversion recursesubdirs createallsubdirs; Excludes: "internal\temp\*"
; Prebuilt Rust runtime (libqb_runtime.a, linked into every program) and QIR runner.
Source: "{#RepoRoot}crates\qb-runtime\os\windows\*"; DestDir: "{app}\crates\qb-runtime\os\windows"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#RepoRoot}crates\qb-exec\bin\windows\*"; DestDir: "{app}\crates\qb-exec\bin\windows"; Flags: ignoreversion recursesubdirs createallsubdirs skipifsourcedoesntexist

[Icons]
Name: "{group}\QBNex Compiler"; Filename: "{app}\qb.cmd"; WorkingDir: "{app}"
Name: "{group}\QBNex README"; Filename: "{app}\README.md"
Name: "{group}\Uninstall QBNex"; Filename: "{uninstallexe}"
Name: "{autodesktop}\QBNex Compiler"; Filename: "{app}\qb.cmd"; WorkingDir: "{app}"; Tasks: desktopicon

[Registry]
Root: HKLM; Subkey: "SYSTEM\CurrentControlSet\Control\Session Manager\Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Check: NeedsAddPath(ExpandConstant('{app}')); Tasks: addtopath; Flags: preservestringtype

[Run]
Filename: "{app}\qb.exe"; Parameters: "--version"; Description: "Verify QBNex installation (show version)"; Flags: postinstall nowait skipifsilent

[Code]
function NeedsAddPath(PathToCheck: string): Boolean;
var
  CurrentPath: string;
begin
  if not RegQueryStringValue(HKEY_LOCAL_MACHINE,
    'SYSTEM\CurrentControlSet\Control\Session Manager\Environment',
    'Path', CurrentPath) then
  begin
    Result := True;
    exit;
  end;

  Result := Pos(';' + UpperCase(PathToCheck) + ';', ';' + UpperCase(CurrentPath) + ';') = 0;
end;
