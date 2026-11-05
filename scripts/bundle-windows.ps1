. "$PSScriptRoot/windows-common.ps1"

Invoke-Native git @('-C', $Repo, 'diff', '--quiet', 'HEAD', '--')
$untracked = Invoke-Native git @('-C', $Repo, 'ls-files', '--others', '--exclude-standard', '--', 'src', 'assets', 'scripts', '.cargo', 'Cargo.toml', 'Cargo.lock', 'build.rs')
if ($untracked) { throw 'Commit source inputs before packaging' }
if ([IO.Path]::GetFullPath($env:MPV_DLL) -ne [IO.Path]::GetFullPath("$Deps/mpv/libmpv-2.dll")) {
    throw 'Bundle requires the locally source-built libmpv'
}
$source = Get-WindowsSources
Invoke-Native cargo @('build', '--profile', 'dist', '--locked')
$dist = Join-Path $Repo 'target/windows-dist'
if (Test-Path $dist) { Remove-Item -Recurse -Force $dist }
New-Item -ItemType Directory -Force "$dist/licenses", "$dist/components" | Out-Null
Copy-Item "$Repo/target/dist/tbis.exe", $env:MPV_DLL, "$Repo/LICENSE" $dist
Copy-Item "$source/licenses/*" "$dist/licenses" -Recurse
Copy-Item "$source/inventory.json", "$PSScriptRoot/windows-deps.json", "$Repo/Cargo.lock" "$dist/components"
$inventory = Get-Content "$source/inventory.json" -Raw | ConvertFrom-Json
$inventory.components | ForEach-Object { "$($_.name) $($_.version)" } | Set-Content "$dist/bundled-components.txt"

Assert-WindowsImports @(Get-ChildItem $dist -File | Where-Object { $_.Extension -in @('.exe', '.dll') })
$env:PATH = "$dist;$env:PATH"
Push-Location $dist
try { Invoke-Native "$dist/tbis.exe" @('--self-test') } finally { Pop-Location }
Remove-Item "$dist/tbis.log"

$version = (Select-String '^version = "([^"]+)"' "$Repo/Cargo.toml" | Select-Object -First 1).Matches[0].Groups[1].Value
$zip = "$Repo/target/tbis-$version-windows-x86_64.zip"
$sources = "$Deps/release-source/sources"
if (Test-Path $sources) { Remove-Item -Recurse -Force $sources }
New-Item -ItemType Directory -Force "$sources/tbis" | Out-Null
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
Invoke-Native git @('-C', $Repo, 'rev-parse', 'HEAD') | Set-Content "$sources/tbis-revision.txt"
Invoke-Native tar @('-cf', "$Repo/target/tbis-$version-windows-sources.tar", '-C', (Split-Path $sources), 'sources')
Compress-Archive -Path "$dist/*" -DestinationPath $zip -Force
