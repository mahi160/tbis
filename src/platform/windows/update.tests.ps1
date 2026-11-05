$ErrorActionPreference = 'Stop'
$previousMode = $env:TBIS_UPDATE_MODE
try {
    $env:TBIS_UPDATE_MODE = 'Library'
    . ([IO.Path]::Combine($PSScriptRoot, 'update.ps1'))
} finally {
    $env:TBIS_UPDATE_MODE = $previousMode
}

function Assert([bool] $Condition, [string] $Message) {
    if (-not $Condition) { throw $Message }
}

function Write-File([string] $Path, [string] $Value) {
    [void] [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($Path))
    [IO.File]::WriteAllText($Path, $Value)
}

function New-Case([string] $Root, [string] $Name) {
    $bundle = [IO.Path]::Combine($Root, $Name)
    $staging = [IO.Path]::Combine($bundle, ".tbis-update space ' quote")
    [void] [IO.Directory]::CreateDirectory([IO.Path]::Combine($staging, 'old'))
    $oldFiles = @('tbis.exe', 'libmpv-2.dll', 'obsolete.dll', 'LICENSE', 'bundled-components.txt', 'licenses/mpv/COPYING', 'components/old.dat')
    $newFiles = @('tbis.exe', 'libmpv-2.dll', 'fresh.dll', 'LICENSE', 'bundled-components.txt', 'licenses/mpv/COPYING', 'components/new.dat')
    foreach ($name in $oldFiles) { Write-File ([IO.Path]::Combine($bundle, $name)) "old:$name" }
    foreach ($name in $newFiles) { Write-File ([IO.Path]::Combine($staging, 'new', $name)) "new:$name" }
    Write-File ([IO.Path]::Combine($bundle, 'config.json')) 'personal config'
    Write-File ([IO.Path]::Combine($bundle, 'tbis.log')) 'personal log'
    return @{
        Stage = $staging
        Plan = @{
            bundle = $bundle
            old = @('tbis.exe', 'libmpv-2.dll', 'obsolete.dll', 'LICENSE', 'bundled-components.txt', 'licenses', 'components')
            new = @('tbis.exe', 'libmpv-2.dll', 'fresh.dll', 'LICENSE', 'bundled-components.txt', 'licenses', 'components')
        }
        OldFiles = $oldFiles
        NewFiles = $newFiles
        State = (New-State)
    }
}

function Assert-Personal($Case) {
    Assert ([IO.File]::ReadAllText([IO.Path]::Combine($Case.Plan.bundle, 'config.json')) -eq 'personal config') 'Config was modified'
    Assert ([IO.File]::ReadAllText([IO.Path]::Combine($Case.Plan.bundle, 'tbis.log')) -eq 'personal log') 'Log was modified'
}

function Assert-Restored($Case) {
    foreach ($name in $Case.OldFiles) {
        Assert ([IO.File]::ReadAllText([IO.Path]::Combine($Case.Plan.bundle, $name)) -eq "old:$name") "Not restored: $name"
    }
    Assert-Personal $Case
}

$testRoot = [IO.Path]::Combine([IO.Path]::GetTempPath(), "tbis-update-tests-$([Guid]::NewGuid())")
[void] [IO.Directory]::CreateDirectory($testRoot)
try {
    $case = New-Case $testRoot 'complete swap and startup rollback'
    Move-Bundle $case.Plan $case.Stage $case.State
    foreach ($name in $case.NewFiles) {
        Assert ([IO.File]::ReadAllText([IO.Path]::Combine($case.Plan.bundle, $name)) -eq "new:$name") "Not installed: $name"
    }
    Assert (-not [IO.File]::Exists([IO.Path]::Combine($case.Plan.bundle, 'obsolete.dll'))) 'Obsolete dependency remained installed'
    Assert ([IO.File]::ReadAllText([IO.Path]::Combine($case.Stage, 'old', 'tbis.exe')) -eq 'old:tbis.exe') 'Backup was removed before startup success'
    Assert-Personal $case
    Restore-Bundle $case.Plan $case.Stage $case.State
    Assert-Restored $case
    Assert (-not [IO.File]::Exists([IO.Path]::Combine($case.Plan.bundle, 'fresh.dll'))) 'New dependency remained after rollback'

    $case = New-Case $testRoot 'old move fails halfway'
    Write-File ([IO.Path]::Combine($case.Stage, 'old', 'libmpv-2.dll')) 'backup conflict'
    $failed = $false
    try { Move-Bundle $case.Plan $case.Stage $case.State } catch { $failed = $true }
    Assert $failed 'Expected an old-file move failure'
    Assert ($case.State.moved.Count -eq 1) 'Expected exactly one old file moved'
    Restore-Bundle $case.Plan $case.Stage $case.State
    Assert-Restored $case

    $case = New-Case $testRoot 'new move fails halfway'
    Write-File ([IO.Path]::Combine($case.Plan.bundle, 'fresh.dll')) 'destination appeared after staging'
    $failed = $false
    try { Move-Bundle $case.Plan $case.Stage $case.State } catch { $failed = $true }
    Assert $failed 'Expected a new-file move failure'
    Assert ($case.State.installed.Count -eq 2) 'Expected exe and first DLL installed before failure'
    Restore-Bundle $case.Plan $case.Stage $case.State
    Assert-Restored $case

    $case = New-Case $testRoot 'rollback error is reported and backup retained'
    Move-Bundle $case.Plan $case.Stage $case.State
    Write-File ([IO.Path]::Combine($case.Stage, 'failed', 'tbis.exe')) 'recovery destination conflict'
    $failure = ''
    try { Restore-Bundle $case.Plan $case.Stage $case.State } catch { $failure = $_.ToString() }
    Assert ($failure.Contains('Cannot move failed new file tbis.exe') -and $failure.Contains('Cannot restore tbis.exe')) 'Rollback errors were lost'
    Assert ([IO.File]::ReadAllText([IO.Path]::Combine($case.Stage, 'old', 'tbis.exe')) -eq 'old:tbis.exe') 'Unrestored old executable was lost'
    Assert ([IO.File]::ReadAllText([IO.Path]::Combine($case.Plan.bundle, 'libmpv-2.dll')) -eq 'old:libmpv-2.dll') 'Other files were not restored after an error'
    Assert-Personal $case
} finally {
    [IO.Directory]::Delete($testRoot, $true)
}
