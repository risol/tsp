/**
 * Session E2E Test Page
 *
 * 用于测试session功能在编译二进制中是否正常工作
 */

export default Page(async function (ctx, { session }) {
  // 测试1: 验证session对象和方法存在
  const methodCheck = {
    hasSession: session !== null && session !== undefined,
    hasGetUser: typeof session?.getUser === "function",
    hasLogin: typeof session?.login === "function",
    hasLogout: typeof session?.logout === "function",
    hasSet: typeof session?.set === "function",
    hasGet: typeof session?.get === "function",
    hasDelete: typeof session?.delete === "function",
  };

  // 测试2: 尝试调用getUser
  let user = null;
  let getUserError = null;
  try {
    user = await session.getUser();
  } catch (error) {
    getUserError = error instanceof Error ? error.message : String(error);
  }

  // 测试3: 尝试调用login
  let loginResult = null;
  let loginError = null;
  if (ctx.method === "POST") {
    try {
      const { action } = ctx.body as { action?: string };
      if (action === "login") {
        await session.login("e2e-test-user", {
          name: "E2E Test User",
          email: "e2e@test.com",
        });
        loginResult = "success";
      }
    } catch (error) {
      loginError = error instanceof Error ? error.message : String(error);
    }
  }

  // 测试4: 如果已登录，尝试调用set/get
  let setDataResult = null;
  if (ctx.method === "POST" && loginResult === "success") {
    try {
      await session.set("testKey", "testValue");
      const getDataResult = await session.get<string>("testKey");
      setDataResult = getDataResult === "testValue" ? "success" : "failed";
    } catch (error) {
      setDataResult = error instanceof Error ? error.message : String(error);
    }
  }

  // 返回JSON格式的测试结果
  return (
    <div>
      {JSON.stringify(
        {
          timestamp: new Date().toISOString(),
          methodChecks: methodCheck,
          getUser: {
            called: true,
            user: user,
            error: getUserError,
          },
          login: {
            called: ctx.method === "POST",
            result: loginResult,
            error: loginError,
          },
          setData: {
            result: setDataResult,
          },
        },
        null,
        2,
      )}
    </div>
  );
});
