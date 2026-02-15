<#
.SYNOPSIS
  Sign Tauri Windows installers (.exe/.msi) and emit SHA256 hashes.

.DESCRIPTION
  - If Input/Output paths are provided, signs explicit files.
  - Otherwise auto-detects latest .exe/.msi under src-tauri\target\release\bundle\ and
    writes signed copies to signed/ with standardized names.
#>

[CmdletBinding()]
param(
  [string]$SignToolPath = "",
  [string]$TimestampUrl = "http://timestamp.digicert.com",
  [string]$InputExe = "",
  [string]$OutputExe = "",
  [string]$InputMsi = "",
  [string]$OutputMsi = "",
  [string]$OutputDir = ""
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

function Resolve-SignTool([string]$explicitPath) {
  if ($explicitPath -and (Test-Path $explicitPath)) {
    return (Resolve-Path $explicitPath).Path
  }

  $cmd = Get-Command signtool.exe -ErrorAction SilentlyContinue
  if ($cmd) { return $cmd.Source }

  $kitsRoot = "C:\Program Files (x86)\Windows Kits\10\bin"
  if (Test-Path $kitsRoot) {
    $candidate = Get-ChildItem $kitsRoot -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
      Sort-Object FullName -Descending |
      Select-Object -First 1
    if ($candidate) { return $candidate.FullName }
  }

  throw "signtool.exe not found. Install Windows SDK or pass -SignToolPath."
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

function Exec([string]$file, [string[]]$args, [string]$cwd) {
  $psi = New-Object System.Diagnostics.ProcessStartInfo
  $psi.FileName = $file
  $psi.WorkingDirectory = $cwd
  $psi.RedirectStandardOutput = $true
  $psi.RedirectStandardError = $true
  $psi.UseShellExecute = $false

  $psi.Arguments = ($args | ForEach-Object {
    if ($_ -match '[\s"]') { '"' + ($_ -replace '"', '\\"') + '"' } else { $_ }
  }) -join ' '

  $proc = New-Object System.Diagnostics.Process
  $proc.StartInfo = $psi
  [void]$proc.Start()
  $stdout = $proc.StandardOutput.ReadToEnd()
  $stderr = $proc.StandardError.ReadToEnd()
  $proc.WaitForExit()

  if ($stdout) { Write-Host $stdout.TrimEnd() }
  if ($stderr) { Write-Host $stderr.TrimEnd() -ForegroundColor Yellow }

  if ($proc.ExitCode -ne 0) {
    throw "Command failed: $file $($psi.Arguments)"
  }
}

function Sign-File([string]$signtool, [string]$filePath, [string]$tsUrl) {
  Exec $signtool @("sign", "/a", "/fd", "SHA256", "/td", "SHA256", "/tr", $tsUrl, $filePath) (Split-Path $filePath -Parent)
}

function Verify-File([string]$signtool, [string]$filePath) {
  Exec $signtool @("verify", "/pa", "/v", $filePath) (Split-Path $filePath -Parent)
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
    throw "No Windows installers found in $bundleRoot"
  }

  return @{ Exe = $exe; Msi = $msi }
}

function Infer-Locale([string]$msiFileName) {
  $match = [regex]::Match($msiFileName, '_(?<loc>[A-Za-z]{2}-[A-Za-z]{2})\.msi$', 'IgnoreCase')
  if ($match.Success) {
    return $match.Groups['loc'].Value
  }

  return 'default'
}

$scriptPath = $MyInvocation.MyCommand.Path
if (-not $scriptPath) {
  throw "Cannot determine script path."
}

$repoRoot = Resolve-RepoRoot (Split-Path -Parent $scriptPath)
$meta = Read-TauriMeta $repoRoot

if (-not $OutputDir) {
  $OutputDir = Join-Path $repoRoot "signed"
}
Ensure-Dir $OutputDir

$signtool = Resolve-SignTool $SignToolPath

$items = @()
$explicitMode = ($InputExe -or $OutputExe -or $InputMsi -or $OutputMsi)

if ($explicitMode) {
  if ($InputExe -and -not $OutputExe) { throw "InputExe provided without OutputExe" }
  if ($OutputExe -and -not $InputExe) { throw "OutputExe provided without InputExe" }
  if ($InputMsi -and -not $OutputMsi) { throw "InputMsi provided without OutputMsi" }
  if ($OutputMsi -and -not $InputMsi) { throw "OutputMsi provided without InputMsi" }

  if ($InputExe) {
    $items += [pscustomobject]@{ Kind = "exe"; Input = (Resolve-Path $InputExe).Path; Output = $OutputExe }
  }
  if ($InputMsi) {
    $items += [pscustomobject]@{ Kind = "msi"; Input = (Resolve-Path $InputMsi).Path; Output = $OutputMsi }
  }
}
else {
  $det = Detect-LatestInstallers $repoRoot

  if ($det.Exe) {
    $outExe = Join-Path $OutputDir ("{0}-v{1}-windows-x64-setup-signed.exe" -f $meta.Slug, $meta.Version)
    $items += [pscustomobject]@{ Kind = "exe"; Input = $det.Exe.FullName; Output = $outExe }
  }

  if ($det.Msi) {
    $locale = Infer-Locale $det.Msi.Name
    $outMsi = Join-Path $OutputDir ("{0}-v{1}-windows-x64_{2}-signed.msi" -f $meta.Slug, $meta.Version, $locale)
    $items += [pscustomobject]@{ Kind = "msi"; Input = $det.Msi.FullName; Output = $outMsi }
  }
}

if ($items.Count -eq 0) {
  throw "No installers selected for signing."
}

foreach ($item in $items) {
  Ensure-Dir (Split-Path $item.Output -Parent)
  Copy-Item -Force $item.Input $item.Output
  Write-Host "Signing $($item.Output)"
  Sign-File $signtool $item.Output $TimestampUrl
  Verify-File $signtool $item.Output
}

$hashLines = @()
foreach ($item in $items) {
  $hash = (Get-FileHash -Algorithm SHA256 -Path $item.Output).Hash.ToLowerInvariant()
  $line = "$hash  $($item.Output.Replace($repoRoot + '\\', ''))"
  $hashLines += $line
  Write-Host $line
}

$hashOut = Join-Path $OutputDir "artifacts-sha256.txt"
$hashLines | Set-Content -Encoding UTF8 -Path $hashOut

Write-Host "Done. Hash manifest: $hashOut"