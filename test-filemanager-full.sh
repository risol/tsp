#!/bin/bash

echo "=== TSP 文件管理器完整测试 ==="
echo ""

BASE_URL="http://localhost:9000/__filemanager"

echo "1. 测试登录页面..."
PAGE_TITLE=$(curl -s "$BASE_URL/" | grep -o "<title>.*</title>")
if [ "$PAGE_TITLE" = "<title>文件管理器 - 登录</title>" ]; then
  echo "✓ 登录页面正常"
else
  echo "✗ 登录页面异常: $PAGE_TITLE"
fi
echo ""

echo "2. 测试登录 API..."
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/login" \
  -H "Content-Type: application/json" \
  -d '{"password":"admin123"}' \
  -c /tmp/fm_test_cookies.txt)

if echo "$LOGIN_RESPONSE" | grep -q "success"; then
  echo "✓ 登录 API 正常"
  CSRF_TOKEN=$(echo "$LOGIN_RESPONSE" | grep -o '"csrfToken":"[^"]*"' | cut -d'"' -f4)
  echo "  CSRF Token: ${CSRF_TOKEN:0:20}..."
else
  echo "✗ 登录 API 失败: $LOGIN_RESPONSE"
  exit 1
fi
echo ""

echo "3. 测试登录后主页（带 session cookie）..."
MAIN_PAGE=$(curl -s "$BASE_URL/" -b /tmp/fm_test_cookies.txt)
MAIN_TITLE=$(echo "$MAIN_PAGE" | grep -o "<title>.*</title>")

if [ "$MAIN_TITLE" = "<title>文件管理器</title>" ]; then
  echo "✓ 登录后主页面正常"

  # 检查 JavaScript 中是否包含 window 错误
  if echo "$MAIN_PAGE" | grep -q "let currentPath = '/'"; then
    echo "✓ JavaScript 初始化代码已修复"
  else
    echo "✗ JavaScript 初始化代码可能有问题"
  fi
else
  echo "✗ 主页面异常（可能未认证）: $MAIN_TITLE"
  echo "  提示: 这可能是因为 session cookie 设置问题"
fi
echo ""

echo "4. 测试浏览目录 API..."
BROWSE_RESPONSE=$(curl -s "$BASE_URL/api/browse?path=D:/GitHub/tsp/www" \
  -b /tmp/fm_test_cookies.txt)

if echo "$BROWSE_RESPONSE" | grep -q "success"; then
  echo "✓ 浏览目录 API 正常"
else
  echo "✗ 浏览目录 API 失败"
  echo "  响应: ${BROWSE_RESPONSE:0:100}..."
fi
echo ""

echo "5. 测试登出 API..."
LOGOUT_RESPONSE=$(curl -s -X POST "$BASE_URL/api/logout" \
  -b /tmp/fm_test_cookies.txt)

if echo "$LOGOUT_RESPONSE" | grep -q "success"; then
  echo "✓ 登出 API 正常"
else
  echo "✗ 登出 API 失败"
fi
echo ""

echo "=== 测试完成 ==="
echo ""
echo "请在浏览器中手动测试以验证前端功能："
echo "URL: $BASE_URL"
echo "密码: admin123"
