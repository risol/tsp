import type { PageContext } from "../src/cache.ts";

export default async function (_context: PageContext) {
  const redirectObj = { redirect: "/" };

  // 调试输出
  console.log("[redirect_simple] 返回对象:", JSON.stringify(redirectObj));

  return redirectObj;
}
