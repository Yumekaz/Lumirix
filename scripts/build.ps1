# Build Lumirix CLI on Windows with MSVC environment.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

$vcvars = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if (-not (Test-Path $vcvars)) {
    Write-Error "MSVC vcvars64.bat not found. Install Visual Studio Build Tools 2022 (C++ workload)."
}

cmd /c "`"$vcvars`" >nul && cargo build -p lumirix-cli $args"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$bin = Join-Path $Root "target\debug\lumirix.exe"
Write-Host "Built: $bin"
& $bin --version
