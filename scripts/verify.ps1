# Runs exactly what CI runs minus the end-to-end smoke test, in order, stopping at the first
# failure (docs/TESTING.md section 2). Windows PowerShell 5.1 has no && operator, so every step
# reads $LASTEXITCODE; --% hands the rest of a line to the program untouched.
$ErrorActionPreference = 'Continue'
Set-Location (Join-Path $PSScriptRoot '..')

function Invoke-Step([string]$Name, [scriptblock]$Command) {
    Write-Host ""
    Write-Host "== $Name"
    & $Command
    if ($LASTEXITCODE -ne 0) {
        Write-Host "verify: $Name failed (exit $LASTEXITCODE)"
        exit $LASTEXITCODE
    }
}

Invoke-Step 'frontend build'     { npm --prefix app run build }
Invoke-Step 'frontend typecheck' { npm --prefix app run typecheck }
Invoke-Step 'frontend lint'      { npm --prefix app run lint }
Invoke-Step 'frontend format'    { npm --prefix app run format:check }
Invoke-Step 'frontend tests'     { npm --prefix app test }
Invoke-Step 'rust format'        { cargo fmt --all --check }
Invoke-Step 'rust clippy'        { cargo --% clippy --workspace --all-targets -- -D warnings }
Invoke-Step 'rust licences'      { cargo deny check licenses }
Invoke-Step 'npm licences'       { npm --prefix app run licenses }
Invoke-Step 'fixtures'           { cargo --% run -q -p keepers-fixtures -- --out tests/fixtures/gen --seed 1 --check tests/fixtures/gen.sha256 }
Invoke-Step 'rust tests'         { cargo test --workspace }

Write-Host ""
Write-Host "verify: green"
