# Docker 测试数据清理脚本

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "============================================"
Write-Host "  警告：删除所有测试数据"
Write-Host "============================================"
Write-Host ""
Write-Host "此操作将会："
Write-Host "  - 停止所有容器"
Write-Host "  - 删除所有容器"
Write-Host "  - 删除所有数据卷"
Write-Host ""
Write-Host "以下数据将会丢失："
Write-Host "  • MySQL 数据库数据"
Write-Host "  • Redis 缓存数据"
Write-Host "  • LDAP 目录数据"
Write-Host ""

$confirm = Read-Host "确认要删除所有测试数据吗？请输入 'yes' 确认删除"

if ($confirm -ne "yes") {
    Write-Host ""
    Write-Host "[OK] 操作已取消"
    Write-Host ""
    Read-Host "按 Enter 键退出"
    exit 0
}

Write-Host ""
Write-Host "正在清理..."
Write-Host ""

Write-Host "停止并删除所有容器和数据卷..."
docker-compose down -v

Write-Host ""
Write-Host "============================================"
Write-Host "  清理完成！"
Write-Host "============================================"
Write-Host ""
Write-Host "所有测试数据已删除"
Write-Host ""
Write-Host "后续操作："
Write-Host ""
Write-Host "1. 重新启动服务："
Write-Host "   .\docker-start.ps1"
Write-Host ""
Read-Host "按 Enter 键退出"
