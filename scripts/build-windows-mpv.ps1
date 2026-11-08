. "$PSScriptRoot/windows-common.ps1"

if (-not $IsWindows) { throw 'The libmpv recipe requires native Windows x64 with the MSVC SDK environment' }
if ($env:VSCMD_ARG_TGT_ARCH -ne 'x64') { throw 'Initialize the x64 MSVC developer environment first' }
if (Test-Path $Deps) { Remove-Item -Recurse -Force $Deps }
New-Item -ItemType Directory -Force $Deps | Out-Null
$python = (Get-Command python -CommandType Application | Select-Object -First 1).Source
$env:PYTHONDONTWRITEBYTECODE = '1'
$env:GIT_CEILING_DIRECTORIES = [IO.Path]::GetFullPath($Deps)
Invoke-Native $python @("$PSScriptRoot/windows_sources.py", 'prepare', '--repo', $Repo, '--deps', $Deps)

$sdkRoots = @($env:VCToolsInstallDir, $env:WindowsSdkDir, $env:UniversalCRTSdkDir) | Where-Object { $_ }
foreach ($name in @('INCLUDE', 'LIB')) {
    $paths = @([Environment]::GetEnvironmentVariable($name) -split ';' | Where-Object { $_ })
    $isolated = @($paths | Where-Object {
        $path = [IO.Path]::GetFullPath($_)
        @($sdkRoots | Where-Object { $path.StartsWith([IO.Path]::GetFullPath($_).TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase) }).Count -gt 0
    })
    if (-not $isolated.Count) { throw "No MSVC/Windows SDK $name paths" }
    [Environment]::SetEnvironmentVariable($name, ($isolated -join ';'))
}
foreach ($name in @('CL', '_CL_', 'CFLAGS', 'CXXFLAGS', 'CPPFLAGS', 'LDFLAGS', 'C_INCLUDE_PATH', 'CPLUS_INCLUDE_PATH', 'LIBRARY_PATH')) {
    [Environment]::SetEnvironmentVariable($name, $null)
}
Invoke-Native $python @("$PSScriptRoot/windows_sources.py", 'toolchain', '--deps', $Deps)
$tools = (Get-Content "$Deps/toolchain.json" -Raw | ConvertFrom-Json).tools

$env:PYTHONPATH = "$Deps/tools/meson"
$env:PYTHONNOUSERSITE = '1'
$env:PATH = (Split-Path $tools.ninja.path) + ';' + (Split-Path $tools.nasm.path) + ';' + $env:PATH
$env:PKG_CONFIG_PATH = ''
$env:PKG_CONFIG_LIBDIR = "$Deps/empty-pkgconfig"
$env:CMAKE_PREFIX_PATH = ''
New-Item -ItemType Directory -Force $env:PKG_CONFIG_LIBDIR | Out-Null
$providers = @($Pins.sources | Where-Object { $_.PSObject.Properties.Name -contains 'provides' } | ForEach-Object { $_.name })
$setup = @('-m', 'mesonbuild.mesonmain', 'setup', "$Deps/build", "$Deps/source/mpv", '--native-file', "$Deps/native.ini", '--wrap-mode=nodownload', "--force-fallback-for=$($providers -join ',')")
$setup += @($Pins.buildOptions | ForEach-Object { "-D$_" })
$compile = @('-C', "$Deps/build")
@{ setup = @($python) + $setup; compile = @($tools.ninja.path) + $compile } | ConvertTo-Json -Depth 4 | Set-Content "$Deps/build-command.json"
Invoke-Native $python $setup 2>&1 | Tee-Object -FilePath "$Deps/build.log"
$dll = Invoke-Native $python @("$PSScriptRoot/windows_sources.py", 'audit', '--repo', $Repo, '--deps', $Deps)
Invoke-Native $tools.ninja.path $compile 2>&1 | Tee-Object -FilePath "$Deps/build.log" -Append
$builtDlls = @(Get-ChildItem "$Deps/build" -Recurse -Filter '*.dll')
if ($builtDlls.Count -ne 1 -or $builtDlls[0].FullName -ne [IO.Path]::GetFullPath($dll)) {
    throw 'Unexpected shared subproject DLLs or missing libmpv'
}
$mpv = Join-Path $Deps 'mpv'
New-Item -ItemType Directory -Force $mpv | Out-Null
Copy-Item $dll "$mpv/libmpv-2.dll"
Assert-WindowsImports @(Get-Item "$mpv/libmpv-2.dll")
$exports = Invoke-Native $tools.dumpbin.path @('/nologo', '/exports', "$mpv/libmpv-2.dll")
$names = $exports | Select-String '^\s+\d+\s+[0-9A-F]+\s+[0-9A-F]{8}\s+(\S+)' |
    ForEach-Object { $_.Matches[0].Groups[1].Value }
if (-not $names -or 'mpv_create' -notin $names -or 'mpv_render_context_create' -notin $names) {
    throw 'Missing libmpv client/render exports'
}
@('LIBRARY libmpv-2', 'EXPORTS') + $names | Set-Content "$mpv/mpv.def"
Invoke-Native $tools.lib.path @('/nologo', "/def:$mpv/mpv.def", "/out:$mpv/mpv.lib", '/machine:x64')
Invoke-Native $python @("$PSScriptRoot/windows_sources.py", 'inventory', '--repo', $Repo, '--deps', $Deps)
