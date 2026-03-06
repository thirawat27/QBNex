# QBNex Installer Build Script
# This script builds the QBNex installer using Inno Setup

param(
    [switch]$Release = $true,
    [switch]$SkipBuild = $false
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  QBNex Installer Build Script" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Check if Inno Setup is installed
$InnoSetupPath = "C\Program Files (x86)\Inno Setup 6\ISCC.exe"
if (-not (Test-Path $InnoSetupPath)) {
    Write-Host "ERROR Inno Setup not found at $InnoSetupPath" -ForegroundColor Red
    Write-Host "Please install Inno Setup from https//jrsoftware.org/isdl.php" -ForegroundColor Yellow
    exit 1
}

# Build the project if not skipped
if (-not $SkipBuild) {
    Write-Host "Building QBNex..." -ForegroundColor Green
    
    if ($Release) {
        cargo build --release
    } else {
        cargo build
    }
    
    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR Build failed!" -ForegroundColor Red
        exit 1
    }
    
    Write-Host "Build completed successfully!" -ForegroundColor Green
    Write-Host ""
}

# Check if executable exists
$ExePath = if ($Release) { "target\release\qb.exe" } else { "target\debug\qb.exe" }
if (-not (Test-Path $ExePath)) {
    Write-Host "ERROR Executable not found at $ExePath" -ForegroundColor Red
    exit 1
}

# Check if icon exists
if (-not (Test-Path "assets\QBNex.ico")) {
    Write-Host "WARNING Icon file not found at assets\QBNex.ico" -ForegroundColor Yellow
}

# Create installer output directory
$InstallerDir = "target\installer"
if (-not (Test-Path $InstallerDir)) {
    New-Item -ItemType Directory -Path $InstallerDir | Out-Null
}

# Compile installer
Write-Host "Compiling installer..." -ForegroundColor Green
& $InnoSetupPath "installer\setup.iss"

if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR Installer compilation failed!" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Installer built successfully!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Installer location $InstallerDir\QBNex-Setup-1.0.0.exe" -ForegroundColor Yellow
Write-Host ""
Write-Host "To test the installer" -ForegroundColor Cyan
Write-Host "  .\$InstallerDir\QBNex-Setup-1.0.0.exe" -ForegroundColor White
Write-Host ""
