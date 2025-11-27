@echo off
REM Batch script to build Rust with Visual Studio environment
set "VS_PATH=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools"
set "VCVARS_PATH=%VS_PATH%\VC\Auxiliary\Build\vcvars64.bat"

if exist "%VCVARS_PATH%" (
    call "%VCVARS_PATH%" >nul 2>&1
    cargo %*
) else (
    echo Error: Visual Studio Build Tools not found at %VS_PATH%
    exit /b 1
)

