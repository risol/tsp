# TSP 测试文档

## 测试结构

```
tests/
├── unit/                     # 单元测试（不启动服务器）
│   ├── router_test.ts       # 路由解析测试
│   ├── context_test.ts      # 上下文构建测试
│   ├── injection_test.ts    # 依赖注入测试
│   └── security_test.ts     # 安全检查测试
│
├── test_www/                 # E2E测试页面
│   ├── index.tsx            # 首页
│   ├── api.tsx              # API测试页
│   ├── form.tsx             # 表单测试页
│   ├── injection.tsx        # 依赖注入测试页
│   ├── jsx-imports.tsx      # JSX Import测试页
│   ├── components/          # JSX组件（用于测试）
│   │   ├── Header.tsx
│   │   └── Card.tsx
│   └── utils/               # 工具函数（用于测试）
│       └── helpers.ts
│
├── run_unit_tests.ts        # 单元测试运行器
├── run_e2e_tests.ts         # E2E测试运行器（二进制文件）
├── test_jsx_imports.ts      # JSX Import专项测试（源码模式）
└── run_all_tests.ts         # 全部测试运行器
```

## 运行测试

```bash
# 所有测试
deno task test

# 只运行单元测试
deno task test:unit

# 只运行E2E测试（二进制文件）
deno task test:e2e

# JSX Import专项测试（源码模式）
deno task test:jsx-imports

# 依赖注入测试
deno task test:injection

# 单个测试文件
deno test --allow-all tests/unit/router_test.ts
```

## 测试套件说明

### 1. 单元测试 (`test:unit`)

测试单个模块功能，不启动服务器：
- 路由解析 (`router_test.ts`)
- 上下文构建 (`context_test.ts`)
- 依赖注入 (`injection_test.ts`)
- 安全检查 (`security_test.ts`)

### 2. E2E测试 (`test:e2e`)

使用编译后的二进制文件进行端到端测试：
- 编译并启动服务器
- 基本 HTTP 功能
- API 响应
- 错误处理
- 安全性（路径穿越防护）

**注意**：由于二进制文件的限制，E2E 测试不包含 JSX 组件导入测试。

### 3. JSX Import测试 (`test:jsx-imports`)

**专门测试 JSX 组件和 TS 工具函数导入功能**（仅在源码模式下运行）：

**测试覆盖**：
- ✅ JSX 组件导入（`Header.tsx`, `Card.tsx`）
- ✅ TS 工具函数导入（`helpers.ts`）
- ✅ 组件嵌套渲染
- ✅ 非 src 目录导入
- ✅ 动态数据传递

**为什么单独测试？**
- 编译后的二进制文件无法正确处理 JSX 组件导入
- 这是 `deno compile` 的已知限制
- 详见 [二进制文件限制说明](../binary-limitations.md)

**运行方式**：
```bash
deno task test:jsx-imports
```

## 测试最佳实践

### 开发阶段

使用源码模式，支持所有功能：
```bash
# 开发模式（热重载）
deno task dev

# 或启动服务器
deno task start
```

### 测试阶段

运行完整测试套件：
```bash
# 所有测试
deno task test

# 或分别运行
deno task test:unit
deno task test:e2e
deno task test:jsx-imports
```

### 部署前

确保二进制文件的基本功能正常：
```bash
# E2E测试（使用二进制文件）
deno task test:e2e
```

## 已知限制

详见 [二进制文件限制说明](../binary-limitations.md)：

1. **JSX 组件导入** - 二进制文件中不支持
2. **TS 工具函数导入** - 二进制文件中不支持
3. **相对路径导入** - 二进制文件中有限支持

这些功能在源码模式（`deno run`）下完全正常。

## 相关文档

- [测试文档首页](./README.md) - 测试文档导航
- [测试页面设置](./test-pages.md) - 测试页面配置
- [任务权限说明](./task-permissions.md) - Deno 任务权限
- [二进制文件限制说明](../binary-limitations.md) - 已知限制和解决方案
- [开发指南](../development.md) - 如何编写测试

---

[← 返回测试文档](./README.md) | [← 返回文档中心](../README.md)
