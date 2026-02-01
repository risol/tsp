@echo off
REM Docker 测试数据清理脚本
REM 删除所有容器和数据卷，完全清理测试数据

echo.
echo ╔════════════════════════════════════════╗
echo ║     ⚠️  警告：删除所有测试数据 ⚠️        ║
echo ╚════════════════════════════════════════╝
echo.
echo 此操作将会：
echo   ✓ 停止所有容器
echo   ✓ 删除所有容器
echo   ✓ 删除所有数据卷（MySQL、Redis、LDAP数据）
echo.
echo 以下数据将会丢失：
echo   • MySQL 数据库（test_db）及所有表和数据
echo   • Redis 缓存中的所有数据
echo   • LDAP 目录服务中的所有数据（包括6个测试用户）
echo   • 所有已创建的用户、会话、文章等
echo.
echo 提示：数据删除后，重新启动服务将重新初始化测试数据
echo.

REM 确认操作
set /p confirm="确认要删除所有测试数据吗？请输入 'yes' 确认删除: "

if not "%confirm%"=="yes" (
    echo.
    echo ✓ 操作已取消
    echo.
    pause
    exit /b 0
)

echo.
echo 正在清理...
echo.

REM 删除所有容器及数据卷
echo 📦 停止并删除所有容器...
docker-compose down -v

echo.
echo ╔════════════════════════════════════════╗
echo ║     ✓ 清理完成！                        ║
echo ╚════════════════════════════════════════╝
echo.
echo 所有测试数据已删除
echo.
echo 💡 后续操作：
echo.
echo 1. 重新启动服务（将重新初始化所有数据）：
echo    docker-start.bat
echo.
echo 2. 仅导入 LDAP 测试用户：
echo    docker\import-ldap-users.bat
echo.
echo 3. 或完全不使用 Docker：
echo    无需操作
echo.
pause
