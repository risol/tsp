#!/bin/bash

echo "=== TSP 文件管理器功能测试 ==="
echo ""

BASE_URL="http://localhost:9000/__filemanager"

echo "1. 测试主页访问..."
curl -s "$BASE_URL/" | grep -o "<title>.*</title>"
echo ""

echo "2. 测试登录 API..."
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/login" \
  -H "Content-Type: application/json" \
  -d '{"password":"admin123"}')
echo "$LOGIN_RESPONSE" | head -5
echo ""

# 提取 session ID (如果有)
echo "3. 测试登出 API..."
curl -s -X POST "$BASE_URL/api/logout" \
  -H "Content-Type: application/json"
echo ""

echo "=== 测试完成 ==="
echo ""
echo "请在浏览器中打开以下地址进行手动测试："
echo "URL: http://localhost:9000/__filemanager"
echo "密码: admin123"
