. "$PSScriptRoot/windows-common.ps1"

& "$PSScriptRoot/build-windows-mpv.ps1"
$mpv = Join-Path $Deps 'mpv'

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
$env:MPV_DLL = "$mpv/libmpv-2.dll"
$env:TBIS_WINDOWS_ICON = "$Deps/icon.ico"
$paths = @($mpv)
$env:PATH = ($paths -join ';') + ';' + $env:PATH
if ($env:GITHUB_ENV) {
    foreach ($name in @('MPV_LIB_DIR', 'MPV_DLL', 'TBIS_WINDOWS_ICON')) {
        "$name=$([Environment]::GetEnvironmentVariable($name))" >> $env:GITHUB_ENV
    }
    $paths | Add-Content $env:GITHUB_PATH
}
