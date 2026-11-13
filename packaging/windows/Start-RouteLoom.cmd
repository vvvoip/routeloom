@echo off
setlocal
set "ROUTELOOM_PACKAGE_DIR=%~dp0"
powershell.exe -NoProfile -Command "Start-Process -FilePath (Join-Path $env:ROUTELOOM_PACKAGE_DIR 'routeloom.exe') -WorkingDirectory $env:ROUTELOOM_PACKAGE_DIR -Verb RunAs"
if errorlevel 1 pause
