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
#>
param(
    [Parameter(Mandatory = $true)]
    [string] $ZipPath,

    [string] $ArduinoAvrRoot = "${env:ProgramFiles(x86)}\Arduino\hardware\tools\avr"
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
    $exe = Get-ChildItem -Path $tmp -Filter avrdude.exe -Recurse | Select-Object -First 1
    if (-not $exe) { throw "avrdude.exe not found inside zip" }
    $conf = Get-ChildItem -Path $tmp -Filter avrdude.conf -Recurse | Select-Object -First 1

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

    Copy-Item -LiteralPath $exe.FullName -Destination $destExe -Force
    if ($conf) {
        New-Item -ItemType Directory -Force -Path (Split-Path $destConf) | Out-Null
        Copy-Item -LiteralPath $conf.FullName -Destination $destConf -Force
    }

    Write-Host "Installed:" $exe.FullName "->" $destExe
    & $destExe -v 2>&1 | Select-Object -First 5
    Write-Host ""
    Write-Host "Restart Arduino IDE. Verbose upload should no longer show 6.3-20190619."
}
finally {
    Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
