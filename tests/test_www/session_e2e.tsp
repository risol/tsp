/**
 * Session E2E Test Page - TSP Version
 *
 * Used to test if session functionality works correctly in compiled binary
 */

export default Page(async function (ctx, { session, body, createZod }) {
  const z = await createZod();
  // Test 1: Verify session object and methods exist
  const methodCheck = {
    hasSession: session !== null && session !== undefined,
    hasInit: typeof session?.init === "function",
    hasDestroy: typeof session?.destroy === "function",
    hasSet: typeof session?.set === "function",
    hasGet: typeof session?.get === "function",
    hasDelete: typeof session?.delete === "function",
    hasClear: typeof session?.clear === "function",
    hasAll: typeof session?.all === "function",
  };

  // Test 2: Initialize session and store data
  let initResult = null;
  let initError = null;
  if (ctx.method === "POST") {
    try {
      const data = body(z.object({
        action: z.string().optional()
      }));
      if (data.action === "init") {
        await session.init();
        await session.set("userId", "e2e-test-user");
        await session.set("userName", "E2E Test User");
        await session.set("userEmail", "e2e@test.com");
        initResult = "success";
      }
    } catch (error) {
      initError = error instanceof Error ? error.message : String(error);
    }
  }

  // Test 3: Read session data
  let userId = null;
  let userName = null;
  let getDataError = null;
  try {
    userId = await session.get("userId");
    userName = await session.get("userName");
  } catch (error) {
    getDataError = error instanceof Error ? error.message : String(error);
  }

  // Test 4: Get all session data
  let allData = null;
  let getAllError = null;
  if (initResult === "success") {
    try {
      allData = await session.all();
    } catch (error) {
      getAllError = error instanceof Error ? error.message : String(error);
    }
  }

  // Test 5: Delete data
  let deleteResult = null;
  if (ctx.method === "POST" && initResult === "success") {
    try {
      await session.delete("userEmail");
      const checkDeleted = await session.get("userEmail");
      deleteResult = checkDeleted === null ? "success" : "failed";
    } catch (error) {
      deleteResult = error instanceof Error ? error.message : String(error);
    }
  }

  // Test 6: Clear all data
  let clearResult = null;
  let clearAction = null;
  let clearActionError = null;
  if (ctx.method === "POST") {
    try {
      const data = body(z.object({
        action: z.string().optional()
      }));
      clearAction = data.action;
      if (clearAction === "clear") {
        await session.clear();
        const checkCleared = await session.all();
        clearResult = Object.keys(checkCleared).length === 0 ? "success" : "failed";
      }
    } catch (error) {
      clearActionError = error instanceof Error ? error.message : String(error);
    }
  }

  // Test 7: Destroy session
  let destroyResult = null;
  let destroyAction = null;
  let destroyActionError = null;
  if (ctx.method === "POST") {
    try {
      const data = body(z.object({
        action: z.string().optional()
      }));
      destroyAction = data.action;
      if (destroyAction === "destroy") {
        await session.destroy();
        destroyResult = "success";
      }
    } catch (error) {
      destroyActionError = error instanceof Error ? error.message : String(error);
    }
  }

  // Return JSON formatted test results
  return (
    <div>
      {JSON.stringify(
        {
          timestamp: new Date().toISOString(),
          methodChecks: methodCheck,
          init: {
            called: ctx.method === "POST" && initResult === "success",
            result: initResult,
            error: initError,
          },
          getData: {
            userId: userId,
            userName: userName,
            error: getDataError,
          },
          getAll: {
            result: allData,
            error: getAllError,
          },
          delete: {
            result: deleteResult,
          },
          clear: {
            result: clearResult,
            error: clearActionError,
          },
          destroy: {
            result: destroyResult,
            error: destroyActionError,
          },
        },
        null,
        2,
      )}
    </div>
  );
});
