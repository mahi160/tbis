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
    $system = @(
        'avicap32.dll', 'shcore.dll', 'combase.dll', 'gdiplus.dll', 'd3dcompiler_47.dll',
        'icuuc.dll', 'uiautomationcore.dll',
        'api-ms-win-core-path-l1-1-0.dll', 'api-ms-win-core-winrt-error-l1-1-1.dll',
        'api-ms-win-shcore-stream-winrt-l1-1-0.dll', 'api-ms-win-shcore-scaling-l1-1-1.dll',
        'advapi32.dll', 'avrt.dll', 'bcrypt.dll', 'bcryptprimitives.dll', 'cabinet.dll',
        'cfgmgr32.dll', 'comctl32.dll', 'comdlg32.dll', 'credui.dll', 'crypt32.dll',
        'cryptbase.dll', 'd2d1.dll', 'd3d11.dll', 'd3d12.dll', 'd3d9.dll', 'dcomp.dll',
        'dbghelp.dll', 'dnsapi.dll', 'dsound.dll', 'dwmapi.dll', 'dwrite.dll', 'dxgi.dll',
        'dxva2.dll', 'gdi32.dll', 'hid.dll', 'imm32.dll', 'iphlpapi.dll', 'kernel32.dll',
        'kernelbase.dll', 'mf.dll', 'mfplat.dll', 'mfreadwrite.dll', 'mfuuid.dll',
        'mmdevapi.dll', 'mpr.dll', 'msacm32.dll', 'msimg32.dll', 'msvcrt.dll', 'ncrypt.dll',
        'netapi32.dll', 'normaliz.dll', 'ntdll.dll', 'ole32.dll', 'oleacc.dll', 'oleaut32.dll',
        'opengl32.dll', 'powrprof.dll', 'propsys.dll', 'psapi.dll', 'rpcrt4.dll', 'secur32.dll',
        'setupapi.dll', 'shell32.dll', 'shlwapi.dll', 'user32.dll', 'userenv.dll', 'usp10.dll',
        'ucrtbase.dll', 'uxtheme.dll', 'version.dll', 'winhttp.dll', 'wininet.dll',
        'winmm.dll', 'winspool.drv', 'wintrust.dll', 'wldap32.dll', 'ws2_32.dll',
        'wtsapi32.dll', 'runtimeobject.dll',
        'api-ms-win-core-synch-l1-2-0.dll', 'api-ms-win-core-winrt-l1-1-0.dll',
        'api-ms-win-core-winrt-error-l1-1-0.dll', 'api-ms-win-core-winrt-string-l1-1-0.dll',
        'api-ms-win-crt-conio-l1-1-0.dll', 'api-ms-win-crt-convert-l1-1-0.dll',
        'api-ms-win-crt-environment-l1-1-0.dll', 'api-ms-win-crt-filesystem-l1-1-0.dll',
        'api-ms-win-crt-heap-l1-1-0.dll', 'api-ms-win-crt-locale-l1-1-0.dll',
        'api-ms-win-crt-math-l1-1-0.dll', 'api-ms-win-crt-multibyte-l1-1-0.dll',
        'api-ms-win-crt-private-l1-1-0.dll', 'api-ms-win-crt-process-l1-1-0.dll',
        'api-ms-win-crt-runtime-l1-1-0.dll', 'api-ms-win-crt-stdio-l1-1-0.dll',
        'api-ms-win-crt-string-l1-1-0.dll', 'api-ms-win-crt-time-l1-1-0.dll',
        'api-ms-win-crt-utility-l1-1-0.dll'
    )
    foreach ($file in $files) {
        $output = & dumpbin /nologo /dependents $file.FullName
        if ($LASTEXITCODE -ne 0) { throw "dumpbin failed: $($file.Name)" }
        $imports = @($output | Select-String '^\s+([A-Za-z0-9_.-]+\.(?:dll|drv))\s*$' |
            ForEach-Object { $_.Matches[0].Groups[1].Value.ToLowerInvariant() })
        if (-not $imports.Count) { throw "No imports parsed: $($file.Name)" }
        foreach ($import in $imports) {
            if ($import -match '^(vcruntime|msvcp|concrt)' -or
                ($import -notin $files.Name -and $import -notin $system)) {
                throw "Unbundled/non-system import $import in $($file.Name)"
            }
        }
    }
}

function Get-WindowsSources {
    Invoke-Native python @("$PSScriptRoot/windows_sources.py", 'verify', '--repo', $Repo, '--deps', $Deps)
}
