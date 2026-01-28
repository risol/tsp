import type { PageContext } from "../../src/cache.ts";
import type { RedirectResult } from "../../src/cache.ts";
import { Layout } from "../components/Layout.tsx";

export default async function(context: PageContext) {
  const { url } = context;

  // 处理重定向演示
  const searchParams = new URL(url.href).searchParams;
  const action = searchParams.get('action');
  const status = searchParams.get('status');

  if (action === 'redirect') {
    const statusCode = status ? parseInt(status) : 302;
    const validStatuses = [301, 302, 303, 307, 308];

    return {
      redirect: '/features',
      status: validStatuses.includes(statusCode) ? statusCode as 301 | 302 | 303 | 307 | 308 : 302
    } as RedirectResult;
  }

  return (
    <Layout title="重定向 - TSP" description="HTTP 重定向功能">
      <h1 style={{ fontSize: "32px", marginBottom: "24px" }}>➡️ HTTP 重定向</h1>
      <p style={{ color: "#64748b", marginBottom: "32px" }}>
        返回包含 redirect 属性的对象即可触发 HTTP 重定向
      </p>

      {/* Demo */}
      <div className="section">
        <h2 className="section-title">🎮 交互式演示</h2>
        <div className="card">
          <div style={{ display: "flex", gap: "12px", flexWrap: "wrap" }}>
            <a href="/features/redirect?action=redirect&status=301" className="btn btn-secondary">301 永久</a>
            <a href="/features/redirect?action=redirect&status=302" className="btn btn-secondary">302 临时</a>
            <a href="/features/redirect?action=redirect&status=303" className="btn btn-secondary">303 See Other</a>
            <a href="/features/redirect?action=redirect&status=307" className="btn btn-secondary">307 临时(POST)</a>
            <a href="/features/redirect?action=redirect&status=308" className="btn btn-secondary">308 永久(POST)</a>
          </div>
        </div>
      </div>

      {/* Usage */}
      <div className="section">
        <h2 className="section-title">📖 使用方法</h2>
        <div className="card">
          <div className="code-block">
            {`export default async function(context) {
  return {
    redirect: '/new-location',
    status: 302
  };
}`}
          </div>
        </div>
      </div>

      <div style={{ marginTop: "40px" }}>
        <a href="/features" className="btn btn-secondary">← 返回功能列表</a>
      </div>
    </Layout>
  );
}
