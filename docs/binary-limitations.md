# 二进制文件限制说明

## 已知限制

编译后的二进制文件（使用 `deno compile`）存在以下已知限制：

### 1. JSX 组件导入问题 ⚠️

**问题描述**：编译后的二进制文件无法正确处理 TSX 文件中对其他 JSX 组件的导入。

**示例**：
```tsx
// www/index.tsx
import { Header } from "./components/Header.tsx";  // ❌ 在二进制文件中会失败
import { Card } from "./components/Card.tsx";      // ❌ 在二进制文件中会失败

export default async function(context: PageContext) {
  return (
    <div>
      <Header title="Hello" />
      <Card content="World" />
    </div>
  );
}
```

**错误信息**：
```
[FALLBACK] Using @deno/loader for www\index.tsx
Request error: Import "./components/Layout.tsx" not a dependency and not in import map from
```

**原因**：
- 二进制文件无法直接 import TSX 文件
- 需要使用 `@deno/loader` 进行转译
- 转译后的相对路径导入无法正确解析

**解决方案**：
1. **在开发/源码模式下使用**：使用 `deno run --allow-* src/main.ts` 运行服务器
2. **将所有 JSX 写在一个文件中**：对于简单的页面，可以不使用组件导入
3. **使用内联组件**：在同一个文件中定义和使用组件

### 2. TS 工具函数导入问题 ⚠️

**问题描述**：编译后的二进制文件也无法正确导入非 src 目录下的 TS 工具函数。

**示例**：
```tsx
// www/utils/helpers.ts
export function formatDate(date: Date): string {
  // ...
}

// www/index.tsx
import { formatDate } from "./utils/helpers.ts";  // ❌ 在二进制文件中会失败

export default async function(context: PageContext) {
  const today = formatDate(new Date());
  return <div>{today}</div>;
}
```

**解决方案**：
- 将工具函数放在 src 目录下（如果需要在二进制文件中使用）
- 或者使用 `deno run` 模式

## 功能对比

| 功能 | `deno run` 模式 | 编译后的二进制 |
|------|----------------|---------------|
| 基本 TSX 页面 | ✅ | ✅ |
| 导入 src/ 下的模块 | ✅ | ✅ |
| 导入 JSX 组件 | ✅ | ❌ |
| 导入非 src/ 的 TS 文件 | ✅ | ❌ |
| 相对路径导入 | ✅ | ⚠️ 有限支持 |
| 依赖注入 | ✅ | ✅ |
| 热重载 | ✅ (开发模式) | ❌ |

## 测试建议

### 开发/测试时

使用源码模式进行开发和测试：

```bash
# 开发模式（支持热重载）
deno task dev

# 启动服务器
deno task start
```

### 部署时

如果需要部署到生产环境：

1. **避免使用 JSX 组件导入**
   ```tsx
   // ✅ 推荐 - 所有 JSX 在一个文件中
   export default async function(context: PageContext) {
     return (
       <div>
         <header>Header Here</header>
         <main>Content Here</main>
       </div>
     );
   }
   ```

2. **使用已验证的模式**
   - 基本页面功能
   - API 路由
   - 重定向
   - 依赖注入

3. **运行完整测试**
   ```bash
   # 运行所有测试
   deno task test

   # 或分别运行
   deno task test:unit
   deno task test:e2e
   deno task test:jsx-imports  # JSX import 功能测试（仅源码模式）
   ```

## JSX Import 专项测试

我们提供了专门的 JSX Import 测试套件，该测试只在源码模式下运行：

```bash
deno task test:jsx-imports
```

**测试覆盖**：
- ✅ JSX 组件导入（Header.tsx, Card.tsx）
- ✅ TS 工具函数导入（helpers.ts）
- ✅ 组件嵌套渲染
- ✅ 非 src 目录导入
- ✅ 动态数据传递

**测试文件**：
- `tests/test_jsx_imports.ts` - 测试套件
- `tests/test_www/jsx-imports.tsx` - 测试页面
- `tests/test_www/components/` - JSX 组件
- `tests/test_www/utils/helpers.ts` - 工具函数

## 技术细节

### 为什么会出现这个问题？

1. **Deno compile 的限制**
   - Deno compile 会将所有依赖打包进二进制文件
   - 但 TSX 文件的动态导入需要在运行时转译

2. **@deno/loader 的 fallback**
   - 我们使用 `@deno/loader` 作为转译的 fallback
   - 但转译后的代码中的相对路径导入无法正确解析

3. **缺少完整的模块解析系统**
   - 需要实现类似 webpack 的模块打包系统
   - 或者在编译时预处理所有 TSX 文件

### 未来可能的解决方案

1. **预编译所有 TSX**
   - 在编译时将所有 TSX 转换为 JS
   - 打包时包含转换后的文件

2. **实现完整的模块系统**
   - 类似 Next.js 的打包方式
   - 或者使用 Vite 的模式

3. **使用不同的打包策略**
   - 探索其他打包工具
   - 或者改进现有的 @deno/loader 使用方式

## 相关文档

- [架构设计](./architecture.md) - 系统架构和设计原理
- [测试指南](./testing/overview.md) - 测试概述和运行方法
- [开发指南](./development.md) - 开发环境配置和最佳实践
