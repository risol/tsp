# Docker 服务重启脚本 (Windows)

$ErrorActionPreference = "Stop"
$CONTAINER_NAME = "tsp-openldap"

Write-Host ""
Write-Host "============================================"
Write-Host "  重启 Docker 测试服务"
Write-Host "============================================"
Write-Host ""

Write-Host "重启服务..."
docker-compose restart

Write-Host ""
Write-Host "[SUCCESS] 服务已重启!" -ForegroundColor Green
Write-Host ""
Write-Host "============================================"
Write-Host "服务状态："
Write-Host "============================================"
Write-Host ""
docker-compose ps
Write-Host ""

# 健康检查
Write-Host "============================================"
Write-Host "健康检查："
Write-Host "============================================"
Write-Host ""

Write-Host "MySQL:" -NoNewline
try {
    docker exec tsp-mysql mysqladmin ping -h localhost -uroot -proot123456 | Out-Null
    Write-Host "     [OK] 运行中" -ForegroundColor Green
} catch {
    Write-Host "     [FAIL] 未响应" -ForegroundColor Red
}

Write-Host "Redis:" -NoNewline
try {
    docker exec tsp-redis redis-cli ping | Out-Null
    Write-Host "     [OK] 运行中" -ForegroundColor Green
} catch {
    Write-Host "     [FAIL] 未响应" -ForegroundColor Red
}

Write-Host "LDAP:" -NoNewline
try {
    docker exec $CONTAINER_NAME ldapsearch -x -H ldap://localhost:389 -b dc=example,dc=org -D "cn=admin,dc=example,dc=org" -w admin123456 "(objectClass=*)" | Out-Null
    Write-Host "     [OK] 运行中" -ForegroundColor Green
} catch {
    Write-Host "     [FAIL] 未响应" -ForegroundColor Red
}

Write-Host ""
Write-Host "============================================"
Write-Host ""
Read-Host "按 Enter 键退出"
