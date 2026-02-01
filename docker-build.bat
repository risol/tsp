@echo off
REM TSP Server Docker 镜像构建脚本 (Windows)
REM 自动构建并标记 TSP Server Docker 镜像

setlocal EnableDelayedExpansion

:: 配置
set IMAGE_NAME=tspserver
set IMAGE_TAG=%1
if "%IMAGE_TAG%"=="" set IMAGE_TAG=latest
set REGISTRY=%2
if not "%REGISTRY%"=="" set FULL_IMAGE=%REGISTRY%%IMAGE_NAME%:%IMAGE_TAG%
if "%REGISTRY%"=="" set FULL_IMAGE=%IMAGE_NAME%:%IMAGE_TAG%

echo.
echo ╔═══════════════════════════════════════════════════════╗
echo ║     TSP Server Docker 镜像构建                        ║
echo ╚═══════════════════════════════════════════════════════╝
echo.

:: 检查 Docker 是否运行
echo 🔍 检查 Docker 环境...
docker info >nul 2>&1
if errorlevel 1 (
    echo 错误: Docker 未运行，请先启动 Docker
    pause
    exit /b 1
)
echo ✓ Docker 运行中
echo.

:: 显示构建信息
echo 📋 构建信息:
echo    镜像名称: %IMAGE_NAME%
echo    镜像标签: %IMAGE_TAG%
if not "%REGISTRY%"=="" (
    echo    仓库: %REGISTRY%
)
echo.

:: 询问是否清理
set /p CLEANUP="是否删除旧的构建缓存？这将释放磁盘空间 (y/N): "
echo.
if /i "%CLEUP%"=="y" (
    echo.
    echo 🧹 清理旧镜像...
    docker image prune -f
    echo ✓ 清理完成
    echo.
)

:: 构建镜像
echo 🔨 开始构建 Docker 镜像...
echo ════════════════════════════════════════════════════════
echo.

:: 执行构建
if not "%REGISTRY%"=="" (
    REM 如果指定了仓库，构建并推送
    docker buildx build --platform linux/amd64,linux/arm64 -t !FULL_IMAGE! --push .
) else (
    REM 只构建本地镜像
    docker build -t !FULL_IMAGE! .
)

set BUILD_STATUS=%errorlevel%

echo.
echo ════════════════════════════════════════════════════════
echo.

if %BUILD_STATUS%==0 (
    echo ╔══════════════════════════════════════════╗
    echo ║     🎉 构建成功！                            ║
    echo ╚══════════════════════════════════════════╝
    echo.
    echo 📦 镜像信息:
    echo    本地镜像: !FULL_IMAGE!
    echo.
    echo 💡 使用方法:
    echo.
    echo    运行容器:
    echo      docker run -d --name tspserver -p 9000:9000 !FULL_IMAGE!
    echo.
    echo    带环境变量运行:
    echo      docker run -d --name tspserver -p 9000:9000 ^
    echo        -e TSP_PORT=8080 ^
    echo        !FULL_IMAGE!
    echo.
    echo    挂载自定义配置:
    echo      docker run -d --name tspserver -p 9000:9000 ^
    echo        -v C:/path/to/config:/app/config ^
    echo        !FULL_IMAGE! --root /app/www --port 9000
    echo.

    REM 显示镜像大小
    for /f "tokens=*" %%a in ('docker images !IMAGE_NAME!:!IMAGE_TAG! --format "{{.Size}}"') do set SIZE=%%a
    echo    镜像大小: !SIZE!
    echo.
) else (
    echo ╔══════════════════════════════════════════╗
    echo ║     ❌ 构�建失败！                            ║
    echo ╚══════════════════════════════════════════╝
    echo.
    pause
    exit /b 1
)

pause
