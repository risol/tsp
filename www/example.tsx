import Layout from "./components/Layout.tsx";
import Header from "./components/Header.tsx";
import Footer from "./components/Footer.tsx";

export default async function (context: PageContext) {
  
  const { method, url, query } = context;
  const sectionStyle = {
    margin: "20px 0",
    padding: "20px",
    background: "#f8f9fa",
    borderRadius: "8px"
  } as Record<string, string>;

  return (
    <Layout title="组件示例 - TTS-FPM" context={context}>
      <Header
        title="🎨 组件复用示例"
        subtitle="使用公共组件构建页面"
      />

      <div style={sectionStyle}>
        <h2>当前请求信息</h2>
        <p>请求方法: <strong>{method}</strong></p>
        <p>请求路径: <strong>{url.pathname}</strong></p>
      </div>

      <div style={sectionStyle}>
        <h2>组件的优势</h2>
        <ul style={{ marginLeft: "20px", lineHeight: "1.8" }}>
          <li>代码复用 - 避免重复编写相同的 HTML 结构</li>
          <li>统一样式 - 在一个地方修改，全局生效</li>
          <li>易于维护 - 组件化让代码结构更清晰</li>
          <li>TypeScript 支持 - 类型安全的属性传递</li>
        </ul>
      </div>

      <div style={sectionStyle}>
        <h2>测试导航</h2>
        <ul style={{ marginLeft: "20px" }}>
          <li><a href="/">返回首页</a></li>
          <li><a href="/form">表单示例</a></li>
          <li><a href="/api">API 信息</a></li>
        </ul>
      </div>

      <Footer />
    </Layout>
  );
}
