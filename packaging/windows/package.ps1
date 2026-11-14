param(
    [Parameter(Mandatory)][string]$Executable,
    [Parameter(Mandatory)][string]$ReleaseDirectory
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$exe = (Resolve-Path -LiteralPath $Executable).Path
$metadata = cargo metadata --locked --no-deps --format-version 1 | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'Could not read the Gatto version.' }
$version = ($metadata.packages | Where-Object name -eq 'gatto').version
if ($version -cnotmatch '^[0-9]+\.[0-9]+\.[0-9]+$') {
    throw 'Expected a Gatto version in major.minor.patch format.'
}
$tagPrefix = "windows-v$version-preview."
$refs = git ls-remote --tags --refs origin "refs/tags/$tagPrefix*"
if ($LASTEXITCODE -ne 0) { throw 'Could not read existing Windows preview tags from origin.' }
$pattern = '\trefs/tags/' + [regex]::Escape($tagPrefix) + '([1-9][0-9]*)$'
$latest = 0L
foreach ($ref in $refs) {
    if ($ref -cmatch $pattern) {
        $latest = [Math]::Max($latest, [long]$Matches[1])
    }
}
if ($latest -eq [long]::MaxValue) { throw 'Windows preview numbering has reached its limit.' }
$preview = $latest + 1
$tag = "$tagPrefix$preview"
$baseName = "Gatto-windows-x64-v$version-preview.$preview"
$archive = "$baseName.zip"
$setup = "$baseName-setup.exe"
Write-Host "Packaging $tag"

$iscc = (Get-Command ISCC.exe -ErrorAction SilentlyContinue).Source
if (-not $iscc) { $iscc = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe" }
if (-not (Test-Path -LiteralPath $iscc)) { throw 'Install Inno Setup 6 to package Gatto.' }
$script = Join-Path $PSScriptRoot 'Gatto.iss'
$release = (New-Item -ItemType Directory -Path $ReleaseDirectory).FullName
& $iscc "/DAppVersion=$version" "/DGattoExe=$exe" "/DReleaseDir=$release" "/DSetupName=$baseName-setup" $script | Out-Host
if ($LASTEXITCODE -ne 0) { throw 'Could not build the Windows installer.' }

Compress-Archive -LiteralPath $exe, (Join-Path $repo 'LICENSE') -DestinationPath (Join-Path $release $archive)
$checksums = foreach ($asset in @($archive, $setup)) {
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $release $asset)).Hash.ToLowerInvariant()
    "$hash  $asset"
}
# Explicit LF: the publish job verifies these exact bytes on Linux.
[System.IO.File]::WriteAllText(
    (Join-Path $release 'SHA256SUMS'),
    ($checksums -join "`n") + "`n",
    [System.Text.Encoding]::ASCII
)
if ($env:GITHUB_OUTPUT) { "tag=$tag" >> $env:GITHUB_OUTPUT }
