/**
 * Page 包装器 E2E 测试页面
 *
 * 测试 Page 包装器在编译二进制中是否正常工作
 */

export default Page(async function(ctx, { session, cookies, testFunc }) {
  // 测试所有注入的依赖
  const tests = {
    testFunc: {
      exists: typeof testFunc === "function",
      result: testFunc ? testFunc() : null,
    },
    session: {
      exists: session !== null && session !== undefined,
      hasGetUser: typeof session.getUser === "function",
      hasLogin: typeof session.login === "function",
      hasLogout: typeof session.logout === "function",
    },
    cookies: {
      exists: cookies !== null && cookies !== undefined,
      hasSet: typeof cookies.set === "function",
      hasGet: typeof cookies.get === "function",
      hasDelete: typeof cookies.delete === "function",
    },
  };

  // 返回 JSON 格式的测试结果
  return (
    <div>
      <h1>Page Wrapper E2E Test</h1>
      <pre>{JSON.stringify(tests, null, 2)}</pre>
    </div>
  );
});
