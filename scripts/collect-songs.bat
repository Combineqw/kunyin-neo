@echo off
rem  Pure ASCII on purpose: cmd parses .bat with the system ANSI codepage,
rem  so Chinese here would be mojibake under chcp 65001. Node prints all
rem  human-facing text instead.
chcp 65001 >nul
setlocal
cd /d "%~dp0.."
node scripts/collect-songs.mjs
set CODE=%errorlevel%
echo.
pause
exit /b %CODE%
