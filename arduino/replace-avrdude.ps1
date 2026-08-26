#Requires -Version 5.1
<#
.SYNOPSIS
  Replace Arduino IDE's bundled avrdude with a modern WinUSB-capable build.

.DESCRIPTION
  Arduino 1.8.19 ships avrdude 6.3, which cannot open Microsoft WinUSB used by
  USBasp NG classic. This script backs up the IDE copy and installs avrdude 8.x
  MSVC binaries from an official release zip.

.PARAMETER ZipPath
  Path to avrdude-v*-windows-x64.zip from https://github.com/avrdudes/avrdude/releases

.PARAMETER ArduinoAvrRoot
  Arduino hardware/tools/avr directory (contains bin\ and etc\).

.PARAMETER MinMajor
  Minimum accepted avrdude major version after install (default 7).
#>
param(
    [Parameter(Mandatory = $true)]
    [string] $ZipPath,

    [string] $ArduinoAvrRoot = "${env:ProgramFiles(x86)}\Arduino\hardware\tools\avr",

    [int] $MinMajor = 7
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $ZipPath)) {
    throw "Zip not found: $ZipPath"
}
if (-not (Test-Path -LiteralPath (Join-Path $ArduinoAvrRoot "bin"))) {
    throw "Arduino avr tools not found: $ArduinoAvrRoot"
}

$tmp = Join-Path $env:TEMP ("usbasp-ng-avrdude-" + [guid]::NewGuid().ToString("n"))
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
    Expand-Archive -LiteralPath $ZipPath -DestinationPath $tmp -Force

    # Prefer known relative layouts from avrdudes Windows zips over "first exe found".
    $candidates = @(
        (Join-Path $tmp "avrdude.exe"),
        (Join-Path $tmp "bin\avrdude.exe")
    )
    $exe = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
    if (-not $exe) {
        $found = @(Get-ChildItem -Path $tmp -Filter avrdude.exe -Recurse -ErrorAction SilentlyContinue)
        if ($found.Count -eq 0) { throw "avrdude.exe not found inside zip" }
        if ($found.Count -gt 1) {
            throw ("Multiple avrdude.exe in zip; refuse ambiguous pick: " +
                (($found | ForEach-Object { $_.FullName }) -join "; "))
        }
        $exe = $found[0].FullName
    }

    $confCandidates = @(
        (Join-Path $tmp "avrdude.conf"),
        (Join-Path $tmp "etc\avrdude.conf"),
        (Join-Path (Split-Path $exe) "avrdude.conf")
    )
    $conf = $confCandidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
    if (-not $conf) {
        $confFiles = @(Get-ChildItem -Path $tmp -Filter avrdude.conf -Recurse -ErrorAction SilentlyContinue)
        if ($confFiles.Count -eq 1) { $conf = $confFiles[0].FullName }
    }

    $destExe = Join-Path $ArduinoAvrRoot "bin\avrdude.exe"
    $destConf = Join-Path $ArduinoAvrRoot "etc\avrdude.conf"
    $bakExe = "$destExe.bak-usbasp-ng"
    $bakConf = "$destConf.bak-usbasp-ng"

    if (-not (Test-Path -LiteralPath $bakExe)) {
        Copy-Item -LiteralPath $destExe -Destination $bakExe
        Write-Host "Backup: $bakExe"
    }
    if ($conf -and (Test-Path -LiteralPath $destConf) -and -not (Test-Path -LiteralPath $bakConf)) {
        Copy-Item -LiteralPath $destConf -Destination $bakConf
        Write-Host "Backup: $bakConf"
    }

    Copy-Item -LiteralPath $exe -Destination $destExe -Force
    if ($conf) {
        New-Item -ItemType Directory -Force -Path (Split-Path $destConf) | Out-Null
        Copy-Item -LiteralPath $conf -Destination $destConf -Force
    }

    Write-Host "Installed:" $exe "->" $destExe
    $verOut = & $destExe -v 2>&1 | Out-String
    Write-Host ($verOut -split "`n" | Select-Object -First 8)
    if ($verOut -notmatch 'avrdude\s+version\s+(\d+)\.') {
        throw "Could not parse avrdude version from: $destExe -v"
    }
    $major = [int]$Matches[1]
    if ($major -lt $MinMajor) {
        throw "avrdude major $major < required $MinMajor (WinUSB needs modern avrdude)"
    }
    Write-Host "Version check OK (major $major >= $MinMajor)."
    Write-Host "Restart Arduino IDE. Verbose upload should no longer show 6.3-20190619."
}
finally {
    Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
