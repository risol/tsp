@echo off
REM Docker 服务启动脚本 (Windows)
REM 用于启动测试所需的 MySQL、Redis 和 LDAP 容器

set LDAP_CONTAINER=tsp-openldap

echo.
echo ╔════════════════════════════════════════╗
echo ║     启动 Docker 测试服务                    ║
echo ╚════════════════════════════════════════╝
echo.

REM 检查 Docker 是否运行
docker info >nul 2>&1
if errorlevel 1 (
    echo ❌ 错误: Docker 未运行
    echo 请先启动 Docker Desktop
    pause
    exit /b 1
)

echo ✓ Docker 运行中
echo.
echo 📋 启动服务...
echo.

REM 启动所有服务
docker-compose up -d

echo.
echo ⏳ 等待 LDAP 服务启动...
timeout /t 5 /nobreak >nul

REM 检查并导入测试用户
echo.
echo 🔍 检查测试用户...
docker exec %LDAP_CONTAINER% ldapsearch -x -H ldap://localhost:389 -b ou=developers,dc=example,dc=org -D "cn=admin,dc=example,dc=org" -w admin123456 "(objectClass=person)" >nul 2>&1
if errorlevel 1 (
    echo 📥 导入测试用户...
    if exist "docker\ldap\test-users.ldif" (
        docker exec -i %LDAP_CONTAINER% ldapadd -x -H ldap://localhost:389 -D "cn=admin,dc=example,dc=org" -w admin123456 ^< docker\ldap\test-users.ldif >nul 2>&1
        if errorlevel 1 (
            echo ⚠️  测试用户导入失败
        ) else (
            echo ✓ 测试用户导入成功 (6个用户^)
        )
    )
) else (
    echo ✓ 测试用户已存在
)

echo.
echo ✅ 服务启动成功！
echo.
echo ══════════════════════════════════════════
echo 📊 服务信息：
echo ══════════════════════════════════════════
echo.
echo 🐬 MySQL 数据库:
echo     Host: 127.0.0.1
echo     Port: 3306
echo     Root Password: root123456
echo     Database: test_db
echo     User: test_user / test123456
echo.
echo 🔴 Redis 缓存:
echo     Host: 127.0.0.1
echo     Port: 6379
echo     No password (默认无密码)
echo.
echo 🔐 LDAP 认证服务:
echo     URL: ldap://localhost:1389
echo     Base DN: dc=example,dc=org
echo     Admin DN: cn=admin,dc=example,dc=org
echo     Admin Password: admin123456
echo.
echo ══════════════════════════════════════════
echo 🌐 管理工具：
echo ══════════════════════════════════════════
echo.
echo   phpMyAdmin (MySQL):    http://localhost:8080
echo   Redis Commander:       http://localhost:8081
echo   phpLDAPadmin (LDAP):   http://localhost:8082
echo.
echo ══════════════════════════════════════════
echo 🔧 常用命令：
echo ══════════════════════════════════════════
echo.
echo   查看日志:
echo     docker-compose logs -f mysql
echo     docker-compose logs -f redis
echo     docker-compose logs -f openldap
echo.
echo   进入 MySQL:
echo     docker-compose exec mysql mysql -uroot -proot123456
echo.
echo   进入 Redis:
echo     docker-compose exec redis redis-cli
echo.
echo   测试 LDAP 连接:
echo     deno run --allow-all tests/unit/ldap_docker_test.ts
echo.
echo   重启服务:
echo     docker-restart.bat
echo.
echo   停止服务:
echo     docker-stop.bat
echo     或: docker-compose down
echo.
echo ✓ 服务已在后台运行
echo.
pause
