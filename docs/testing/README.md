# TSP 测试文档

本文档介绍 TSP 的测试框架和测试方法。

## 📚 测试文档

### 测试指南

- [测试概述](./overview.md) - 测试框架介绍和运行方法
- [测试页面设置](./test-pages.md) - 测试页面配置和使用
- [测试 WWW 说明](./test-www.md) - 测试页面目录说明
- [任务权限说明](./task-permissions.md) - Deno 任务权限配置

## 🚀 快速开始

### 运行所有测试

```bash
deno task test
```

### 运行单元测试

```bash
deno task test:unit
```

### 运行 E2E 测试

```bash
deno task test:e2e
```

### 运行特定测试

```bash
deno test --allow-net tests/unit/router_test.ts
```

## 📁 测试结构

```
tests/
├── unit/                    # 单元测试（不启动服务器）
│   ├── router_test.ts      # 路由解析测试
│   ├── context_test.ts     # 上下文构建测试
│   ├── security_test.ts    # 安全检查测试
│   └── injection_test.ts   # 依赖注入测试
│
├── e2e/                     # E2E 测试（启动二进制服务器）
│   └── test_utils.ts       # 共享工具函数
│
├── test_www/                # 测试页面
│   ├── index.tsx           # 首页测试
│   ├── form.tsx            # 表单测试
│   ├── api.tsx             # API 测试
│   └── error.tsx           # 错误处理测试
│
├── run_unit_tests.ts        # 单元测试运行器
├── run_e2e_tests.ts         # E2E 测试运行器
└── run_all_tests.ts         # 全部测试运行器
```

## 🔧 测试配置

### E2E 测试配置

- **测试端口**: 9100（避免与开发服务器冲突）
- **测试目录**: `./tests/test_www`
- **二进制名称**: `tspserver-test` / `tspserver-test.exe`

### 单元测试配置

- **无需启动服务器** - 直接测试模块功能
- **快速执行** - 适合开发时快速验证
- **隔离测试** - 每个测试文件独立运行

## 📊 测试覆盖

当前测试覆盖以下功能：

- ✅ 路由解析和路径映射
- ✅ 上下文构建和类型检查
- ✅ 安全检查（路径穿越、文件类型）
- ✅ 依赖注入功能
- ✅ HTTP 基本功能（GET, POST）
- ✅ 表单处理
- ✅ 错误处理
- ✅ 重定向功能
- ✅ 自定义响应

## 🔗 相关文档

- [开发指南](../development.md) - 如何编写测试
- [任务文档](../tasks.md) - Deno 任务说明
- [功能文档](../features/README.md) - 被测试的功能说明

---

[← 返回文档中心](../README.md)
