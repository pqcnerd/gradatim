# PowerShell script to build Rust with Visual Studio environment
$ErrorActionPreference = "Stop"

# Path to Visual Studio Build Tools
$vsPath = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools"
$vcvarsPath = Join-Path $vsPath "VC\Auxiliary\Build\vcvars64.bat"

if (-not (Test-Path $vcvarsPath)) {
    Write-Host "Error: Visual Studio Build Tools not found at $vsPath" -ForegroundColor Red
    exit 1
}

# Change to the rust-core directory (in case we're called from parent)
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptDir

# Build the cargo command with arguments
$cargoArgs = $args
if ($cargoArgs.Count -eq 0) {
    $cargoArgs = @("run")
}

# Set up Visual Studio environment and run cargo
# We need to call cmd.exe because vcvars64.bat is a batch file
$cargoCommand = "cargo " + ($cargoArgs -join " ")
$fullCommand = "call `"$vcvarsPath`" >nul 2>&1 && $cargoCommand"

# Run the command in cmd.exe
& cmd.exe /c $fullCommand
