$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$Pins = Get-Content "$PSScriptRoot/windows-deps.json" -Raw | ConvertFrom-Json
$Repo = Split-Path $PSScriptRoot
$Deps = Join-Path $Repo 'target/windows-deps'

function Invoke-Native {
    param([string]$Program, [string[]]$Arguments)
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Program failed ($LASTEXITCODE)" }
}

function Get-PinnedFile {
    param([string]$Url, [string]$Sha256, [string]$Destination)
    if ($Url -notmatch '^https://' -or $Sha256 -notmatch '^[0-9a-fA-F]{64}$') {
        throw 'An HTTPS URL and SHA256 are required'
    }
    Invoke-WebRequest $Url -OutFile $Destination
    if ((Get-FileHash $Destination -Algorithm SHA256).Hash -ne $Sha256) {
        throw "Checksum mismatch: $Destination"
    }
}

function Get-WindowsSources {
    param([string]$Url, [string]$Sha256, [string]$Destination)
    New-Item -ItemType Directory -Force $Destination | Out-Null
    $archive = Join-Path $Destination 'corresponding-source.zip'
    Get-PinnedFile $Url $Sha256 $archive
    $root = Join-Path $Destination 'corresponding-source'
    if (Test-Path $root) { throw "Source destination already exists: $root" }
    $zip = [IO.Compression.ZipFile]::OpenRead($archive)
    try {
        $names = @{}
        foreach ($entry in $zip.Entries) {
            $name = $entry.FullName
            if ($name -notmatch '^[a-zA-Z0-9_./+-]+$' -or $name -match '(^|/)\.\.?(/|$)' -or
                $name -match '//' -or $name.StartsWith('/') -or
                (($entry.ExternalAttributes -shr 16) -band 0xF000) -eq 0xA000 -or
                $names.ContainsKey($name)) { throw "Unsafe source entry: $name" }
            $names[$name] = $true
        }
    } finally { $zip.Dispose() }
    [IO.Compression.ZipFile]::ExtractToDirectory($archive, $root)
    $inventory = Get-Content "$root/inventory.json" -Raw | ConvertFrom-Json
    if ($inventory.completeCorrespondingSource -ne $true -or
        $inventory.binarySha256 -ne $Pins.mpv.sha256 -or
        -not $inventory.reviewedBy -or -not $inventory.components.Count) {
        throw 'Missing matching, maintainer-reviewed complete Corresponding Source inventory'
    }
    foreach ($component in $inventory.components) {
        if (-not $component.name -or -not $component.version) { throw 'Incomplete component identity' }
        foreach ($kind in @('sources', 'recipes', 'licenses')) {
            if (-not $component.$kind.Count) { throw "Missing $kind for $($component.name)" }
            foreach ($file in $component.$kind) {
                if ($file.path -notmatch "^$kind/[a-zA-Z0-9_./+-]+$" -or
                    $file.path -match '(^|/)\.\.?(/|$)' -or
                    $file.sha256 -notmatch '^[0-9a-fA-F]{64}$' -or
                    -not (Test-Path "$root/$($file.path)" -PathType Leaf) -or
                    (Get-FileHash "$root/$($file.path)" -Algorithm SHA256).Hash -ne $file.sha256) {
                    throw "Invalid inventory file for $($component.name)"
                }
            }
        }
    }
    if (-not ($inventory.components | Where-Object name -eq 'mpv') -or
        -not ($inventory.components | Where-Object name -eq 'FFmpeg')) {
        throw 'Inventory must cover mpv, FFmpeg and every incorporated dependency'
    }
    return $root
}
