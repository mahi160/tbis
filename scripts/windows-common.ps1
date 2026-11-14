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

function Assert-WindowsImports {
    param([IO.FileInfo[]]$Files)
    $system = @($Pins.systemDlls)
    $prefixes = @($Pins.systemDllPrefixes)
    foreach ($file in $files) {
        $output = & dumpbin /nologo /dependents $file.FullName
        if ($LASTEXITCODE -ne 0) { throw "dumpbin failed: $($file.Name)" }
        $imports = @($output | Select-String '^\s+([A-Za-z0-9_.-]+\.(?:dll|drv))\s*$' |
            ForEach-Object { $_.Matches[0].Groups[1].Value.ToLowerInvariant() })
        if (-not $imports.Count) { throw "No imports parsed: $($file.Name)" }
        foreach ($import in $imports) {
            if ($import -match '^(vcruntime|msvcp|concrt)' -or
                ($import -notin $files.Name -and $import -notin $system -and
                    -not @($prefixes | Where-Object { $import.StartsWith($_) }).Count)) {
                throw "Unbundled/non-system import $import in $($file.Name)"
            }
        }
    }
}

# Downloads the first URL that answers to $Path and fails unless its SHA-256 matches.
function Get-Pinned {
    param([string[]]$Urls, [string]$Sha256, [string]$Path)
    foreach ($url in $Urls) {
        try {
            Invoke-WebRequest -Uri $url -OutFile $Path -UseBasicParsing
            break
        } catch {
            Write-Warning "Download failed: $url"
        }
    }
    if (-not (Test-Path $Path)) { throw "No pinned URL answered: $($Urls -join ', ')" }
    $actual = (Get-FileHash -Algorithm SHA256 $Path).Hash.ToLowerInvariant()
    if ($actual -ne $Sha256) {
        Remove-Item $Path
        throw "$Path hashed $actual, expected $Sha256"
    }
}
