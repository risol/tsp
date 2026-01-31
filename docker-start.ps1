# Docker Test Services Startup Script
# Start MySQL and Redis containers for testing

Write-Host "========================================" -ForegroundColor Green
Write-Host "     Start Docker Test Services           " -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""

# Check if Docker is running
try {
    docker info | Out-Null
} catch {
    Write-Host "[ERROR] Docker is not running" -ForegroundColor Red
    Write-Host "Please start Docker Desktop first"
    Read-Host "Press Enter to exit"
    exit 1
}

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

Write-Host "Starting services..." -ForegroundColor Yellow
Write-Host ""

# Start services
& $composeCmd up -d

Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "     Services Started Successfully!       " -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Service Information:" -ForegroundColor Yellow
Write-Host ""
Write-Host "  MySQL Database:"
Write-Host "    Host: 127.0.0.1"
Write-Host "    Port: 3306"
Write-Host "    Root Password: root123456"
Write-Host "    Database: test_db"
Write-Host "    User: test_user / test123456"
Write-Host ""
Write-Host "  Redis Cache:"
Write-Host "    Host: 127.0.0.1"
Write-Host "    Port: 6379"
Write-Host "    No password (default)"
Write-Host ""
Write-Host "  Management Tools:"
Write-Host "    phpMyAdmin: http://localhost:8080"
Write-Host "    Redis Commander: http://localhost:8081"
Write-Host ""
Write-Host "Common Commands:"
Write-Host ""
Write-Host "  View logs:"
Write-Host "    $composeCmd logs -f mysql"
Write-Host "    $composeCmd logs -f redis"
Write-Host ""
Write-Host "  Enter MySQL:"
Write-Host "    $composeCmd exec mysql mysql -uroot -proot123456"
Write-Host ""
Write-Host "  Enter Redis:"
Write-Host "    $composeCmd exec redis redis-cli"
Write-Host ""
Write-Host "  Stop services:"
Write-Host "    docker-stop.ps1"
Write-Host "    Or: $composeCmd down"
Write-Host ""
Write-Host "Services are running in the background" -ForegroundColor Green

Write-Host ""
Read-Host "Press Enter to continue..."
