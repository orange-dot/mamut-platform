# =============================================================================
# PowerShell script to stop the Docker demo environment
# Usage: .\stop.ps1 [-Clean] [-Help]
# =============================================================================

param(
    [switch]$Clean,  # Remove volumes (persistent data)
    [switch]$Help    # Show help
)

$ErrorActionPreference = "Stop"

function Write-Header($text) { Write-Host "`n=== $text ===" -ForegroundColor Cyan }
function Write-Success($text) { Write-Host "[OK] $text" -ForegroundColor Green }
function Write-Info($text) { Write-Host "[INFO] $text" -ForegroundColor Yellow }

if ($Help) {
    Write-Host @"

Stop Azure Durable Functions Demo Environment
=============================================

Usage: .\stop.ps1 [options]

Options:
  -Clean    Remove volumes (clears all persistent data)
  -Help     Show this help message

Examples:
  .\stop.ps1           # Stop containers, keep data
  .\stop.ps1 -Clean    # Stop containers and remove all data

"@
    exit 0
}

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptDir

Write-Header "Stopping Demo Environment"

$composeArgs = @("compose", "down")
if ($Clean) {
    $composeArgs += "-v"
    Write-Info "Will remove volumes (persistent data)"
}

& docker @composeArgs

Write-Success "Environment stopped"
