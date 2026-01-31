export default async function (_context: PageContext) {
  // 故意抛出错误，用于测试错误处理
  throw new Error("这是一个测试错误，用于验证错误处理机制");
}
