/**
 * 类型安全的依赖注入示例
 * 使用 Page 函数，更简洁的命名
 */

import { Page } from "../src/injection-typed.ts";

// ✅ 简洁！使用 Page 而不是 withDeps
export default Page(async function(context, { testFunc }) {
  // ✅ testFunc 有完整的类型提示！
  const result = testFunc();  // 类型: string

  return (
    <html lang="zh-CN">
      <head>
        <meta charset="UTF-8" />
        <title>Page 函数示例</title>
      </head>
      <body>
        <h1>类型安全的依赖注入</h1>
        <p>结果: {result}</p>
        <p>✅ 使用 Page 而不是 withDeps</p>
        <p>✅ 完整的 TypeScript 类型提示</p>
        <p>✅ 更简洁、更语义化</p>
      </body>
    </html>
  );
});
