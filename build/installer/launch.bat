@echo off
REM office-agent launcher — starts the dashboard (http://127.0.0.1:9119/)
setlocal
if not defined HERMES_HOME set HERMES_HOME=%LOCALAPPDATA%\hermes
set PATH=%HERMES_HOME%\venv\Scripts;%PATH%
title Office Agent
"%HERMES_HOME%\venv\Scripts\hermes.exe" dashboard --skip-build
endlocal