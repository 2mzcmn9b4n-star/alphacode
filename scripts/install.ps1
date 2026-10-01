#!/usr/bin/env pwsh
# install.ps1 — PowerShell installer for Alphacode (Windows + cross-platform pwsh)
#
# Usage:
#   iwr -useb https://raw.githubusercontent.com/dragonked2/alphacode/main/scripts/install.ps1 | iex
#   iwr -useb ... | iex -Version v1.0.0
#   iwr -useb ... | iex -Prefix "$env:LOCALAPPDATA\Programs\alphacode"
#   iwr -useb ... | iex -FromSource                  # skip release, build locally
#   iwr -useb ... | iex -SourceRef main               # build from a specific ref
#   iwr -useb ... | iex -SkipPathUpdate               # skip automatic PATH update
#
# By default, tries to download a prebuilt release asset. If no release is
# published (or there is no asset for this OS/arch), it falls back to
# building from source. Requires: git, cargo, rustc >= 1.91.

[CmdletBinding()]
param(
  [string]$Version   = $env:ALPHACODE_VERSION,
  [string]$Repo      = ($env:ALPHACODE_REPO -as [string]),
  [string]$Prefix    = $env:ALPHACODE_PREFIX,
  [string]$BinDir    = $env:ALPHACODE_BIN_DIR,
  [switch]$NoPath,
  [switch]$SkipPathUpdate,
  [switch]$FromSource,
  [switch]$SourceOnly,
  [string]$SourceRef = $env:ALPHACODE_SOURCE_REF
)

$ErrorActionPreference = 'Stop'

if (-not $Repo)    { $Repo    = 'dragonked2/alphacode' }
if (-not $Version) { $Version = 'latest' }
if (-not $Prefix)  {
  if ($IsWindows) { $Prefix = "$env:LOCALAPPDATA\alphacode" }
  else            { $Prefix = "$HOME/.local" }
}
if (-not $BinDir)  { $BinDir = Join-Path $Prefix 'bin' }

function Print([string]$msg) { Write-Host "==> $msg" -ForegroundColor Cyan }
function Warn ([string]$msg) { Write-Host "[warn] $msg" -ForegroundColor Yellow }
function Fail ([string]$msg) { Write-Host "[fail] $msg" -ForegroundColor Red; exit 1 }

function Update-UserPath {
  param(
    [string]$BinDir,
    [switch]$Force
  )

  try {
    $regPath = 'HKCU:\Environment'
    $pathValue = (Get-ItemProperty -Path $regPath -Name 'Path' -ErrorAction SilentlyContinue).Path

    if (-not $pathValue) {
      $pathValue = ''
    }

    $pathEntries = @($pathValue -split [IO.Path]::PathSeparator | Where-Object { $_ -and (-not [string]::IsNullOrWhiteSpace($_)) })

    $binDirNormalized = [System.IO.Path]::GetFullPath($BinDir)
    $alreadyInPath = $false
    foreach ($entry in $pathEntries) {
      $entryNormalized = [System.IO.Path]::GetFullPath($entry)
      if ($entryNormalized -ieq $binDirNormalized) {
        $alreadyInPath = $true
        break
      }
    }

    if ($alreadyInPath -and -not $Force) {
      Print "PATH already contains $BinDir"
      return $true
    }

    if (-not $alreadyInPath) {
      $pathEntries += $BinDir
      $newPathValue = $pathEntries -join [IO.Path]::PathSeparator
      Set-ItemProperty -Path $regPath -Name 'Path' -Value $newPathValue -Type String
      Print "Added $BinDir to user PATH"
    }

    $env:PATH = [System.Environment]::GetEnvironmentVariable('Path', 'User') + [IO.Path]::PathSeparator + [System.Environment]::GetEnvironmentVariable('Path', 'Machine')

    return $true
  } catch {
    Warn "Could not update PATH automatically: $($_.Exception.Message)"
    Warn "Please manually add $BinDir to your PATH environment variable."
    return $false
  }
}

function Build-FromSource {
  if (-not (Get-Command git   -ErrorAction SilentlyContinue)) { Fail "git is required to build from source" }
  if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { Fail "cargo is required to build from source (install Rust from https://rustup.rs)" }

  $rv = (& rustc --version) 2>$null
  if ($rv -match 'rustc\s+(\d+)\.(\d+)') {
    $major = [int]$Matches[1]; $minor = [int]$Matches[2]
    if ($major -lt 1 -or ($major -eq 1 -and $minor -lt 91)) {
      Fail "rustc $($Matches[0]) is too old; need >= 1.91 (update via 'rustup update')"
    }
  }

  $srcDir = Join-Path ([System.IO.Path]::GetTempPath()) ("alphacode-src-" + [System.Guid]::NewGuid().ToString('N'))
  New-Item -ItemType Directory -Force -Path $srcDir | Out-Null

  try {
    Print "Cloning $Repo into a temporary build directory ..."
    $cloneUrl = "https://github.com/$Repo.git"
    if ($SourceRef) {
      & git clone --depth 1 --branch $SourceRef $cloneUrl "$srcDir\src" | Out-Null
      if ($LASTEXITCODE -ne 0) { Fail "git clone failed (ref: $SourceRef)" }
    } else {
      & git clone --depth 1 $cloneUrl "$srcDir\src" | Out-Null
      if ($LASTEXITCODE -ne 0) { Fail "git clone failed" }
    }

    Print "Compiling alphacode (this can take 5-30 minutes on a first build) ..."
    & cargo build --release --manifest-path "$srcDir\src\Cargo.toml"
    if ($LASTEXITCODE -ne 0) { Fail "cargo build failed" }

    $builtExe = Join-Path "$srcDir\src\target\release" 'alphacode.exe'
    if (-not (Test-Path $builtExe)) {
      Fail "build succeeded but target\release\alphacode.exe was not produced"
    }

    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
    $destExe = Join-Path $BinDir 'alphacode.exe'
    Copy-Item -Path $builtExe -Destination $destExe -Force
    Print "Installed -> $destExe (built from source)"
  } finally {
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $srcDir
  }
}

switch ($env:PROCESSOR_ARCHITECTURE) {
  'AMD64' { $Arch = 'x86_64' }
  'ARM64' { $Arch = 'arm64' }
  default { Fail "unsupported architecture: $env:PROCESSOR_ARCHITECTURE" }
}

if ($IsWindows -or ($env:OS -eq 'Windows_NT')) {
  $Platform = 'windows'
  $asset   = "alphacode-windows-$Arch.zip"
} else {
  Fail "this script is for Windows. On Linux/macOS use scripts/install.sh."
}

if ($FromSource) {
  Print '[FromSource] requested, skipping release download.'
  Build-FromSource
  return
}

if ($Version -eq 'latest') {
  Print "Resolving latest release from $Repo ..."
  $rel = $null
  try {
    $rel = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
  } catch {
    $rel = $null
  }
  if (-not $rel -or -not $rel.tag_name) {
    if ($SourceOnly) { Fail "no release found for $Repo and -SourceOnly is set" }
    Warn "no GitHub release found for $Repo — falling back to building from source."
    Build-FromSource
    Print "Done."
    return
  }
  $Version = $rel.tag_name
  Print "Latest release: $Version"
}
$VersionNoV = $Version.TrimStart('v')

$Tmp      = [System.IO.Path]::GetTempPath() + [System.Guid]::NewGuid().ToString('N')
$ZipPath  = Join-Path $Tmp $asset
$Extract  = Join-Path $Tmp 'extract'
$Url      = "https://github.com/$Repo/releases/download/$Version/$asset"
New-Item -ItemType Directory -Force -Path $Tmp,$Extract | Out-Null

Print "Downloading $Url"
try {
  Invoke-WebRequest -Uri $Url -OutFile $ZipPath -UseBasicParsing
} catch {
  if ($SourceOnly) { Fail "download failed: $($_.Exception.Message)" }
  Warn "no prebuilt asset for $Platform/$Arch at $Version — falling back to building from source."
  Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $Tmp
  Build-FromSource
  Print "Done."
  return
}

try {
  $sums = Invoke-WebRequest -Uri "https://github.com/$Repo/releases/download/$Version/SHA256SUMS" -UseBasicParsing -ErrorAction Stop
  $expected = ($sums.Content -split "`n" | Where-Object { $_ -like "*$asset*" } | Select-Object -First 1)
  if ($expected) {
    $expectedHash = ($expected -split ' ')[0]
    $actualHash   = (Get-FileHash -Path $ZipPath -Algorithm SHA256).Hash.ToLower()
    if ($expectedHash -ne $actualHash) {
      Fail "checksum mismatch (expected $expectedHash, got $actualHash)"
    }
    Print "Checksum verified."
  }
} catch {
  Warn "could not fetch/verify SHA256SUMS — continuing"
}

Print "Extracting ..."
try {
  Expand-Archive -Path $ZipPath -DestinationPath $Extract -Force
} catch {
  Fail "extract failed: $($_.Exception.Message)"
}

$binary = Get-ChildItem -Path $Extract -Recurse -Filter 'alphacode.exe' -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $binary) {
  Fail "extracted archive did not contain 'alphacode.exe'"
}

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
$installedExe = Join-Path $BinDir 'alphacode.exe'
Copy-Item -Path $binary.FullName -Destination $installedExe -Force

$payloadFiles = Get-ChildItem -Path $Extract -Recurse -Filter '*.bin' -ErrorAction SilentlyContinue
foreach ($pf in $payloadFiles) {
    $destBin = Join-Path $BinDir $pf.Name
    Copy-Item -Path $pf.FullName -Destination $destBin -Force
}

Print "Installed -> $installedExe"

Start-Sleep -Milliseconds 500
try {
  $proc = Start-Process -FilePath "$BinDir\alphacode.exe" -ArgumentList '--version' `
    -NoNewWindow -Wait -PassThru -RedirectStandardOutput "$Tmp\version_stdout.txt" `
    -RedirectStandardError "$Tmp\version_stderr.txt"
  $exitCode = $proc.ExitCode
  $stdout = if (Test-Path "$Tmp\version_stdout.txt") { Get-Content "$Tmp\version_stdout.txt" -Raw } else { '' }
  $stderr = if (Test-Path "$Tmp\version_stderr.txt") { Get-Content "$Tmp\version_stderr.txt" -Raw } else { '' }
  if ($exitCode -eq 0 -and $stdout) {
    $versionLine = ($stdout -split "`n" | Where-Object { $_ -match 'alphacode\s+v[\d.]+' } | Select-Object -First 1)
    if ($versionLine) {
      $version = ($versionLine -replace '.*alphacode\s+(v[\d.]+).*','$1')
      Print "Installed version: $version"
    } else {
      Print "Installed (could not parse version from: $stdout)"
    }
  } else {
    $detail = if ($stderr) { $stderr.Trim() } else { "exit code $exitCode" }
    Warn "Binary installed but could not verify version: $detail"
  }
} catch {
  $excMsg = "$($_.Exception.Message)"
  if ([string]::IsNullOrWhiteSpace($excMsg)) {
    Write-Host '[warn] Binary installed but could not verify version: Start-Process failed (no exception detail; check file permissions and antivirus)' -ForegroundColor Yellow
  } else {
    Warn "Binary installed but could not verify version: $excMsg"
  }
}

if (-not $NoPath -and -not $SkipPathUpdate) {
  Write-Host ""
  Update-UserPath -BinDir $BinDir
  Write-Host ""
  Write-Host "alphacode is ready to use! Run the following commands:" -ForegroundColor Green
  Write-Host "  alphacode login"
  Write-Host "  alphacode"
  Write-Host ""
} elseif (-not $NoPath) {
  $haveIt = ($env:PATH -split [IO.Path]::PathSeparator) | Where-Object { $_ -ieq $BinDir } | Select-Object -First 1
  if (-not $haveIt) {
    Write-Host ""
    Write-Host "Next step: add the install location to your user PATH." -ForegroundColor Yellow
    Write-Host ""
    Write-Host "  See https://github.com/dragonked2/alphacode#install for PATH instructions."
    Write-Host ""
    Write-Host "Then open a new shell and:"
    Write-Host "  alphacode login"
    Write-Host "  alphacode"
  }
}

Remove-Item -Recurse -Force $Tmp
Print "Done."
