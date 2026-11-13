$ErrorActionPreference = 'Stop'
$stage = Join-Path (Get-Location) '.local/package-windows'
$signature = Get-AuthenticodeSignature -LiteralPath (Join-Path $stage 'wintun.dll')
if ($signature.Status -ne 'Valid') { throw 'Wintun signature is not valid' }
$process = Start-Process -FilePath (Join-Path $stage 'routeloom.exe') -WorkingDirectory $stage -PassThru -WindowStyle Hidden
try {
  Start-Sleep -Seconds 8
  $process.Refresh()
  if ($process.HasExited) { throw "Packaged GUI exited with code $($process.ExitCode)" }
  Write-Output 'Packaged Windows GUI started with its bundled runtime.'
} finally {
  if (-not $process.HasExited) { Stop-Process -Id $process.Id }
}
