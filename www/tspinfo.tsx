/**
 * TSP Info 页面
 * 类似 PHP 的 phpinfo()，显示 TSP 服务器运行时信息
 */

import { Layout } from "./components/Layout.tsx";

export default Page(async function (context, { tspinfo }) {
  // 返回 HTML 格式的信息页面
  const html = await tspinfo.renderHTML();

  // 直接返回 HTML（不使用 Layout，因为 tspinfo 已经返回完整的 HTML）
  return new Response(html, {
    status: 200,
    headers: {
      "Content-Type": "text/html; charset=utf-8",
    },
  });
});
