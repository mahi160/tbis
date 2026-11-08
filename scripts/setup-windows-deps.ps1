. "$PSScriptRoot/windows-common.ps1"

& "$PSScriptRoot/build-windows-mpv.ps1"
$mpv = Join-Path $Deps 'mpv'

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
