. "$PSScriptRoot/windows-common.ps1"

if (-not $IsWindows) { throw 'Windows dependency setup requires native Windows x64' }
if ($env:VSCMD_ARG_TGT_ARCH -ne 'x64') { throw 'Initialize the x64 MSVC developer environment first' }
if (Test-Path $Deps) { Remove-Item -Recurse -Force $Deps }
$mpv = Join-Path $Deps 'mpv'
New-Item -ItemType Directory -Force $mpv | Out-Null

# libmpv: shinchiro's prebuilt package, mirrored in this repo's releases because upstream keeps ~30 days
$pin = $Pins.libmpv
$package = Join-Path $Deps $pin.file
Get-Pinned @($pin.mirror, $pin.upstream) $pin.sha256 $package
Invoke-Native 7z @('x', '-y', "-o$mpv", $package, 'libmpv-2.dll', 'include') | Out-Null
$exports = Invoke-Native dumpbin @('/nologo', '/exports', "$mpv/libmpv-2.dll")
$names = $exports | Select-String '^\s+\d+\s+[0-9A-F]+\s+[0-9A-F]{8}\s+(\S+)' |
    ForEach-Object { $_.Matches[0].Groups[1].Value }
if (-not $names -or 'mpv_create' -notin $names -or 'mpv_render_context_create' -notin $names) {
    throw 'Missing libmpv client/render exports'
}
@('LIBRARY libmpv-2', 'EXPORTS') + $names | Set-Content "$mpv/mpv.def"
Invoke-Native lib @('/nologo', "/def:$mpv/mpv.def", "/out:$mpv/mpv.lib", '/machine:x64')

# libmpv hard-imports vulkan-1.dll, which Windows only has when a GPU driver installed it
$vulkan = $Pins.vulkanLoader
$work = Join-Path $Deps 'vulkan'
New-Item -ItemType Directory -Force $work | Out-Null
foreach ($name in @('Vulkan-Headers', 'Vulkan-Loader')) {
    $archive = "$work/$name.tar.gz"
    $sha = if ($name -eq 'Vulkan-Headers') { $vulkan.headersSha256 } else { $vulkan.loaderSha256 }
    Get-Pinned @("https://github.com/KhronosGroup/$name/archive/refs/tags/$($vulkan.tag).tar.gz") $sha $archive
    Invoke-Native tar @('-xzf', $archive, '-C', $work)
}
$headers = "$work/Vulkan-Headers-$($vulkan.tag)"
$loader = "$work/Vulkan-Loader-$($vulkan.tag)"
Invoke-Native cmake @('-S', $headers, '-B', "$headers/build", "-DCMAKE_INSTALL_PREFIX=$work/prefix")
Invoke-Native cmake @('--install', "$headers/build")
# UPDATE_DEPS=OFF: the loader must not clone unpinned dependencies at build time
Invoke-Native cmake @('-S', $loader, '-B', "$loader/build", "-DVULKAN_HEADERS_INSTALL_DIR=$work/prefix", '-DBUILD_TESTS=OFF', '-DUPDATE_DEPS=OFF')
Invoke-Native cmake @('--build', "$loader/build", '--config', 'Release')
$built = @(Get-ChildItem "$loader/build" -Recurse -Filter 'vulkan-1.dll')
if ($built.Count -ne 1) { throw "Expected one vulkan-1.dll, found $($built.Count)" }
Copy-Item $built[0].FullName "$mpv/vulkan-1.dll"
if (-not ((Invoke-Native dumpbin @('/nologo', '/headers', "$mpv/vulkan-1.dll")) -match '8664 machine \(x64\)')) {
    throw 'vulkan-1.dll is not x64'
}
New-Item -ItemType Directory -Force "$Deps/licenses/vulkan-loader" | Out-Null
Copy-Item "$loader/LICENSE.txt" "$Deps/licenses/vulkan-loader/"

Assert-WindowsImports @(Get-Item "$mpv/libmpv-2.dll", "$mpv/vulkan-1.dll")

$env:MPV_LIB_DIR = $mpv
$env:MPV_DLL = "$mpv/libmpv-2.dll"
$paths = @($mpv)
$env:PATH = ($paths -join ';') + ';' + $env:PATH
if ($env:GITHUB_ENV) {
    foreach ($name in @('MPV_LIB_DIR', 'MPV_DLL')) {
        "$name=$([Environment]::GetEnvironmentVariable($name))" >> $env:GITHUB_ENV
    }
    $paths | Add-Content $env:GITHUB_PATH
}
