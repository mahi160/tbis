param([string]$Repository = 'mahi160/tbis')
. "$PSScriptRoot/windows-common.ps1"

New-Item -ItemType Directory -Force $Deps | Out-Null
$mpvArchive = Join-Path $Deps $Pins.mpv.archive
$url = "https://github.com/$Repository/releases/download/$($Pins.mirrorTag)/$($Pins.mpv.archive)"
Get-PinnedFile $url $Pins.mpv.sha256 $mpvArchive
$mpv = Join-Path $Deps 'mpv'
Invoke-Native 7z @('x', '-y', "-o$mpv", $mpvArchive)
$dlls = @(Get-ChildItem $mpv -Recurse -Filter libmpv-2.dll)
if ($dlls.Count -ne 1) { throw 'Expected exactly one libmpv-2.dll' }
$exports = & dumpbin /nologo /exports $dlls[0].FullName
if ($LASTEXITCODE -ne 0) { throw 'dumpbin exports failed' }
$names = $exports | Select-String '^\s+\d+\s+[0-9A-F]+\s+[0-9A-F]{8}\s+(\S+)' |
    ForEach-Object { $_.Matches[0].Groups[1].Value }
if (-not $names) { throw 'No libmpv exports found' }
@('LIBRARY libmpv-2', 'EXPORTS') + $names | Set-Content "$mpv/mpv.def"
Invoke-Native lib @('/nologo', "/def:$mpv/mpv.def", "/out:$mpv/mpv.lib", '/machine:x64')

foreach ($name in @('headers', 'loader')) {
    $pin = $Pins.$name
    $archive = Join-Path $Deps "$name.tar.gz"
    Get-PinnedFile $pin.url $pin.sha256 $archive
    Invoke-Native tar @('-xf', $archive, '-C', $Deps)
}
$prefix = Join-Path $Deps 'install'
Invoke-Native cmake @('-S', "$Deps/$($Pins.headers.directory)", '-B', "$Deps/headers-build", '-A', 'x64', "-DCMAKE_INSTALL_PREFIX=$prefix", '-DVULKAN_HEADERS_ENABLE_TESTS=OFF', '-DVULKAN_HEADERS_ENABLE_MODULE=OFF')
Invoke-Native cmake @('--build', "$Deps/headers-build", '--config', 'Release')
Invoke-Native cmake @('--install', "$Deps/headers-build", '--config', 'Release')
Invoke-Native cmake @('-S', "$Deps/$($Pins.loader.directory)", '-B', "$Deps/loader-build", '-A', 'x64', "-DCMAKE_INSTALL_PREFIX=$prefix", "-DCMAKE_PREFIX_PATH=$prefix", '-DBUILD_TESTS=OFF', '-DLOADER_CODEGEN=OFF')
Invoke-Native cmake @('--build', "$Deps/loader-build", '--config', 'Release')
Invoke-Native cmake @('--install', "$Deps/loader-build", '--config', 'Release')
if (-not (Test-Path "$prefix/bin/vulkan-1.dll")) { throw 'Vulkan loader installation missing' }

Add-Type -AssemblyName System.Drawing
$original = [Drawing.Image]::FromFile("$Repo/assets/icon.png")
$bitmap = [Drawing.Bitmap]::new($original, 256, 256)
$stream = [IO.MemoryStream]::new()
try {
    $bitmap.Save($stream, [Drawing.Imaging.ImageFormat]::Png)
    $png = $stream.ToArray()
    $icon = [IO.File]::Create("$Deps/icon.ico")
    $writer = [IO.BinaryWriter]::new($icon)
    try {
        $writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]1)
        $writer.Write([byte]0); $writer.Write([byte]0); $writer.Write([byte]0); $writer.Write([byte]0)
        $writer.Write([uint16]1); $writer.Write([uint16]32)
        $writer.Write([uint32]$png.Length); $writer.Write([uint32]22); $writer.Write($png)
    } finally { $writer.Dispose() }
} finally { $stream.Dispose(); $bitmap.Dispose(); $original.Dispose() }

$env:MPV_LIB_DIR = $mpv
$env:MPV_DLL = $dlls[0].FullName
$env:VULKAN_DLL = "$prefix/bin/vulkan-1.dll"
$env:TBIS_WINDOWS_ICON = "$Deps/icon.ico"
$paths = @($dlls[0].DirectoryName, "$prefix/bin")
$env:PATH = ($paths -join ';') + ';' + $env:PATH
if ($env:GITHUB_ENV) {
    foreach ($name in @('MPV_LIB_DIR', 'MPV_DLL', 'VULKAN_DLL', 'TBIS_WINDOWS_ICON')) {
        "$name=$([Environment]::GetEnvironmentVariable($name))" >> $env:GITHUB_ENV
    }
    $paths | Add-Content $env:GITHUB_PATH
}
