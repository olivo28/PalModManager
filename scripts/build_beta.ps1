param (
    [Parameter(Position = 0)]
    [string]$BetaNumber = ""
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Split-Path -Parent $ScriptDir
Set-Location $RootDir

# If no beta number was provided, prompt the user
if ([string]::IsNullOrWhiteSpace($BetaNumber)) {
    Write-Host ""
    Write-Host "==================================================" -ForegroundColor Cyan
    Write-Host "      PalModManager - Beta Build Script" -ForegroundColor Cyan
    Write-Host "==================================================" -ForegroundColor Cyan
    $BetaNumber = Read-Host "Enter Beta Number (e.g. 1 for Beta 1, 2 for Beta 2)"
    if ([string]::IsNullOrWhiteSpace($BetaNumber)) {
        $BetaNumber = "1"
    }
}

# Clean input: strip accidental 'beta' or 'b' prefix and allow numbers, dots, and hyphens
$BetaNumClean = ($BetaNumber -replace '^(?i)beta[-_\s]*', '' -replace '^(?i)b[-_\s]*', '' -replace '[^a-zA-Z0-9._-]', '').Trim()
if ([string]::IsNullOrWhiteSpace($BetaNumClean)) {
    $BetaNumClean = "1"
}

# Read current version from tauri.conf.json (code stays clean, no beta in code)
$TauriConfPath = Join-Path $RootDir "src-tauri/tauri.conf.json"
$TauriJson = Get-Content -Raw $TauriConfPath | ConvertFrom-Json
$BaseVersion = ($TauriJson.version -replace '-.*$', '').Trim()

if ([string]::IsNullOrWhiteSpace($BaseVersion)) {
    $BaseVersion = "1.7.2"
}

Write-Host ""
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "   PalModManager - Building Beta Release" -ForegroundColor Cyan
Write-Host "   Base Version   : $BaseVersion (Code remains untouched)" -ForegroundColor Yellow
Write-Host "   Beta Tag       : Beta $BetaNumClean" -ForegroundColor Yellow
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host ""

# Close any running instances of PalModManager so binaries are not locked
Get-Process -Name "*palmodmanager*" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

# Run standard Tauri release build
Write-Host "Compiling Release binary and installer via 'pnpm tauri build'..." -ForegroundColor Cyan
pnpm tauri build

if ($LASTEXITCODE -ne 0) {
    Write-Host ""
    Write-Host "Build failed with exit code $LASTEXITCODE." -ForegroundColor Red
    exit $LASTEXITCODE
}

Write-Host ""
Write-Host "Build finished. Packaging Beta executables, installers and ZIP archives..." -ForegroundColor Cyan

$ReleaseDir = Join-Path $RootDir "src-tauri/target/release"
$BundleNsis = Join-Path $ReleaseDir "bundle/nsis"
$BundleMsi  = Join-Path $ReleaseDir "bundle/msi"
$DistBeta   = Join-Path $RootDir "dist-beta"

if (-not (Test-Path $DistBeta)) {
    New-Item -ItemType Directory -Path $DistBeta | Out-Null
}

$CreatedArtifacts = @()

# 1. Package NSIS Setup Installer
$BetaSetupName = $null
if (Test-Path $BundleNsis) {
    $SetupExes = Get-ChildItem -Path $BundleNsis -Filter "*_x64-setup.exe" | Where-Object { $_.Name -notmatch "beta" } | Sort-Object LastWriteTime -Descending
    if ($SetupExes.Count -gt 0) {
        $SrcSetup = $SetupExes[0]
        $BetaSetupName = "PalModManager_${BaseVersion}_beta${BetaNumClean}_x64-setup.exe"
        $DestSetup = Join-Path $BundleNsis $BetaSetupName
        Copy-Item -Path $SrcSetup.FullName -Destination $DestSetup -Force
        
        # Also copy to root dist-beta/ for easy access
        $DistSetup = Join-Path $DistBeta $BetaSetupName
        Copy-Item -Path $DestSetup -Destination $DistSetup -Force
        $CreatedArtifacts += $DistSetup
    }
}

# 2. Package Standalone Release Exe
$BetaExeName = $null
$Standalones = Get-ChildItem -Path $ReleaseDir -Filter "palmodmanager.exe" -File
if ($Standalones.Count -gt 0) {
    $SrcExe = $Standalones[0]
    $BetaExeName = "PalModManager_${BaseVersion}_beta${BetaNumClean}.exe"
    $DestExe = Join-Path $ReleaseDir $BetaExeName
    Copy-Item -Path $SrcExe.FullName -Destination $DestExe -Force
    
    # Also copy to root dist-beta/ for easy access
    $DistExe = Join-Path $DistBeta $BetaExeName
    Copy-Item -Path $DestExe -Destination $DistExe -Force
    $CreatedArtifacts += $DistExe
}

# 3. Package MSI Installer if generated
$BetaMsiName = $null
if (Test-Path $BundleMsi) {
    $Msis = Get-ChildItem -Path $BundleMsi -Filter "*.msi" | Where-Object { $_.Name -notmatch "beta" } | Sort-Object LastWriteTime -Descending
    if ($Msis.Count -gt 0) {
        $SrcMsi = $Msis[0]
        $BetaMsiName = "PalModManager_${BaseVersion}_beta${BetaNumClean}_x64.msi"
        $DistMsi = Join-Path $DistBeta $BetaMsiName
        Copy-Item -Path $SrcMsi.FullName -Destination $DistMsi -Force
        $CreatedArtifacts += $DistMsi
    }
}

# 4. Generate Clean ZIP Archives (compressed from within dist-beta so files are at root of zip)
function Compress-ArchiveWithRetry {
    param (
        [string[]]$Path,
        [string]$DestinationPath,
        [int]$MaxRetries = 5,
        [int]$DelaySeconds = 2
    )
    for ($i = 1; $i -le $MaxRetries; $i++) {
        try {
            Compress-Archive -Path $Path -DestinationPath $DestinationPath -Force -ErrorAction Stop
            return
        } catch {
            if ($i -eq $MaxRetries) {
                throw $_
            }
            Write-Host "  [Notice] File temporarily locked (antivirus/indexer). Retrying in $DelaySeconds s ($i/$MaxRetries)..." -ForegroundColor Yellow
            Start-Sleep -Seconds $DelaySeconds
        }
    }
}

Push-Location $DistBeta
try {
    # 4a. ZIP for Setup Installer
    if ($BetaSetupName -and (Test-Path $BetaSetupName)) {
        $ZipSetupName = "PalModManager_${BaseVersion}_beta${BetaNumClean}_x64-setup.zip"
        Write-Host "Compressing Setup Installer into $ZipSetupName..." -ForegroundColor Gray
        Compress-ArchiveWithRetry -Path $BetaSetupName -DestinationPath $ZipSetupName
        $CreatedArtifacts += (Join-Path $DistBeta $ZipSetupName)
    }

    # 4b. ZIP for Portable Standalone EXE
    if ($BetaExeName -and (Test-Path $BetaExeName)) {
        $ZipPortableName = "PalModManager_${BaseVersion}_beta${BetaNumClean}_Portable.zip"
        Write-Host "Compressing Standalone Exe into $ZipPortableName..." -ForegroundColor Gray
        Compress-ArchiveWithRetry -Path $BetaExeName -DestinationPath $ZipPortableName
        $CreatedArtifacts += (Join-Path $DistBeta $ZipPortableName)
    }

    # 4c. ZIP for MSI Installer if present
    if ($BetaMsiName -and (Test-Path $BetaMsiName)) {
        $ZipMsiName = "PalModManager_${BaseVersion}_beta${BetaNumClean}_x64_msi.zip"
        Write-Host "Compressing MSI Installer into $ZipMsiName..." -ForegroundColor Gray
        Compress-ArchiveWithRetry -Path $BetaMsiName -DestinationPath $ZipMsiName
        $CreatedArtifacts += (Join-Path $DistBeta $ZipMsiName)
    }

    # 4d. Full Bundle ZIP (Setup + Portable Exe)
    if ($BetaSetupName -and $BetaExeName -and (Test-Path $BetaSetupName) -and (Test-Path $BetaExeName)) {
        $ZipAllName = "PalModManager_${BaseVersion}_beta${BetaNumClean}_All.zip"
        Write-Host "Compressing Full Bundle (Setup + Portable) into $ZipAllName..." -ForegroundColor Gray
        Compress-ArchiveWithRetry -Path @($BetaSetupName, $BetaExeName) -DestinationPath $ZipAllName
        $CreatedArtifacts += (Join-Path $DistBeta $ZipAllName)
    }
} finally {
    Pop-Location
}

Write-Host ""
Write-Host "==================================================" -ForegroundColor Green
Write-Host "   BETA BUILD & PACKAGING COMPLETED SUCCESSFULLY!" -ForegroundColor Green
Write-Host "==================================================" -ForegroundColor Green
Write-Host ""

if ($CreatedArtifacts.Count -gt 0) {
    Write-Host "Deliverables ready in 'dist-beta/' folder:" -ForegroundColor Yellow
    foreach ($art in $CreatedArtifacts) {
        $FileObj = Get-Item $art
        $SizeMB = [math]::Round($FileObj.Length / 1MB, 2)
        Write-Host "  -> $($FileObj.Name) ($SizeMB MB)" -ForegroundColor White
    }
    Write-Host ""
    Write-Host "Full folder path: $DistBeta" -ForegroundColor Cyan
} else {
    Write-Host "Warning: No output files found to rename in $ReleaseDir" -ForegroundColor Yellow
}

Write-Host ""
