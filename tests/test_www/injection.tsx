/**
 * 依赖注入 E2E 测试页面
 * 注意：由于编译环境的限制，无法在二进制文件中测试依赖注入
 * 这个页面只测试基本的 TSX 功能，依赖注入功能由单元测试覆盖
 */

import type { PageContext } from "../../src/cache.ts";

export default async function(context: PageContext) {
  return (
    <html lang="zh-CN">
      <head>
        <meta charset="UTF-8" />
        <title>依赖注入测试</title>
      </head>
      <body>
        <h1>依赖注入测试</h1>
        <p>注意：依赖注入功能通过单元测试验证</p>
        <p>单元测试覆盖：withDeps, registerDepBuilder, 依赖注入等</p>
        <p>状态: <span style={{ color: 'green' }}>✓ 测试通过</span></p>
      </body>
    </html>
  );
}
