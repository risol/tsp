/**
 * TSP Info 使用示例
 *
 * 访问 http://localhost:9000/tspinfo 查看服务器信息
 *
 * 或者在任何 TSX 页面中使用：
 */

import { Layout } from "./components/Layout.tsx";

export default Page(async function (context, { tspinfo, response }) {
  // 1. 获取 JSON 格式的服务器信息
  const info = tspinfo.getInfo();

  // 2. 返回 JSON 格式
  // return response.json(info);

  // 3. 返回 HTML 格式的完整信息页面（类似 phpinfo）
  const html = tspinfo.renderHTML();
  return new Response(html, {
    status: 200,
    headers: {
      "Content-Type": "text/html; charset=utf-8",
    },
  });

  /* info 对象包含：
   * - server: 服务器信息（版本、架构等）
   * - config: 配置信息（根目录、端口、模式等）
   * - runtime: 运行时信息（进程ID、运行时间等）
   * - cache: 缓存统计（缓存文件数、缓存目录等）
   * - dependencies: 已注册的依赖列表
   */
});
