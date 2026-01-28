import type { PageContext } from "../../src/cache.ts";
import { Layout } from "../components/Layout.tsx";

export default async function(context: PageContext) {
  const { method, url, headers, query, cookies, body } = context;

  return (
    <Layout title="请求信息 - TSP-FPM" description="HTTP 请求信息完整展示">
      <h1 style={{ fontSize: "32px", marginBottom: "24px" }}>📡 请求信息</h1>
      <p style={{ color: "#64748b", marginBottom: "32px" }}>
        当前 HTTP 请求的完整信息，包括方法、URL、Headers、查询参数等
      </p>

      {/* Request Info */}
      <div className="info-grid">
        <div className="info-item">
          <div className="info-label">HTTP 方法</div>
          <div className="info-value">
            <span className="badge badge-info">{method}</span>
          </div>
        </div>
        <div className="info-item">
          <div className="info-label">请求路径</div>
          <div className="info-value">{url.pathname}</div>
        </div>
        <div className="info-item">
          <div className="info-label">查询参数</div>
          <div className="info-value">{Object.keys(query).length > 0 ? JSON.stringify(query) : "无"}</div>
        </div>
        <div className="info-item">
          <div className="info-label">Cookies</div>
          <div className="info-value">{Object.keys(cookies).length > 0 ? JSON.stringify(cookies) : "无"}</div>
        </div>
      </div>

      <div style={{ marginTop: "40px" }}>
        <a href="/features" className="btn btn-secondary">← 返回功能列表</a>
      </div>
    </Layout>
  );
}
