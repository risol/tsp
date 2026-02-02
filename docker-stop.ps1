# Docker 服务停止脚本 (Windows)

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "============================================"
Write-Host "  停止 Docker 测试服务"
Write-Host "============================================"
Write-Host ""

Write-Host "停止所有服务..."
docker-compose down
Write-Host "[OK] 服务已停止" -ForegroundColor Green
Write-Host ""

Write-Host "============================================"
Write-Host "是否要删除数据卷？（所有数据将丢失）"
Write-Host "============================================"
Write-Host ""
Write-Host "  警告：此操作将删除以下数据："
Write-Host "  - MySQL 数据库数据"
Write-Host "  - Redis 缓存数据"
Write-Host "  - LDAP 目录数据（包括6个测试用户）"
Write-Host ""

$answer = Read-Host "请输入 'yes' 确认删除，或按 Enter 取消"

if ($answer -eq "yes") {
    Write-Host ""
    Write-Host "删除数据卷..."
    docker-compose down -v
    Write-Host "[OK] 数据卷已删除" -ForegroundColor Green
} else {
    Write-Host "[OK] 数据卷已保留"
}

Write-Host ""
Write-Host "[OK] 操作完成"
Write-Host ""
Read-Host "按 Enter 键退出"
