/**
 * 类型安全的依赖注入示例
 * 注意：这个文件无需 import，withDeps 是全局的！
 */

// ✅ 无需 import！withDeps 已经是全局函数
export default withDeps(async function(context, { testFunc }) {
  // ✅ testFunc 有完整的类型提示！
  const result = testFunc();  // 类型: string

  return (
    <html lang="zh-CN">
      <head>
        <meta charset="UTF-8" />
        <title>类型安全的依赖注入</title>
      </head>
      <body>
        <h1>类型安全的依赖注入示例</h1>
        <p>结果: {result}</p>
        <p>✅ 无需 import withDeps</p>
        <p>✅ 完整的 TypeScript 类型提示</p>
      </body>
    </html>
  );
});
