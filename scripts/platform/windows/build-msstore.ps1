<#
.SYNOPSIS
  Build Tauri using Microsoft Store-oriented config and collect latest installers in build/.
#>

[CmdletBinding()]
param(
  [string]$ConfigPath = "src-tauri/tauri.microsoftstore.conf.json",
  [string]$OutputDirName = "build"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Resolve-RepoRoot([string]$startDir) {
  $dir = Resolve-Path $startDir
  while ($true) {
    $candidate = $dir.Path
    if (Test-Path (Join-Path $candidate "src-tauri")) { return $candidate }
    $parent = Split-Path -Parent $candidate
    if (-not $parent -or $parent -eq $candidate) {
      throw "Could not resolve repository root from $startDir"
    }
    $dir = Resolve-Path $parent
  }
}

function Ensure-Dir([string]$dir) {
  if (!(Test-Path $dir)) {
    New-Item -ItemType Directory -Path $dir | Out-Null
  }
}

function Slugify([string]$value) {
  $lower = $value.ToLowerInvariant()
  $slug = [regex]::Replace($lower, '[^a-z0-9]+', '-')
  return $slug.Trim('-')
}

function Read-TauriMeta([string]$repoRoot) {
  $confPath = Join-Path $repoRoot "src-tauri\tauri.conf.json"
  if (!(Test-Path $confPath)) {
    throw "Missing src-tauri/tauri.conf.json"
  }

  $conf = Get-Content $confPath -Raw | ConvertFrom-Json
  return @{
    ProductName = [string]$conf.productName
    Version = [string]$conf.version
    Slug = Slugify([string]$conf.productName)
  }
}

function Infer-Locale([string]$msiFileName) {
  $match = [regex]::Match($msiFileName, '_(?<loc>[A-Za-z]{2}-[A-Za-z]{2})\.msi$', 'IgnoreCase')
  if ($match.Success) {
    return $match.Groups['loc'].Value
  }

  return 'default'
}

function Detect-LatestInstallers([string]$repoRoot) {
  $bundleRoot = Join-Path $repoRoot "src-tauri\target\release\bundle"
  $nsisDir = Join-Path $bundleRoot "nsis"
  $msiDir = Join-Path $bundleRoot "msi"

  $exe = $null
  if (Test-Path $nsisDir) {
    $exe = Get-ChildItem $nsisDir -Recurse -File -Filter "*.exe" -ErrorAction SilentlyContinue |
      Sort-Object LastWriteTime -Descending |
      Select-Object -First 1
  }

  $msi = $null
  if (Test-Path $msiDir) {
    $msi = Get-ChildItem $msiDir -Recurse -File -Filter "*.msi" -ErrorAction SilentlyContinue |
      Sort-Object LastWriteTime -Descending |
      Select-Object -First 1
  }

  if (-not $exe -and -not $msi) {
    throw "No Windows installers found under src-tauri/target/release/bundle"
  }

  return @{ Exe = $exe; Msi = $msi }
}

$scriptPath = $MyInvocation.MyCommand.Path
if (-not $scriptPath) {
  throw "Cannot determine script path."
}

$repoRoot = Resolve-RepoRoot (Split-Path -Parent $scriptPath)
$meta = Read-TauriMeta $repoRoot

$configFull = Join-Path $repoRoot $ConfigPath
if (!(Test-Path $configFull)) {
  throw "Config not found: $configFull"
}

$outDir = Join-Path $repoRoot $OutputDirName
Ensure-Dir $outDir

Write-Host "Building with config: $configFull"
Push-Location $repoRoot
try {
  & cargo tauri build --config $configFull
  if ($LASTEXITCODE -ne 0) {
    throw "cargo tauri build failed"
  }
}
finally {
  Pop-Location
}

$det = Detect-LatestInstallers $repoRoot
$copied = @()

if ($det.Exe) {
  $dstExe = Join-Path $outDir ("{0}-v{1}-windows-x64-setup.exe" -f $meta.Slug, $meta.Version)
  Copy-Item -Force $det.Exe.FullName $dstExe
  $copied += $dstExe
  Write-Host "Copied EXE: $dstExe"
}

if ($det.Msi) {
  $locale = Infer-Locale $det.Msi.Name
  $dstMsi = Join-Path $outDir ("{0}-v{1}-windows-x64_{2}.msi" -f $meta.Slug, $meta.Version, $locale)
  Copy-Item -Force $det.Msi.FullName $dstMsi
  $copied += $dstMsi
  Write-Host "Copied MSI: $dstMsi"
}

$hashOut = Join-Path $outDir "artifacts-sha256.txt"
$lines = @()
foreach ($file in $copied) {
  $hash = (Get-FileHash -Algorithm SHA256 -Path $file).Hash.ToLowerInvariant()
  $line = "$hash  $($file.Replace($repoRoot + '\\', ''))"
  $lines += $line
  Write-Host $line
}
$lines | Set-Content -Encoding UTF8 -Path $hashOut

Write-Host "Done. Artifacts in: $outDir"