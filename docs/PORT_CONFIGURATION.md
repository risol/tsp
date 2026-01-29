# TSP 端口配置总结

## 📋 端口分配

### 开发服务器
- **端口**: `9000`
- **用途**: 开发模式和生产模式的默认端口
- **配置位置**:
  - `src/main.ts` - DEFAULT_CONFIG
  - `config.example.jsonc`

### 测试服务器
- **端口**: `9001` ✅ (从 9100 修改)
- **用途**: E2E 测试专用端口，避免与开发服务器冲突
- **配置位置**:
  - `tests/run_e2e_tests.ts`
  - `tests/e2e/test_utils.ts`

## 🔧 修改的文件

### 源代码文件
1. ✅ `tests/run_e2e_tests.ts` - TEST_PORT: 9100 → 9001
2. ✅ `tests/e2e/test_utils.ts` - TEST_PORT: 9100 → 9001

### 文档文件
3. ✅ `docs/tasks.md` - 所有测试端口引用
4. ✅ `docs/testing/README.md` - 测试配置说明
5. ✅ `docs/testing/test-pages.md` - 测试示例 URL
6. ✅ `docs/history/DENO_JSON_UPDATE.md` - 历史记录
7. ✅ `docs/features/custom-response.md` - 测试示例
8. ✅ `docs/features/error-handling.md` - 测试示例
9. ✅ `docs/features/redirect.md` - 测试 URL
10. ✅ `docs/history/REDIRECT_IMPLEMENTATION_FIX.md` - 测试 URL

## ✅ 验证

### 类型检查
```bash
deno task check
```
结果: ✅ 通过

### 单元测试
```bash
deno task test:unit
```
结果: ✅ 6 个测试套件全部通过

### E2E 测试
```bash
deno task test:e2e
```
注意: E2E 测试需要使用端口 9001，确保该端口可用

## 📝 使用示例

### 开发模式（端口 9000）
```bash
deno task dev
# 访问: http://localhost:9000
```

### E2E 测试（端口 9001）
```bash
deno task test:e2e
# 自动在端口 9001 启动测试服务器
```

### 同时运行开发服务器和测试
```bash
# 终端 1: 开发服务器
deno task dev  # 端口 9000

# 终端 2: E2E 测试
deno task test:e2e  # 端口 9001
```

## 🔍 端口冲突排查

如果遇到端口占用问题：

### 检查端口占用（Windows）
```bash
netstat -ano | findstr :9000
netstat -ano | findstr :9001
```

### 检查端口占用（Linux/Mac）
```bash
lsof -i :9000
lsof -i :9001
```

### 终止进程
```bash
# Windows
taskkill /F /PID <进程ID>

# Linux/Mac
kill -9 <进程ID>
```

## 📊 端口对比

| 用途 | 旧端口 | 新端口 | 变更 |
|------|--------|--------|------|
| 开发服务器 | 9000 | 9000 | ❌ 无变化 |
| E2E 测试 | 9100 | 9001 | ✅ 已修改 |

## 🎯 优势

1. **更清晰的端口分配**
   - 9000: 开发/生产（主服务）
   - 9001: 测试（接近主端口，易于记忆）

2. **避免冲突**
   - 开发和测试可以同时运行
   - 端口编号连续，便于管理

3. **向后兼容**
   - 默认端口保持不变（9000）
   - 只影响测试端口

## 🔗 相关文档

- [测试文档](./testing/README.md)
- [配置说明](./configuration.md)
- [任务文档](./tasks.md)

---

**修改日期**: 2026-01-29
**修改者**: Claude Code
**状态**: ✅ 完成并验证
