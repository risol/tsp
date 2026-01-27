# Windows PowerShell 编译和测试脚本

Write-Host "🧹 清理旧的可执行文件..." -ForegroundColor Green
Remove-Item tsp-fpm.exe -Force -ErrorAction SilentlyContinue

Write-Host "✅ 验证源代码..." -ForegroundColor Green
deno check www/form.tsx www/api.tsx www/components/Layout.tsx

Write-Host "📦 编译新的可执行文件..." -ForegroundColor Green
deno compile --allow-net --allow-read --output tsp-fpm src/main.ts

Write-Host "⏰ 验证时间戳..." -ForegroundColor Green
ls -lh tsp-fpm.exe

Write-Host ""
Write-Host "✨ 编译完成！现在可以运行：" -ForegroundColor Cyan
Write-Host "   ./tsp-fpm.exe -r ./www -p 9000 --dev" -ForegroundColor Yellow
Write-Host ""
