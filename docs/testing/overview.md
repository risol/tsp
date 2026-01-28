# TSP 测试文档

## 测试结构

```
tests/
├── unit/                     # 单元测试（不启动服务器）
│   ├── router_test.ts       # 路由解析测试
│   ├── context_test.ts      # 上下文构建测试
│   └── security_test.ts     # 安全检查测试
│
├── e2e/                      # E2E测试（启动二进制服务器）
│   ├── test_utils.ts        # 共享工具函数
│   ├── 00_setup.test.ts     # 编译和启动
│   ├── 01_basic_http.test.ts # 基本HTTP
│   ├── 02_api.test.ts       # API测试
│   ├── 03_error.test.ts     # 错误处理
│   ├── 04_security.test.ts  # 安全性
│   └── 05_config.test.ts    # 配置文件
│
├── run_unit_tests.ts        # 单元测试运行器
├── run_e2e_tests.ts         # E2E测试运行器
└── run_all_tests.ts         # 全部测试运行器
```

## 运行测试

```bash
# 所有测试
deno task test

# 只运行单元测试
deno run --allow-net --allow-read tests/run_unit_tests.ts

# 只运行E2E测试
deno run --allow-all tests/run_e2e_tests.ts

# 单个测试文件
deno test --allow-net tests/unit/router_test.ts
```

## 相关文档

- [测试文档首页](./README.md) - 测试文档导航
- [测试页面设置](./test-pages.md) - 测试页面配置
- [任务权限说明](./task-permissions.md) - Deno 任务权限
- [开发指南](../development.md) - 如何编写测试

---

[← 返回测试文档](./README.md) | [← 返回文档中心](../README.md)
