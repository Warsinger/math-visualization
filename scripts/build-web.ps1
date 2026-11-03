[CmdletBinding()]
param([string]$Bindgen = 'wasm-bindgen')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    if (-not (Test-Path 'node_modules/@warsinger/web-shared/browser/module-runtime.mjs')) { throw 'Run npm ci --ignore-scripts first to install shared browser assets.' }
    $version = & $Bindgen --version
    if ($LASTEXITCODE -ne 0 -or $version -ne 'wasm-bindgen 0.2.129') {
        throw 'Install matching CLI: cargo install wasm-bindgen-cli --version 0.2.129 --locked'
    }
    $messages = & cargo build --locked --release --target wasm32-unknown-unknown --bin math-playground --message-format=json
    $buildExit = $LASTEXITCODE
    foreach ($line in $messages) {
        $message = $line | ConvertFrom-Json
        if ($message.reason -eq 'compiler-message' -and $message.message.rendered) { Write-Host $message.message.rendered }
    }
    if ($buildExit -ne 0) { throw 'Browser compilation failed.' }
    $output = Join-Path $projectRoot 'target/web-bindgen'
    New-Item -ItemType Directory -Path $output -Force | Out-Null
    $build = $messages | ForEach-Object { $_ | ConvertFrom-Json } | Where-Object { $_.reason -eq 'build-script-executed' -and $_.package_id -match 'math-playground' } | Select-Object -Last 1
    if (-not $build) { throw 'Missing application build metadata output.' }
    Copy-Item -LiteralPath (Join-Path $build.out_dir 'build-info.json') -Destination (Join-Path $output 'build-info.json')
    & $Bindgen 'target/wasm32-unknown-unknown/release/math-playground.wasm' --target web --no-typescript --out-dir $output --out-name math_playground
    if ($LASTEXITCODE -ne 0) { throw 'JavaScript/Wasm binding generation failed.' }
    & node scripts/prepare-web.mjs
    if ($LASTEXITCODE -ne 0) { throw 'Browser asset preparation failed.' }
    Write-Host 'Built dist/web. Serve with: node scripts/serve-web.mjs'
} finally { Pop-Location }
