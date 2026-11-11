@echo off
set PATH=%USERPROFILE%\.cargo\bin;%PATH%
cd /d "%~dp0"
cargo build --release || exit /b 1
if not exist dist mkdir dist
copy /y "target\release\tandem.exe" "dist\Tandem.exe" >nul || (echo Close Tandem first, then run this again & exit /b 1)
echo Built dist\Tandem.exe
