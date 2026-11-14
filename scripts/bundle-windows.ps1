. "$PSScriptRoot/windows-common.ps1"

Invoke-Native git @('-C', $Repo, 'diff', '--quiet', 'HEAD', '--')
$untracked = Invoke-Native git @('-C', $Repo, 'ls-files', '--others', '--exclude-standard', '--', 'src', 'assets', 'scripts', '.cargo', 'Cargo.toml', 'Cargo.lock', 'build.rs')
if ($untracked) { throw 'Commit source inputs before packaging' }
if ([IO.Path]::GetFullPath($env:MPV_DLL) -ne [IO.Path]::GetFullPath("$Deps/mpv/libmpv-2.dll")) {
    throw 'Bundle requires the libmpv from scripts/setup-windows-deps.ps1'
}
Invoke-Native cargo @('build', '--profile', 'dist', '--locked')
$dist = Join-Path $Repo 'target/windows-dist'
if (Test-Path $dist) { Remove-Item -Recurse -Force $dist }
New-Item -ItemType Directory -Force "$dist/licenses", "$dist/components" | Out-Null
Copy-Item "$Repo/target/dist/tbis.exe", $env:MPV_DLL, "$Deps/mpv/vulkan-1.dll", "$Repo/LICENSE" $dist
Copy-Item "$Deps/licenses/*" "$dist/licenses" -Recurse
Copy-Item "$PSScriptRoot/windows-deps.json", "$Repo/Cargo.lock" "$dist/components"
$libmpv = $Pins.libmpv
$vulkan = $Pins.vulkanLoader
$revision = Invoke-Native git @('-C', $Repo, 'rev-parse', 'HEAD')
@(
    "mpv $($libmpv.mpv) (shinchiro/mpv-winbuild-cmake $($libmpv.release))",
    "Vulkan-Loader $($vulkan.tag)",
    'Rust dependencies: exact versions in components/Cargo.lock'
) | Set-Content "$dist/bundled-components.txt"
# GPL source offer (ADR-0004): where the source of every bundled binary is published
@(
    "tbis $revision",
    "  https://github.com/mahi160/tbis/tree/$revision",
    '',
    "libmpv-2.dll: shinchiro/mpv-winbuild-cmake release $($libmpv.release), $($libmpv.file)",
    "  sha256 $($libmpv.sha256)",
    "  package: $($libmpv.upstream)",
    "  mirror: $($libmpv.mirror)",
    "  mpv source: https://github.com/mpv-player/mpv/tree/$($libmpv.mpvCommit)",
    "  build recipe (FFmpeg and every other library, with their source URLs): $($libmpv.buildRecipe)",
    "  build log: $($libmpv.workflowRun)",
    '',
    "vulkan-1.dll: built by scripts/setup-windows-deps.ps1 from",
    "  https://github.com/KhronosGroup/Vulkan-Loader/archive/refs/tags/$($vulkan.tag).tar.gz (sha256 $($vulkan.loaderSha256))",
    "  https://github.com/KhronosGroup/Vulkan-Headers/archive/refs/tags/$($vulkan.tag).tar.gz (sha256 $($vulkan.headersSha256))"
) | Set-Content "$dist/components/sources.txt"

Assert-WindowsImports @(Get-ChildItem $dist -File | Where-Object { $_.Extension -in @('.exe', '.dll') })
$env:PATH = "$dist;$env:PATH"
$info = [Diagnostics.ProcessStartInfo]::new()
$info.FileName = "$dist/tbis.exe"
$info.WorkingDirectory = $dist
$info.UseShellExecute = $false
$info.Arguments = '--self-test'
# an installed tbis.exe logs to its app data folder (dirs::data_dir), not the install folder
$log = Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'tbis/tbis.log'
$started = Get-Date
$process = [Diagnostics.Process]::Start($info)
try {
    if (-not $process.WaitForExit(60000)) {
        try { $process.Kill() } catch {
            $process.Refresh()
            if (-not $process.HasExited) { throw }
        }
        if (-not $process.WaitForExit(10000)) { throw 'tbis self-test did not exit after timeout termination' }
        throw 'tbis self-test did not exit within 60 seconds'
    }
    if ($process.ExitCode -ne 0) { throw "tbis self-test failed (code $($process.ExitCode))" }
} finally { $process.Dispose() }
if (-not (Test-Path $log) -or (Get-Item $log).LastWriteTime -lt $started) { throw "tbis self-test did not write $log" }
if (Test-Path "$dist/tbis.log") { throw 'tbis self-test wrote tbis.log into the install folder' }

$version = (Select-String '^version = "([^"]+)"' "$Repo/Cargo.toml" | Select-Object -First 1).Matches[0].Groups[1].Value
$zip = "$Repo/target/tbis-$version-windows-x86_64.zip"
# vendored only to collect the Rust crates' license notices
$vendor = "$Deps/vendor"
Invoke-Native cargo @('vendor', '--locked', '--versioned-dirs', '--manifest-path', "$Repo/Cargo.toml", $vendor) | Out-Null
foreach ($notice in (Get-ChildItem $vendor -Recurse -File | Where-Object {
    $_.Name -match '^(LICENSE|COPYING|NOTICE|COPYRIGHT)' -or $_.DirectoryName -match '[/\\]LICENSES?([/\\]|$)'
})) {
    $destination = Join-Path "$dist/licenses/rust" ([IO.Path]::GetRelativePath($vendor, $notice.FullName))
    New-Item -ItemType Directory -Force (Split-Path $destination) | Out-Null
    Copy-Item $notice.FullName $destination
}
Compress-Archive -Path "$dist/*" -DestinationPath $zip -Force
