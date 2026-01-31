/**
 * JSX Import 测试页面
 * 测试导入：
 * - JSX 组件（非 src 目录）
 * - TS 工具函数（非 src 目录）
 * - 组件嵌套
 */

import { Header } from "./components/Header.tsx";
import { Card } from "./components/Card.tsx";
import { formatDate, getGreeting, sum, truncate } from "./utils/helpers.ts";

export default async function (context: PageContext) {
  const greeting = getGreeting("开发者");
  const today = formatDate(new Date());
  const numbers = [1, 2, 3, 4, 5];
  const total = sum(numbers);
  const longText =
    "这是一段很长的文本，需要被截断处理，以测试我们的 truncate 工具函数是否正常工作。";
  const truncated = truncate(longText, 20);

  return (
    <html lang="zh-CN">
      <head>
        <meta charset="UTF-8" />
        <title>JSX Import 测试</title>
      </head>
      <body
        style={{
          fontFamily: "system-ui, -apple-system, sans-serif",
          margin: 0,
          padding: "40px",
          backgroundColor: "#f1f5f9",
        }}
      >
        <div style={{ maxWidth: "800px", margin: "0 auto" }}>
          {/* 导入的 Header 组件 */}
          <Header
            title="JSX Import 功能测试"
            subtitle="测试导入组件和工具函数"
          />

          <div style={{ marginBottom: "24px" }}>
            <h2 style={{ color: "#1e293b" }}>测试结果</h2>
          </div>

          {/* 测试工具函数 */}
          <Card
            title="工具函数测试"
            content={`问候语: ${greeting} | 日期: ${today}`}
            footer={`1+2+3+4+5 = ${total}`}
          />

          {/* 测试组件嵌套 */}
          <Card
            title="组件嵌套测试"
            content="成功导入并使用了 Header 和 Card 组件"
            footer="✓ 组件导入正常"
          />

          <Card
            title="文本截断测试"
            content={`原文: ${longText}`}
            footer={`截断后: ${truncated}`}
          />

          {/* 测试状态列表 */}
          <div
            style={{
              backgroundColor: "white",
              borderRadius: "8px",
              padding: "20px",
              marginTop: "16px",
            }}
          >
            <h3 style={{ margin: "0 0 16px 0", color: "#1e293b" }}>功能验证</h3>
            <ul style={{ margin: 0, paddingLeft: "20px", color: "#64748b" }}>
              <li>✓ 导入 JSX 组件 (Header.tsx)</li>
              <li>✓ 导入 JSX 组件 (Card.tsx)</li>
              <li>✓ 导入 TS 工具函数 (helpers.ts)</li>
              <li>✓ 组件嵌套渲染</li>
              <li>✓ 动态数据传递</li>
              <li>✓ 非src目录导入</li>
            </ul>
          </div>

          <div
            style={{
              marginTop: "24px",
              padding: "16px",
              backgroundColor: "#dcfce7",
              borderRadius: "8px",
              color: "#166534",
              textAlign: "center",
            }}
          >
            <strong>✓ 所有 JSX Import 测试通过</strong>
          </div>
        </div>
      </body>
    </html>
  );
}
