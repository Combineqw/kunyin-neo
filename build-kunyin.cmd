@echo off
REM =====================================================================
REM  kunyin-desktop build launcher (double-click this file)
REM
REM  Why this wrapper exists:
REM    - Double-clicking a .ps1 opens Notepad instead of running it.
REM    - Right-click 'Run with PowerShell' is blocked when the machine
REM      ExecutionPolicy is Restricted (the Windows client default).
REM  This launcher calls PowerShell with -ExecutionPolicy Bypass, which
REM  applies to this one process only and changes nothing system-wide.
REM
REM  Pass through any arguments, e.g.:
REM    build-kunyin.cmd -Target portable -TypeCheck
REM    build-kunyin.cmd -Verify            (full acceptance gate first)
REM =====================================================================

setlocal
set "PS1=%~dp0build-kunyin.ps1"

if not exist "%PS1%" (
    echo [ERROR] build-kunyin.ps1 not found next to this launcher.
    echo         Expected: %PS1%
    echo         Keep build-kunyin.cmd and build-kunyin.ps1 in the same folder.
    pause
    exit /b 1
)

powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%PS1%" %*
set "RC=%ERRORLEVEL%"

REM 9009 = powershell.exe itself was not found on PATH. In that case the
REM script never ran, so it never got to print anything or pause.
if "%RC%"=="9009" (
    echo.
    echo [ERROR] powershell.exe not found on PATH.
    echo         Try the full path:
    echo         %%SystemRoot%%\System32\WindowsPowerShell\v1.0\powershell.exe
    pause
)

endlocal & exit /b %RC%
