@echo off
rem  Switch console to UTF-8 so the Node reports render Chinese correctly.
rem  This file itself stays pure ASCII on purpose: cmd parses .bat with the
rem  system ANSI codepage, so Chinese inside a .bat would be mojibake under
rem  chcp 65001. All human-facing text is printed by Node instead.
chcp 65001 >nul
setlocal
cd /d "%~dp0.."
node scripts/native-test.mjs
set CODE=%errorlevel%
echo.
pause
exit /b %CODE%
