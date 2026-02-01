@echo off
REM Docker 服务停止脚本 (Windows)
REM 用于停止并清理 MySQL、Redis 和 LDAP 容器

echo.
echo ╔════════════════════════════════════════╗
echo ║     停止 Docker 测试服务                    ║
echo ╚════════════════════════════════════════╝
echo.

echo ⏹ 停止所有服务...
docker-compose down
echo ✅ 服务已停止
echo.

echo ══════════════════════════════════════════
echo 🗑️  是否要删除数据卷？（所有数据将丢失）
echo ══════════════════════════════════════════
echo.
echo   警告：此操作将删除以下数据：
echo   - MySQL 数据库数据
echo   - Redis 缓存数据
echo   - LDAP 目录数据（包括6个测试用户）
echo.
set /p answer="请输入 'yes' 确认删除，或按 Enter 取消: "

if "%answer%"=="yes" (
    echo.
    echo 删除数据卷...
    docker-compose down -v
    echo ✅ 数据卷已删除
) else (
    echo ✓ 数据卷已保留
)

echo.
echo ✓ 操作完成
echo.
pause
