# QBNex Installer

This directory contains the Inno Setup script for creating the QBNex Windows installer.

## Files

- `setup.iss` - Inno Setup script
- `INFO.txt` - Information page shown during installation
- `README.md` - This file

## Building the Installer

### Prerequisites

1. Install [Inno Setup 6](https://jrsoftware.org/isdl.php)
2. Build QBNex in release mode:
   ```bash
   cargo build --release
   ```

### Build Steps

**Windows (PowerShell):**
```powershell
.\build-installer.ps1
```

**Windows (Command Prompt):**
```cmd
build-installer.bat
```

**Manual Build:**
```bash
"C:\Program Files (x86)\Inno Setup 6\ISCC.exe" installer\setup.iss
```

The installer will be created in the `installer` directory as `QBNex-Setup-1.0.0.exe`.

## Installer Features

- ✅ Installs QBNex to Program Files
- ✅ Adds to PATH environment variable (optional)
- ✅ Creates Start Menu shortcuts
- ✅ Creates Desktop shortcut (optional)
- ✅ Includes example programs
- ✅ Includes documentation
- ✅ Clean uninstallation

## Customization

Edit `setup.iss` to customize:
- Application name and version
- Installation directory
- File associations
- Registry entries
- Custom actions

Edit `INFO.txt` to customize the information page shown during installation.

## Notes

- The installer requires the release build of QBNex (`target\release\qb.exe`)
- Icon file must be present at `assets\QBNex.ico`
- License file must be present at root directory (`LICENSE`)
