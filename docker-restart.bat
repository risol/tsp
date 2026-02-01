@echo off
REM Docker 服务重启脚本 (Windows)
REM 快速重启 MySQL、Redis 和 LDAP 容器

echo.
echo ╔════════════════════════════════════════╗
echo ║     重启 Docker 测试服务                    ║
echo ╚════════════════════════════════════════╝
echo.

echo ══════════════════════════════════════════
echo 🔄 重启服务...
echo ══════════════════════════════════════════
echo.

REM 重启所有服务（MySQL、Redis、LDAP）
docker-compose restart

echo.
echo ✅ 服务已重启！
echo.
echo ══════════════════════════════════════════
echo 📊 服务状态：
echo ══════════════════════════════════════════
echo.
docker-compose ps
echo.

REM 显示 LDAP 容器状态
docker ps | findstr /C:"%CONTAINER_NAME%" >nul 2>&1
if not errorlevel 1 (
    docker ps --filter "name=%CONTAINER_NAME%" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
)
echo.

REM 健康检查
echo ══════════════════════════════════════════
echo 🏥 健康检查：
echo ══════════════════════════════════════════
echo.

echo MySQL:
docker exec tsp-mysql mysqladmin ping -h localhost -uroot -proot123456 >nul 2>&1
if errorlevel 1 (
    echo     ✗ 未响应
) else (
    echo     ✓ 运行中
)
echo.

echo Redis:
docker exec tsp-redis redis-cli ping >nul 2>&1
if errorlevel 1 (
    echo     ✗ 未响应
) else (
    echo     ✓ 运行中
)
echo.

echo LDAP:
docker exec %CONTAINER_NAME% ldapsearch -x -H ldap://localhost:389 -b dc=example,dc=org -D "cn=admin,dc=example,dc=org" -w admin123456 "(objectClass=*)" >nul 2>&1
if errorlevel 1 (
    echo     ✗ 未响应
) else (
    echo     ✓ 运行中
)
echo.

echo ══════════════════════════════════════════
echo.
pause
