# PowerShell 7 is available on all three GitHub-hosted runner platforms.
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateSet('x86_64-pc-windows-msvc', 'x86_64-unknown-linux-gnu', 'aarch64-apple-darwin')]
    [string]$Target
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$packageName = "math-playground-$Target"
$executable = if ($Target -eq 'x86_64-pc-windows-msvc') { 'math-playground.exe' } else { 'math-playground' }
$binary = Join-Path $projectRoot "target/$Target/release/$executable"
if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { throw "Missing release executable: $binary" }
if ((Get-Item -LiteralPath $binary).Length -eq 0) { throw 'Release executable is empty.' }
if ($Target -eq 'x86_64-pc-windows-msvc') {
    # Enforce GUI subsystem in the actual executable being distributed.
    $bytes = [IO.File]::ReadAllBytes($binary)
    $pe = [BitConverter]::ToInt32($bytes, 0x3c)
    if ([BitConverter]::ToUInt32($bytes, $pe) -ne 0x00004550) { throw 'Invalid PE signature.' }
    $subsystem = [BitConverter]::ToUInt16($bytes, $pe + 24 + 68)
    if ($subsystem -ne 2) { throw "Windows package must use GUI subsystem (2), got $subsystem." }
    # A window icon alone does not brand Explorer; verify embedded PE icon resources.
    $optional = $pe + 24
    $directories = if ([BitConverter]::ToUInt16($bytes, $optional) -eq 0x20b) { 112 } else { 96 }
    $resourceRva = [BitConverter]::ToUInt32($bytes, $optional + $directories + 16)
    $sections = $optional + [BitConverter]::ToUInt16($bytes, $pe + 20)
    $resourceOffset = $null
    for ($i = 0; $i -lt [BitConverter]::ToUInt16($bytes, $pe + 6); $i++) {
        $section = $sections + 40 * $i
        $address = [BitConverter]::ToUInt32($bytes, $section + 12)
        $size = [BitConverter]::ToUInt32($bytes, $section + 16)
        if ($resourceRva -ge $address -and $resourceRva -lt ($address + $size)) {
            $resourceOffset = [BitConverter]::ToUInt32($bytes, $section + 20) + $resourceRva - $address
        }
    }
    if ($null -eq $resourceOffset) { throw 'Missing Windows resources.' }
    $count = [BitConverter]::ToUInt16($bytes, $resourceOffset + 12) + [BitConverter]::ToUInt16($bytes, $resourceOffset + 14)
    $types = @(for ($i = 0; $i -lt $count; $i++) { [BitConverter]::ToUInt32($bytes, $resourceOffset + 16 + 8 * $i) })
    if (3 -notin $types -or 14 -notin $types) { throw 'Missing Windows icon/image-group resources.' }
}
$dist = Join-Path $projectRoot 'dist'
# A fresh staging directory avoids including stale files on repeated local runs.
$stage = Join-Path $dist ([guid]::NewGuid().ToString())
$package = Join-Path $stage $packageName
New-Item -ItemType Directory -Path $package -Force | Out-Null
Copy-Item -LiteralPath $binary -Destination (Join-Path $package $executable)
$icons = @('app-icon.png')
Copy-Item -LiteralPath (Join-Path $projectRoot 'assets/icon/app-256.png') -Destination (Join-Path $package 'app-icon.png')
if ($Target -eq 'aarch64-apple-darwin') {
    Copy-Item -LiteralPath (Join-Path $projectRoot 'assets/icon/app.icns') -Destination (Join-Path $package 'app-icon.icns')
    $icons += 'app-icon.icns'
}
$symbols = @()
if ($Target -eq 'x86_64-pc-windows-msvc') {
    # MSVC writes symbols beside the executable, even with split-debuginfo=off.
    $symbolFile = Join-Path $projectRoot "target/$Target/release/math_playground.pdb"
    if (-not (Test-Path -LiteralPath $symbolFile -PathType Leaf)) { throw "Missing release symbols: $symbolFile" }
    Copy-Item -LiteralPath $symbolFile -Destination $package
    $symbols = @('math_playground.pdb')
}
Copy-Item -LiteralPath (Join-Path $projectRoot 'docs/DOWNLOADS.md') -Destination (Join-Path $package 'RUNNING.md')
$revision = git -C $projectRoot rev-parse HEAD
if ($LASTEXITCODE -ne 0) { throw 'Cannot determine source revision.' }
$rustVersion = rustc +stable --version
if ($LASTEXITCODE -ne 0) { throw 'Cannot determine Rust version.' }
$info = [ordered]@{ target = $Target; revision = $revision; rust = $rustVersion }
$info | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $package 'build-info.json') -Encoding utf8
$expected = @($executable, 'RUNNING.md', 'build-info.json') + $symbols + $icons | Sort-Object
$actual = @(Get-ChildItem -LiteralPath $package -File | Select-Object -ExpandProperty Name | Sort-Object)
if (Compare-Object $expected $actual) { throw 'Unexpected package contents.' }
if ((Get-ChildItem -LiteralPath $package -Directory).Count -ne 0) { throw 'Unexpected package subdirectories.' }
if ($Target -eq 'x86_64-pc-windows-msvc') {
    $archive = Join-Path $dist "$packageName.zip"
    Compress-Archive -LiteralPath $package -DestinationPath $archive -Force
    $zip = [System.IO.Compression.ZipFile]::OpenRead($archive)
    try { $entries = @($zip.Entries | Where-Object { $_.Name } | ForEach-Object { $_.FullName.Replace('\','/') } | Sort-Object) }
    finally { $zip.Dispose() }
} else {
    $archive = Join-Path $dist "$packageName.tar.gz"
    # tar preserves the Unix executable permission; upload-artifact's outer ZIP does not.
    & tar -czf $archive -C $stage $packageName
    if ($LASTEXITCODE -ne 0) { throw 'Archive creation failed.' }
    $entries = @(& tar -tzf $archive)
    if ($LASTEXITCODE -ne 0) { throw 'Archive verification failed.' }
    $entries = @($entries | Where-Object { -not $_.EndsWith('/') } | Sort-Object)
}
$expectedEntries = @($expected | ForEach-Object { "$packageName/$_" } | Sort-Object)
if (Compare-Object $expectedEntries $entries) { throw 'Archive contains missing or unexpected files.' }
$digest = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
"$digest  $([System.IO.Path]::GetFileName($archive))" | Set-Content -LiteralPath (Join-Path $dist "$packageName.sha256") -Encoding ascii
Write-Output "Verified $archive ($Target): executable, instructions, build metadata, and SHA-256."
