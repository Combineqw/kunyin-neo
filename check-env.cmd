@echo off
REM Environment self-check launcher (double-click this file).
REM Runs check-env.ps1 with ExecutionPolicy Bypass and keeps the window open.
setlocal
set "PS1=%~dp0check-env.ps1"
if not exist "%PS1%" (
    echo [ERROR] check-env.ps1 not found next to this launcher.
    pause
    exit /b 1
)
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%PS1%" %*
set "RC=%ERRORLEVEL%"
if "%RC%"=="9009" (
    echo.
    echo [ERROR] powershell.exe not found on PATH.
    pause
)
endlocal & exit /b %RC%
