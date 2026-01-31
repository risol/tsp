# Docker Test Services Stop Script
# Stop MySQL and Redis containers

try {
    Write-Host "========================================" -ForegroundColor Green
    Write-Host "  Stop Docker Test Services             " -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
    Write-Host ""

    # Check for docker-compose
    Write-Host "Checking docker-compose..." -ForegroundColor Yellow
    $composeCmd = $null

    try {
        if (Get-Command "docker-compose" -ErrorAction SilentlyContinue) {
            $composeCmd = "docker-compose"
            Write-Host "Found: docker-compose" -ForegroundColor Green
        } elseif (Get-Command "docker" -ErrorAction SilentlyContinue) {
            $versionOutput = docker compose version 2>&1
            if ($LASTEXITCODE -eq 0) {
                $composeCmd = "docker compose"
                Write-Host "Found: docker compose" -ForegroundColor Green
            }
        }
    } catch {
        Write-Host "Error checking docker commands: $_" -ForegroundColor Yellow
    }

    if (-not $composeCmd) {
        Write-Host "[ERROR] docker-compose not found" -ForegroundColor Red
        Read-Host "Press Enter to exit"
        exit 1
    }

    # Check if containers are running
    Write-Host ""
    Write-Host "Checking container status..." -ForegroundColor Yellow
    $running = & $composeCmd ps

    if ($running | Select-String -Pattern "Up" -Quiet) {
        Write-Host "Stopping services..." -ForegroundColor Yellow
        Write-Host ""
        Write-Host "Data will be preserved" -ForegroundColor Cyan
        Write-Host ""

        & $composeCmd down

        if ($LASTEXITCODE -ne 0) {
            Write-Host "[WARNING] Command exited with code: $LASTEXITCODE" -ForegroundColor Yellow
        }

        Write-Host ""
        Write-Host "========================================" -ForegroundColor Green
        Write-Host "  Services Stopped                     " -ForegroundColor Green
        Write-Host "========================================" -ForegroundColor Green
        Write-Host ""
        Write-Host "[SUCCESS] Services stopped, data preserved" -ForegroundColor Green
    } else {
        Write-Host "[WARNING] No running services found" -ForegroundColor Yellow
    }

    Write-Host ""
    Write-Host "To delete data and reset:" -ForegroundColor Yellow
    Write-Host "  .\docker-cleanup.ps1"
    Write-Host ""

} catch {
    Write-Host ""
    Write-Host "[ERROR] Script failed: $_" -ForegroundColor Red
    Write-Host "Error details: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host ""
}

Read-Host "Press Enter to exit..."
