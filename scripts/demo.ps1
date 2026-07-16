# End-to-end MVP demo for Lumirix (Windows).
# Usage (from repo root, after build):
#   powershell -ExecutionPolicy Bypass -File scripts\demo.ps1
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

$bin = Join-Path $Root "target\debug\lumirix.exe"
if (-not (Test-Path $bin)) {
    Write-Host "Binary missing; building..."
    & (Join-Path $PSScriptRoot "build.ps1")
}

function Section($title) {
    Write-Host ""
    Write-Host "==== $title ====" -ForegroundColor Cyan
}

Section "1) Init + status"
& $bin init --force
& $bin status

Section "2) Clean-ish run (allow-dirty if tree is dirty)"
& $bin run --allow-dirty -- git --version
& $bin report last | Select-Object -First 20

Section "3) Critical risk demo (.env)"
& $bin run --allow-dirty -- cmd /C "echo SECRET=demo>> .env"
& $bin risks last
& $bin report last | Select-Object -First 25
if (Test-Path .env) { Remove-Item .env -Force }

Section "4) Auth change + weak evidence"
New-Item -ItemType Directory -Force -Path demo_auth | Out-Null
& $bin run --allow-dirty -- cmd /C "echo //probe>> demo_auth\session.ts"
# treat path containing auth via rename into auth folder if needed
New-Item -ItemType Directory -Force -Path src\auth -ErrorAction SilentlyContinue | Out-Null
Copy-Item demo_auth\session.ts src\auth\session.ts -Force -ErrorAction SilentlyContinue
& $bin run --allow-dirty -- cmd /C "echo //probe2>> src\auth\session.ts"
& $bin evidence last
& $bin report last | Select-Object -First 30

Section "5) Rollback export"
& $bin rollback last --write demo-rollback.patch
if (Test-Path demo-rollback.patch) {
    Write-Host "Wrote demo-rollback.patch ($((Get-Item demo-rollback.patch).Length) bytes)"
    Remove-Item demo-rollback.patch -Force
}

Section "6) List runs"
& $bin runs

# cleanup demo files
Remove-Item -Recurse -Force demo_auth, src -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "Demo complete. Inspect .lumirix\runs\ for artifacts." -ForegroundColor Green
