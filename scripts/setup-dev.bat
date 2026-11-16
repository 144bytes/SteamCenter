@echo off
rem SteamCenter: install everything needed to build (scripts\build.bat) on a clean Windows PC.
rem Uses winget; installs only what is missing. Visual Studio Build Tools asks for administrator rights.
chcp 65001 >nul
setlocal
cd /d "%~dp0.."

where winget >nul 2>nul || (
  echo ERROR: winget not found. Install "App Installer" from the Microsoft Store and run this again.
  exit /b 1
)
set "WG=winget install --exact --silent --accept-package-agreements --accept-source-agreements --id"

echo === Visual Studio Build Tools (C++) ===
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
set "VC="
if exist "%VSWHERE%" for /f "usebackq delims=" %%p in (`"%VSWHERE%" -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VC=%%p"
if defined VC (
  echo already installed
) else (
  %WG% Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended" || goto :fail
)

echo === Rust ===
if exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
  echo already installed
) else (
  %WG% Rustlang.Rustup || goto :fail
)
"%USERPROFILE%\.cargo\bin\rustup.exe" default stable-x86_64-pc-windows-msvc || goto :fail

echo === Node.js (UI) ===
where node >nul 2>nul && echo already installed || if exist "%ProgramFiles%\nodejs\node.exe" (echo already installed) else (%WG% OpenJS.NodeJS.LTS || goto :fail)

echo === UI dependencies ===
set "PATH=%ProgramFiles%\nodejs;%PATH%"
call npm --prefix ui ci || goto :fail

echo.
echo Done. Open a new terminal (so PATH is fresh) and run scripts\build.bat.
exit /b 0

:fail
echo.
echo ERROR: installation stopped, see the messages above.
exit /b 1
