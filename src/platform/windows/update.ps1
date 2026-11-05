$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-Tree([string] $Path) {
    $attributes = [IO.File]::GetAttributes($Path)
    if (($attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Reparse point in bundle: $Path"
    }
    if (($attributes -band [IO.FileAttributes]::Directory) -ne 0) {
        foreach ($child in [IO.Directory]::GetFileSystemEntries($Path)) {
            Assert-Tree $child
        }
    }
}

function Move-Path([string] $Source, [string] $Destination) {
    if ([IO.Directory]::Exists($Source)) {
        [IO.Directory]::Move($Source, $Destination)
    } else {
        [IO.File]::Move($Source, $Destination)
    }
}

function New-State {
    return @{
        moved = [Collections.Generic.List[string]]::new()
        installed = [Collections.Generic.List[string]]::new()
    }
}

function Save-State($State, [string] $Stage) {
    $json = ConvertTo-Json -InputObject @{ moved = @($State.moved.ToArray()); installed = @($State.installed.ToArray()) } -Compress
    [IO.File]::WriteAllText([IO.Path]::Combine($Stage, 'state.json'), $json)
}

function Move-Bundle($Plan, [string] $Stage, $State) {
    $attributes = [IO.File]::GetAttributes($Plan.bundle)
    if (($attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'Installation became a reparse point' }
    foreach ($name in $Plan.old) {
        Assert-Tree ([IO.Path]::Combine($Plan.bundle, $name))
    }
    foreach ($name in $Plan.new) {
        Assert-Tree ([IO.Path]::Combine($Stage, 'new', $name))
    }
    foreach ($name in $Plan.old) {
        Move-Path ([IO.Path]::Combine($Plan.bundle, $name)) ([IO.Path]::Combine($Stage, 'old', $name))
        $State.moved.Add($name)
        Save-State $State $Stage
    }
    foreach ($name in $Plan.new) {
        Move-Path ([IO.Path]::Combine($Stage, 'new', $name)) ([IO.Path]::Combine($Plan.bundle, $name))
        $State.installed.Add($name)
        Save-State $State $Stage
    }
}

function Restore-Bundle($Plan, [string] $Stage, $State) {
    $errors = [Collections.Generic.List[string]]::new()
    $failed = [IO.Path]::Combine($Stage, 'failed')
    foreach ($name in $State.installed) {
        try {
            [void] [IO.Directory]::CreateDirectory($failed)
            Move-Path ([IO.Path]::Combine($Plan.bundle, $name)) ([IO.Path]::Combine($failed, $name))
        } catch {
            $errors.Add("Cannot move failed new file $name aside: $_")
        }
    }
    foreach ($name in $State.moved) {
        try {
            Move-Path ([IO.Path]::Combine($Stage, 'old', $name)) ([IO.Path]::Combine($Plan.bundle, $name))
        } catch {
            $errors.Add("Cannot restore $name`: $_")
        }
    }
    if ($errors.Count -ne 0) {
        throw ($errors -join [Environment]::NewLine)
    }
}

function Write-Log([string] $Message) {
    [Console]::Error.WriteLine("$(Get-Date -Format o) $Message")
}

function Start-Bundle([string] $Bundle) {
    $info = [Diagnostics.ProcessStartInfo]::new()
    $info.FileName = [IO.Path]::Combine($Bundle, 'tbis.exe')
    $info.WorkingDirectory = $Bundle
    $info.UseShellExecute = $false
    return [Diagnostics.Process]::Start($info)
}

function Wait-Startup($Process) {
    $deadline = [DateTime]::UtcNow.AddSeconds(45)
    $visibleSince = $null
    while ([DateTime]::UtcNow -lt $deadline) {
        $Process.Refresh()
        if ($Process.HasExited) {
            throw "Updated tbis exited during startup (code $($Process.ExitCode))"
        }
        if ($Process.MainWindowHandle -ne [IntPtr]::Zero -and $Process.Responding) {
            if ($null -eq $visibleSince) { $visibleSince = [DateTime]::UtcNow }
            if (([DateTime]::UtcNow - $visibleSince).TotalSeconds -ge 10) { return }
        } else {
            $visibleSince = $null
        }
        Start-Sleep -Milliseconds 200
    }
    throw 'Updated tbis did not show a responsive window for 10 seconds within 45 seconds'
}

function Invoke-Apply([string] $Stage) {
    $state = New-State
    $process = $null
    $plan = $null
    $swapping = $false
    try {
        $plan = ConvertFrom-Json -InputObject ([IO.File]::ReadAllText([IO.Path]::Combine($Stage, 'plan.json')))
        $old = [Diagnostics.Process]::GetProcessById($plan.pid)
        [void] $old.Handle
        Write-Log "Helper ready for PID $($plan.pid); staging $Stage"
        [IO.File]::WriteAllText([IO.Path]::Combine($Stage, 'ready'), 'ready')
        if (-not $old.WaitForExit(60000)) { throw 'Old tbis did not exit within 60 seconds; installation unchanged' }
        if ([IO.File]::Exists([IO.Path]::Combine($Stage, 'cancel'))) { throw 'Update cancelled; installation unchanged' }
        $swapping = $true
        Move-Bundle $plan $Stage $state
        $process = Start-Bundle $plan.bundle
        Wait-Startup $process
        Write-Log 'New tbis passed startup observation; update complete'
    } catch {
        $failure = $_
        Write-Log "Update failed: $failure; recovery files: $Stage"
        if (-not $swapping) { return 1 }
        try {
            if ($null -ne $process -and -not $process.HasExited) {
                try { $process.Kill() } catch {
                    $process.Refresh()
                    if (-not $process.HasExited) { throw }
                }
                if (-not $process.WaitForExit(10000)) { throw 'New tbis did not exit; refusing to replace locked files' }
            }
            Restore-Bundle $plan $Stage $state
            Write-Log 'Restored the complete previous bundle'
            $restored = Start-Bundle $plan.bundle
            Write-Log "Restarted previous tbis as PID $($restored.Id)"
        } catch {
            Write-Log "RECOVERY FAILED: $_; backups and journal retained at $Stage"
        }
        return 1
    }
    try {
        [IO.Directory]::Delete($Stage, $true)
    } catch {
        Write-Log "Update succeeded but cleanup failed: $_; staging retained at $Stage"
    }
    return 0
}

$stage = $env:TBIS_UPDATE_STAGE
switch ($env:TBIS_UPDATE_MODE) {
    'Library' { return }
    'Inspect' {
        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $archive = [IO.Compression.ZipFile]::OpenRead([IO.Path]::Combine($stage, 'tbis.zip'))
        try {
            $entries = @($archive.Entries | ForEach-Object {
                @{ name = $_.FullName; attributes = $_.ExternalAttributes; length = $_.Length }
            })
            [Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
            [Console]::Write((ConvertTo-Json -InputObject $entries -Compress))
        } finally {
            $archive.Dispose()
        }
    }
    'Extract' {
        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $entries = @(ConvertFrom-Json -InputObject ([IO.File]::ReadAllText([IO.Path]::Combine($stage, 'entries.json'))))
        $archive = [IO.Compression.ZipFile]::OpenRead([IO.Path]::Combine($stage, 'tbis.zip'))
        try {
            if ($entries.Count -ne $archive.Entries.Count) { throw 'Archive changed after validation' }
            $root = [IO.Path]::Combine($stage, 'new')
            [void] [IO.Directory]::CreateDirectory($root)
            for ($i = 0; $i -lt $entries.Count; $i++) {
                $entry = $archive.Entries[$i]
                $validated = $entries[$i]
                if ($entry.FullName -cne $validated.name -or $entry.Length -ne $validated.length -or $entry.ExternalAttributes -ne $validated.attributes) {
                    throw 'Archive changed after validation'
                }
                $destination = [IO.Path]::Combine($root, $entry.FullName)
                if ($entry.FullName.EndsWith('/')) {
                    [void] [IO.Directory]::CreateDirectory($destination)
                } else {
                    [void] [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($destination))
                    $inputStream = $entry.Open()
                    try {
                        $outputStream = [IO.File]::Open($destination, [IO.FileMode]::CreateNew)
                        try { $inputStream.CopyTo($outputStream) } finally { $outputStream.Dispose() }
                    } finally {
                        $inputStream.Dispose()
                    }
                }
            }
        } finally {
            $archive.Dispose()
        }
    }
    'Apply' { exit (Invoke-Apply $stage) }
    default { throw 'Unknown updater mode' }
}
