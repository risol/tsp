@echo off
REM Linux 二进制文件构建脚本 (Windows)
REM 使用 Docker 构建跨平台的 Linux 二进制文件

echo.
echo ╔════════════════════════════════════════╗
echo ║   构建 Linux x64 二进制文件            ║
echo ╚════════════════════════════════════════╝
echo.

REM 检查 Docker 是否运行
docker info >nul 2>&1
if errorlevel 1 (
    echo 错误: Docker 未运行，请先启动 Docker
    pause
    exit /b 1
)

echo ✓ Docker 运行中
echo.

REM 清理旧的构建产物
echo 清理旧的构建产物...
if exist output\tspserver-linux-x64.tar.gz del /q output\tspserver-linux-x64.tar.gz
if exist output rmdir /s /q output
mkdir output
echo ✓ 清理完成
echo.

REM 构建 Docker 镜像
echo 构建 Docker 镜像...
docker build -t tspserver-linux-builder .
if errorlevel 1 (
    echo 错误: Docker 镜像构建失败
    pause
    exit /b 1
)
echo ✓ Docker 镜像构建完成
echo.

REM 运行容器并复制构建产物
echo 生成二进制文件并打包...
docker create --name temp-build tspserver-linux-builder >nul 2>&1
docker cp temp-build:/output/tspserver-linux-x64.tar.gz output/tspserver-linux-x64.tar.gz
docker rm temp-build >nul 2>&1
if errorlevel 1 (
    echo 错误: 二进制文件生成失败
    pause
    exit /b 1
)
echo ✓ 打包完成
echo.

REM 检查构建产物
if exist "output\tspserver-linux-x64.tar.gz" (
    echo ╔════════════════════════════════════════╗
    echo ║   构建成功！                            ║
    echo ╚════════════════════════════════════════╝
    echo.
    echo 文件位置: output\tspserver-linux-x64.tar.gz
    echo.

    REM 显示文件大小（使用 PowerShell 获取友好格式）
    for /f "delims=" %%F in ('powershell -command "(Get-Item 'output\tspserver-linux-x64.tar.gz').Length / 1MB | ToString 'F2'"') do set SIZE=%%F
    echo 文件大小: %SIZE% MB
    echo.

    echo 使用方法:
    echo   1. 将文件传输到 Linux 服务器
    echo   2. 解压: tar -xzf tspserver-linux-x64.tar.gz
    echo   3. 进入目录: cd tspserver
    echo   4. 运行: ./tspserver --root ./www --port 9000
    echo.
) else (
    echo 错误: 构建失败，找不到输出文件
    pause
    exit /b 1
)

pause
