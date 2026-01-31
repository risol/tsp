# Docker Test Services Cleanup Script
# Delete all containers and data volumes

try {
    Write-Host "========================================" -ForegroundColor Red
    Write-Host "  WARNING: Delete All Test Data         " -ForegroundColor Red
    Write-Host "========================================" -ForegroundColor Red
    Write-Host ""

    Write-Host "This operation will:" -ForegroundColor Yellow
    Write-Host "  [X] Stop all containers" -ForegroundColor Red
    Write-Host "  [X] Delete all containers" -ForegroundColor Red
    Write-Host "  [X] Delete all data volumes (MySQL and Redis)" -ForegroundColor Red
    Write-Host ""

    Write-Host "The following data will be lost:" -ForegroundColor Yellow
    Write-Host "  - MySQL database (test_db) with all tables and data"
    Write-Host "  - Redis cache data"
    Write-Host "  - All created users, sessions, posts, etc."
    Write-Host ""

    Write-Host "Note: After deletion, restart will reinitialize test data" -ForegroundColor Cyan
    Write-Host ""

    # Confirm operation
    Write-Host "Confirm to delete all test data?" -ForegroundColor Yellow
    $confirm = Read-Host "Type 'yes' to confirm deletion"

    if ($confirm -ne "yes") {
        Write-Host "Operation cancelled" -ForegroundColor Green
        Read-Host "Press Enter to exit"
        exit 0
    }

    # Check for docker-compose
    Write-Host ""
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
        Write-Host "Please install Docker Desktop first" -ForegroundColor Yellow
        Read-Host "Press Enter to exit"
        exit 1
    }

    Write-Host ""
    Write-Host "Cleaning up..." -ForegroundColor Yellow
    Write-Host ""

    # Delete containers and volumes
    Write-Host "Stopping and removing containers..." -ForegroundColor Yellow

    try {
        & $composeCmd down -v
        if ($LASTEXITCODE -ne 0) {
            Write-Host "[WARNING] Command exited with code: $LASTEXITCODE" -ForegroundColor Yellow
        }
    } catch {
        Write-Host "Error during cleanup: $_" -ForegroundColor Yellow
    }

    Write-Host ""
    Write-Host "========================================" -ForegroundColor Green
    Write-Host "  Cleanup Complete!                     " -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
    Write-Host ""
    Write-Host "All test data has been deleted" -ForegroundColor Green
    Write-Host ""
    Write-Host "Next steps:" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "1. Restart services (will reinitialize test data):" -ForegroundColor Cyan
    Write-Host "   .\docker-start.ps1"
    Write-Host ""
    Write-Host "2. Or do not use Docker:"
    Write-Host "   No action needed"
    Write-Host ""

} catch {
    Write-Host ""
    Write-Host "[ERROR] Script failed: $_" -ForegroundColor Red
    Write-Host "Error details: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host ""
}

Read-Host "Press Enter to exit..."
