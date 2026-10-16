[CmdletBinding()]
param([string]$Bindgen = 'wasm-bindgen')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    $version = & $Bindgen --version
    if ($LASTEXITCODE -ne 0 -or $version -ne 'wasm-bindgen 0.2.129') {
        throw 'Install matching CLI: cargo install wasm-bindgen-cli --version 0.2.129 --locked'
    }
    & cargo build --locked --release --target wasm32-unknown-unknown --bin math-playground
    if ($LASTEXITCODE -ne 0) { throw 'Browser compilation failed.' }
    $output = Join-Path $projectRoot 'dist/web'
    New-Item -ItemType Directory -Path $output -Force | Out-Null
    & $Bindgen 'target/wasm32-unknown-unknown/release/math-playground.wasm' --target web --no-typescript --out-dir $output --out-name math_playground
    if ($LASTEXITCODE -ne 0) { throw 'JavaScript/Wasm binding generation failed.' }
    Copy-Item -LiteralPath (Join-Path $projectRoot 'web/index.html') -Destination (Join-Path $output 'index.html') -Force
    Write-Host 'Built dist/web. Serve with: node scripts/serve-web.mjs'
} finally { Pop-Location }
