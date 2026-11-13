$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') { throw 'Run installer fixtures only on an ephemeral GitHub runner' }
Add-Type -AssemblyName System.IO.Compression
$helper = (Resolve-Path '.local/package-windows/routeloom-vpn.exe').Path
$root = Join-Path (Get-Location) ('.local/update-fixtures-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
function New-Fixture([string]$name, [bool]$unsafePath) {
    $folder = Join-Path $root $name
    New-Item -ItemType Directory -Path $folder | Out-Null
    foreach ($file in @('routeloom.exe','routeloom-vpn.exe','flutter_windows.dll','wintun.dll')) {
        [IO.File]::WriteAllText((Join-Path $folder $file),'old')
    }
    $state = Join-Path $folder 'routeloom-state'
    New-Item -ItemType Directory -Path $state | Out-Null
    [IO.File]::WriteAllText((Join-Path $state 'keep'),'private-state')
    $archive = Join-Path $root ($name + '.zip')
    $zip = [IO.Compression.ZipArchive]::new([IO.File]::Open($archive,[IO.FileMode]::CreateNew),[IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($file in @('routeloom.exe','routeloom-vpn.exe','flutter_windows.dll','wintun.dll')) {
            $writer = [IO.StreamWriter]::new($zip.CreateEntry($file).Open())
            $writer.Write('new'); $writer.Dispose()
        }
        if ($unsafePath) {
            $writer = [IO.StreamWriter]::new($zip.CreateEntry('../escape').Open())
            $writer.Write('unsafe'); $writer.Dispose()
        }
    } finally { $zip.Dispose() }
    return @{ folder=$folder; archive=$archive; digest=(Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() }
}
function Invoke-Updater($fixture,[bool]$badHash) {
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName=$helper; $start.Arguments='install-update'
    $start.UseShellExecute=$false; $start.CreateNoWindow=$true
    $start.RedirectStandardInput=$true; $start.RedirectStandardOutput=$true; $start.RedirectStandardError=$true
    $process=[Diagnostics.Process]::Start($start)
    $digest=$fixture.digest
    if($badHash){ $digest='0'*64 }
    $process.StandardInput.Write((@{archive=$fixture.archive; target=$fixture.folder; sha256=$digest; pid=0} | ConvertTo-Json -Compress))
    $process.StandardInput.Close()
    if(-not $process.WaitForExit(20000)){ $process.Kill(); throw 'Updater fixture timed out' }
    $output=$process.StandardOutput.ReadToEnd(); $errorText=$process.StandardError.ReadToEnd()
    $code=$process.ExitCode; $process.Dispose()
    return $code
}
function Assert-State($fixture,[string]$expected) {
    foreach($file in @('routeloom.exe','routeloom-vpn.exe','flutter_windows.dll','wintun.dll')){
        if([IO.File]::ReadAllText((Join-Path $fixture.folder $file)) -ne $expected){throw ('Unexpected installed content: '+$file)}
    }
    if([IO.File]::ReadAllText((Join-Path $fixture.folder 'routeloom-state/keep')) -ne 'private-state'){throw 'Updater changed private state'}
}
$good=New-Fixture 'good' $false
if((Invoke-Updater $good $false) -ne 0){throw 'Valid update failed'}
Assert-State $good 'new'
$bad=New-Fixture 'bad-hash' $false
if((Invoke-Updater $bad $true) -eq 0){throw 'Invalid hash accepted'}
Assert-State $bad 'old'
$escape=New-Fixture 'escape' $true
if((Invoke-Updater $escape $false) -eq 0){throw 'Unsafe archive accepted'}
Assert-State $escape 'old'
if(Test-Path -LiteralPath (Join-Path $root 'escape')) {
    if(-not (Get-Item -LiteralPath (Join-Path $root 'escape')).PSIsContainer){throw 'Archive escaped its destination'}
}
$locked=New-Fixture 'rollback' $false
$lock=[IO.File]::Open((Join-Path $locked.folder 'flutter_windows.dll'),[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
try { if((Invoke-Updater $locked $false) -eq 0){throw 'Locked destination unexpectedly updated'} }
finally { $lock.Dispose() }
Assert-State $locked 'old'
Write-Output 'Verified package replacement, digest rejection, archive confinement, rollback and private-state preservation.'
