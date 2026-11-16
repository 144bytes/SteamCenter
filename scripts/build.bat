@echo off
rem SteamCenter: run the tests, then build the program. The result goes to release\SteamCenter.exe.
chcp 65001 >nul
setlocal
cd /d "%~dp0.."
where cargo >nul 2>nul || set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

where cargo >nul 2>nul || (
  echo Rust not found: run scripts\setup-dev.bat, or install it from https://rustup.rs
  goto :fail
)
where npm >nul 2>nul || (
  echo Node.js not found: run scripts\setup-dev.bat, or install the LTS version from https://nodejs.org
  goto :fail
)

echo.
echo ==== SteamCenter build ====
echo.

if not exist ui\node_modules (
  echo === UI dependencies ===
  call npm --prefix ui ci || goto :fail
)

echo === 1/2 Tests ===
cargo test --workspace || goto :fail

echo === 2/2 SteamCenter.exe ===
rem The Tauri CLI reads tauri.conf.json from the current folder and builds the UI itself.
pushd app
call "%~dp0..\ui\node_modules\.bin\tauri.cmd" build || (popd & goto :fail)
popd

if not exist release mkdir release
copy /y target\release\SteamCenter.exe release\SteamCenter.exe >nul || goto :fail

echo.
echo ==== Done: release\SteamCenter.exe (single file, no installation) ====
echo.
pause
exit /b 0

:fail
echo.
echo ERROR: the build stopped, see the messages above.
pause
exit /b 1
