/**
 * 热重载测试页面
 * 模拟真实场景的二级依赖链，测试传递依赖的热重载
 *
 * 依赖链：
 * hot_reload_page.tsx → HotReloadWrapper.tsx → HotReloadComponent.tsx
 *
 * 这样可以测试当修改二级依赖（HotReloadComponent）时，
 * 页面是否能正确热重载
 */

import { HotReloadWrapper } from "./components/HotReloadWrapper.tsx";

export default async function () {
  return (
    <html lang="zh-CN">
      <head>
        <meta charset="UTF-8" />
        <title>热重载测试（传递依赖）</title>
      </head>
      <body>
        <h1>热重载测试页面 - 二级依赖链测试</h1>
        <p>测试场景：Page → Wrapper → Component（修改 Component 触发重载）</p>
        <div id="component-container">
          <HotReloadWrapper />
        </div>
      </body>
    </html>
  );
}
