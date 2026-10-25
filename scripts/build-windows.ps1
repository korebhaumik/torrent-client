param(
    [ValidateSet('debug', 'release')]
    [string]$Profile = 'release'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($env:OS -ne 'Windows_NT') {
    throw 'Build on Windows with the Rust MSVC toolchain and Visual Studio C++ Build Tools.'
}
if (-not (Get-Command nasm -CommandType Application -ErrorAction SilentlyContinue)) {
    throw 'NASM is required to build aws-lc-sys. Install NASM, add its directory to PATH, and restart PowerShell.'
}

$repo = Split-Path $PSScriptRoot -Parent
$target = 'x86_64-pc-windows-msvc'
Push-Location $repo
try {
    $cargo = 'cargo'
    $buildArgs = @('build', '--locked', '--target', $target)
    if ($Profile -eq 'release') { $buildArgs += '--release' }
    & $cargo @buildArgs
    if ($LASTEXITCODE -ne 0) { throw 'Windows compilation failed.' }

    $metadataJson = & $cargo metadata --locked --offline --no-deps --format-version 1
    if ($LASTEXITCODE -ne 0) { throw 'Cannot read Cargo package metadata.' }
    $metadata = $metadataJson | ConvertFrom-Json
    $package = $metadata.packages | Where-Object { $_.name -eq 'torrent-desktop' }
    if (-not $package) { throw 'Cannot find the torrent-desktop package.' }

    $binary = Join-Path $metadata.target_directory "$target/$Profile/torrent-desktop.exe"
    $dist = Join-Path $repo 'dist'
    $name = "Torrent-$($package.version)-windows-x64"
    if ($Profile -ne 'release') { $name += '-debug' }
    # Recreate only this package, so old files cannot leak into the release ZIP.
    $staging = Join-Path $dist $name
    if (Test-Path $staging) { Remove-Item -LiteralPath $staging -Recurse -Force }
    New-Item -ItemType Directory -Path $staging -Force | Out-Null
    $executable = Join-Path $staging 'Torrent.exe'
    Copy-Item -LiteralPath $binary -Destination $executable
    Copy-Item -LiteralPath (Join-Path $repo 'docs/windows-release-readme.txt') -Destination (Join-Path $staging 'README.txt')

    $version = (Get-Item -LiteralPath $executable).VersionInfo
    if ($version.ProductName -ne 'Torrent Desktop' -or $version.ProductVersion -ne $package.version) {
        throw 'Executable version resources do not match Cargo package metadata.'
    }

    $archive = Join-Path $dist "$name.zip"
    Compress-Archive -LiteralPath $staging -DestinationPath $archive -Force
    $hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  $name.zip" | Set-Content -LiteralPath "$archive.sha256" -Encoding ascii
    Write-Host "Built: $archive"
    Write-Host "Executable bytes: $((Get-Item -LiteralPath $executable).Length)"
    Write-Host "ZIP bytes: $((Get-Item -LiteralPath $archive).Length)"
    Write-Host "SHA256: $hash"
}
finally {
    Pop-Location
}
