<#
.SYNOPSIS
    Asserts the version and the AppUserModelID agree everywhere they are
    declared, and optionally that a release tag agrees with the version.

.DESCRIPTION
    Both values are written by hand in files that cannot include each other:
    Cargo.toml and installer.toml are separate TOML files, Info.plist is XML
    and main.rs is Rust. Nothing makes them agree, so this does, and CI runs it
    on every push.

    Cargo.toml is the source the Windows version resource is generated from,
    so it stands in for the exe's Details tab.

    Both failures are silent. A drifted version means the Add/Remove Programs
    entry, the exe's Details tab and the macOS Finder info disagree, and nobody
    notices until they are working out which build a machine is running. A
    drifted AUMID means the running window will not group under its own pinned
    taskbar icon.

.PARAMETER Tag
    A release tag such as v2.0.0. Compared against the version with any leading
    v removed. Omit it outside a release.
#>

[CmdletBinding()]
param(
    [string]$Tag
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepoRoot = Split-Path -Parent $PSScriptRoot

function Read-Source {
    param([string]$RelativePath)

    $path = Join-Path $RepoRoot $RelativePath
    if (-not (Test-Path -LiteralPath $path)) { throw "not found: $RelativePath" }
    return [IO.File]::ReadAllText($path)
}

# Matches against the whole file, so a pattern may span lines. (?m) is on, so ^
# and $ still mean the start and end of a line.
function Get-Declared {
    param([string]$RelativePath, [string]$Pattern, [string]$Text)

    if (-not $PSBoundParameters.ContainsKey('Text')) { $Text = Read-Source $RelativePath }

    $match = [regex]::Match($Text, "(?m)$Pattern")
    if (-not $match.Success) { throw "$RelativePath does not match /$Pattern/" }
    return $match.Groups[1].Value
}

# Cargo.toml has a version key in every [dependencies.x] table that spells one
# out, so only the [package] table counts.
function Get-CargoPackageTable {
    $text = Read-Source 'Cargo.toml'
    $match = [regex]::Match($text, '(?ms)^\[package\]\s*$(.*?)(?=^\[|\z)')
    if (-not $match.Success) { throw 'Cargo.toml has no [package] table' }
    return $match.Groups[1].Value
}

# 2.0.0, 2.0.0.0 and "2,0,0,0" are the same version written three ways.
function ConvertTo-Quad {
    param([string]$Version)

    $parts = @($Version -split '[,.]' | ForEach-Object { $_.Trim() } |
               Where-Object { $_ -ne '' })
    while ($parts.Count -lt 4) { $parts += '0' }
    return ($parts[0..3] -join '.')
}

$plist = 'packaging\macos\Info.plist'

$declared = [ordered]@{
    'Cargo.toml      [package] version' =
        Get-Declared 'Cargo.toml' '^\s*version\s*=\s*"([^"]+)"' (Get-CargoPackageTable)
    'installer.toml  product.version' =
        Get-Declared 'installer.toml' '^\s*version\s*=\s*"([^"]+)"'
    'Info.plist      CFBundleShortVersionString' =
        Get-Declared $plist '<key>CFBundleShortVersionString</key>\s*<string>([^<]+)</string>'
    'Info.plist      CFBundleVersion' =
        Get-Declared $plist '<key>CFBundleVersion</key>\s*<string>([^<]+)</string>'
}

if ($Tag) {
    $declared['git tag'] = $Tag -replace '^v', ''
}

foreach ($entry in $declared.GetEnumerator()) {
    '{0,-42} {1,-10} -> {2}' -f $entry.Key, $entry.Value, (ConvertTo-Quad $entry.Value)
}

$unique = @($declared.Values | ForEach-Object { ConvertTo-Quad $_ } | Sort-Object -Unique)
if ($unique.Count -ne 1) {
    throw "version declarations disagree: $($unique -join ', ')"
}

Write-Host "`nall version declarations agree: $($unique[0])" -ForegroundColor Green

# The AppUserModelID the installer stamps on the shortcut, and the one the
# process claims for itself at startup, have to be the same string or the
# window will not group under its own pinned icon.
$main = Read-Source 'src\main.rs'
if ($main -notmatch 'SetCurrentProcessExplicitAppUserModelID') {
    throw 'src\main.rs never calls SetCurrentProcessExplicitAppUserModelID'
}

$aumid = [ordered]@{
    'installer.toml  product.aumid' =
        Get-Declared 'installer.toml' '^\s*aumid\s*=\s*"([^"]+)"'
    # The string literal rather than the call site: the windows crate takes it
    # through w!() or a const, and either form leaves the literal in main.rs.
    'main.rs         AppUserModelID literal' =
        Get-Declared 'src\main.rs' '"(LockeWerks\.[A-Za-z0-9.]+)"' $main
}

''
foreach ($entry in $aumid.GetEnumerator()) {
    '{0,-42} {1}' -f $entry.Key, $entry.Value
}

$uniqueAumid = @($aumid.Values | Sort-Object -Unique)
if ($uniqueAumid.Count -ne 1) {
    throw "AppUserModelID declarations disagree: $($uniqueAumid -join ', ')"
}

Write-Host "`nAppUserModelID agrees: $($uniqueAumid[0])" -ForegroundColor Green
