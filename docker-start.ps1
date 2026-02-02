# Docker 服务启动脚本 (Windows)
# 用于启动测试所需的 MySQL、Redis 和 LDAP 容器

$ErrorActionPreference = "Stop"
$LDAP_CONTAINER = "tsp-openldap"

Write-Host ""
Write-Host "============================================"
Write-Host "  启动 Docker 测试服务"
Write-Host "============================================"
Write-Host ""

# 检查 Docker 是否运行
try {
    docker info | Out-Null
    Write-Host "[OK] Docker 运行中" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] Docker 未运行" -ForegroundColor Red
    Write-Host "请先启动 Docker Desktop"
    Read-Host "按 Enter 键退出"
    exit 1
}

Write-Host ""
Write-Host "启动服务..."
Write-Host ""

# 启动所有服务
docker-compose up -d

Write-Host ""
Write-Host "等待 LDAP 服务启动..."
Start-Sleep -Seconds 5

# 检查并导入测试用户
Write-Host ""
Write-Host "检查测试用户..."

try {
    docker exec $LDAP_CONTAINER ldapsearch -x -H ldap://localhost:389 -b ou=developers,dc=example,dc=org -D "cn=admin,dc=example,dc=org" -w admin123456 "(objectClass=person)" | Out-Null
    Write-Host "[OK] 测试用户已存在" -ForegroundColor Green
} catch {
    Write-Host "导入测试用户..."
    if (Test-Path "docker\ldap\test-users.ldif") {
        $ldifContent = Get-Content "docker\ldap\test-users.ldif" -Raw
        docker exec -i $LDAP_CONTAINER ldapadd -x -H ldap://localhost:389 -D "cn=admin,dc=example,dc=org" -w admin123456 ([IO.MemoryStream]::new([System.Text.Encoding]::UTF8.GetBytes($ldifContent))) | Out-Null

        if ($LASTEXITCODE -ne 0) {
            Write-Host "[WARNING] 测试用户导入失败" -ForegroundColor Yellow
        } else {
            Write-Host "[OK] 测试用户导入成功 (6个用户)" -ForegroundColor Green
        }
    }
}

Write-Host ""
Write-Host "[SUCCESS] 服务启动成功!" -ForegroundColor Green
Write-Host ""
Write-Host "============================================"
Write-Host "服务信息："
Write-Host "============================================"
Write-Host ""
Write-Host "MySQL 数据库:"
Write-Host "  Host: 127.0.0.1:3306"
Write-Host "  Root Password: root123456"
Write-Host "  Database: test_db"
Write-Host "  User: test_user / test123456"
Write-Host ""
Write-Host "Redis 缓存:"
Write-Host "  Host: 127.0.0.1:6379"
Write-Host "  No password"
Write-Host ""
Write-Host "LDAP 认证服务:"
Write-Host "  URL: ldap://localhost:1389"
Write-Host "  Base DN: dc=example,dc=org"
Write-Host "  Admin DN: cn=admin,dc=example,dc=org"
Write-Host "  Admin Password: admin123456"
Write-Host ""
Write-Host "============================================"
Write-Host "管理工具："
Write-Host "============================================"
Write-Host ""
Write-Host "  phpMyAdmin:    http://localhost:8080"
Write-Host "  Redis Commander: http://localhost:8081"
Write-Host "  phpLDAPadmin:   http://localhost:8082"
Write-Host ""
Write-Host "============================================"
Write-Host "常用命令："
Write-Host "============================================"
Write-Host ""
Write-Host "  重启服务: .\docker-restart.ps1"
Write-Host "  停止服务: .\docker-stop.ps1"
Write-Host ""
Write-Host "[OK] 服务已在后台运行" -ForegroundColor Green
Write-Host ""
Read-Host "按 Enter 键退出"
