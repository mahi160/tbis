param(
    [string]$SourceUrl = $env:WINDOWS_SOURCE_URL,
    [string]$SourceSha256 = $env:WINDOWS_SOURCE_SHA256
)
. "$PSScriptRoot/windows-common.ps1"

Invoke-Native git @('-C', $Repo, 'diff', '--quiet', 'HEAD', '--')
$untracked = Invoke-Native git @('-C', $Repo, 'ls-files', '--others', '--exclude-standard', '--', 'src', 'assets', 'scripts', '.cargo', 'Cargo.toml', 'Cargo.lock', 'build.rs')
if ($untracked) { throw 'Commit source inputs before packaging' }
$source = Get-WindowsSources $SourceUrl $SourceSha256 "$Deps/release-source"
Invoke-Native cargo @('build', '--profile', 'dist', '--locked')
$dist = Join-Path $Repo 'target/windows-dist'
if (Test-Path $dist) { Remove-Item -Recurse -Force $dist }
New-Item -ItemType Directory -Force "$dist/licenses", "$dist/components" | Out-Null
Copy-Item "$Repo/target/dist/tbis.exe", $env:MPV_DLL, $env:VULKAN_DLL, "$Repo/LICENSE" $dist
Copy-Item "$source/licenses/*" "$dist/licenses" -Recurse
Copy-Item "$source/inventory.json", "$PSScriptRoot/windows-deps.json", "$Repo/Cargo.lock" "$dist/components"
$inventory = Get-Content "$source/inventory.json" -Raw | ConvertFrom-Json
$inventory.components | ForEach-Object { "$($_.name) $($_.version)" } | Set-Content "$dist/bundled-components.txt"
foreach ($name in @('headers', 'loader')) {
    $licenseDir = "$dist/licenses/Vulkan-$name"
    New-Item -ItemType Directory $licenseDir | Out-Null
    $upstream = "$Deps/$($Pins.$name.directory)"
    $notices = @(Get-ChildItem $upstream -Recurse -File | Where-Object {
        $_.Name -match '^(LICENSE|COPYING|NOTICE|COPYRIGHT)' -or $_.DirectoryName -match '[/\\]LICENSES?([/\\]|$)'
    })
    if (-not $notices.Count) { throw "No Vulkan $name license notices" }
    foreach ($notice in $notices) {
        $relative = [IO.Path]::GetRelativePath($upstream, $notice.FullName)
        $destination = Join-Path $licenseDir $relative
        New-Item -ItemType Directory -Force (Split-Path $destination) | Out-Null
        Copy-Item $notice.FullName $destination
    }
    "Vulkan-$name $($Pins.$name.sha256)" | Add-Content "$dist/bundled-components.txt"
}

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
$files = @(Get-ChildItem $dist -File | Where-Object { $_.Extension -in @('.exe', '.dll') })
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
$env:PATH = "$dist;$env:PATH"
Push-Location $dist
try { Invoke-Native "$dist/tbis.exe" @('--self-test') } finally { Pop-Location }
Remove-Item "$dist/tbis.log"

$version = (Select-String '^version = "([^"]+)"' "$Repo/Cargo.toml" | Select-Object -First 1).Matches[0].Groups[1].Value
$zip = "$Repo/target/tbis-$version-windows-x86_64.zip"
$sources = "$Deps/release-source/sources"
New-Item -ItemType Directory -Force "$sources/tbis", "$sources/vulkan" | Out-Null
Invoke-Native git @('-C', $Repo, 'archive', '--format=tar', '-o', "$Deps/tbis.tar", 'HEAD')
Invoke-Native tar @('-xf', "$Deps/tbis.tar", '-C', "$sources/tbis")
Push-Location "$sources/tbis"
try {
    Invoke-Native cargo @('vendor', '--locked', '--versioned-dirs', 'vendor') | Add-Content '.cargo/config.toml'
} finally { Pop-Location }
$vendor = "$sources/tbis/vendor"
foreach ($notice in (Get-ChildItem $vendor -Recurse -File | Where-Object {
    $_.Name -match '^(LICENSE|COPYING|NOTICE|COPYRIGHT)' -or $_.DirectoryName -match '[/\\]LICENSES?([/\\]|$)'
})) {
    $destination = Join-Path "$dist/licenses/rust" ([IO.Path]::GetRelativePath($vendor, $notice.FullName))
    New-Item -ItemType Directory -Force (Split-Path $destination) | Out-Null
    Copy-Item $notice.FullName $destination
}
'Rust dependencies: exact versions in components/Cargo.lock; sources in windows-sources.tar' | Add-Content "$dist/bundled-components.txt"
Copy-Item $source "$sources/dependencies" -Recurse
Copy-Item "$Deps/headers.tar.gz", "$Deps/loader.tar.gz" "$sources/vulkan"
Invoke-Native git @('-C', $Repo, 'rev-parse', 'HEAD') | Set-Content "$sources/tbis-revision.txt"
Invoke-Native tar @('-cf', "$Repo/target/tbis-$version-windows-sources.tar", '-C', (Split-Path $sources), 'sources')
Compress-Archive -Path "$dist/*" -DestinationPath $zip -Force
