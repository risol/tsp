#!/bin/bash
# 快速编译和测试脚本

echo "🧹 清理旧的可执行文件..."
rm -f tsp-fpm.exe

echo "✅ 验证源代码..."
deno check www/form.tsx www/api.tsx www/components/Layout.tsx

echo "📦 编译新的可执行文件..."
deno compile --allow-net --allow-read --output tsp-fpm src/main.ts

echo "⏰ 验证时间戳..."
ls -lh tsp-fpm.exe

echo ""
echo "✨ 编译完成！现在可以运行："
echo "   ./tsp-fpm.exe -r ./www -p 9000 --dev"
echo ""
