# Docker Test Services Restart Script
# Quick restart MySQL and Redis containers

Write-Host "========================================" -ForegroundColor Green
Write-Host "     Restart Docker Test Services          " -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""

# Check for docker-compose
$composeCmd = $null
if (Get-Command "docker-compose" -ErrorAction SilentlyContinue) {
    $composeCmd = "docker-compose"
} elseif (Get-Command "docker" -ErrorAction SilentlyContinue) {
    if (docker compose version 2>&1 | Out-Null) {
        $composeCmd = "docker compose"
    }
}

if (-not $composeCmd) {
    Write-Host "[ERROR] docker-compose not found" -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}

Write-Host "Restarting services..." -ForegroundColor Yellow
Write-Host ""

# Restart services
& $composeCmd restart

Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "     Services Restarted!                  " -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Service Status:" -ForegroundColor Yellow
& $composeCmd ps

Write-Host ""
Read-Host "Press Enter to continue..."
