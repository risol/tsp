import type { PageContext } from "../../src/cache.ts";
import { Layout } from "../components/Layout.tsx";

export default async function(context: PageContext) {
  const { method, body } = context;

  return (
    <Layout title="表单处理 - TSP" description="表单提交和数据处理演示">
      <h1 style={{ fontSize: "32px", marginBottom: "24px" }}>📝 表单处理</h1>
      <p style={{ color: "#64748b", marginBottom: "32px" }}>
        支持多种表单提交方式：GET、POST（表单数据）、POST（JSON 数据）
      </p>

      {/* Result Display */}
      {method === "POST" && body && (
        <div className="card" style={{
          background: "#d1fae5",
          border: "2px solid #10b981",
          marginBottom: "32px",
        }}>
          <h3 style={{ color: "#065f46", marginBottom: "12px" }}>✅ 提交成功！</h3>
          <div className="code-block" style={{ background: "#064e3b", marginTop: "16px" }}>
            {JSON.stringify(body, null, 2)}
          </div>
        </div>
      )}

      {/* POST Form */}
      <div className="section">
        <h2 className="section-title">POST 表单</h2>
        <div className="card">
          <form method="POST" style={{ maxWidth: "500px" }}>
            <div style={{ marginBottom: "16px" }}>
              <label style={{ display: "block", fontWeight: "600", marginBottom: "8px" }}>
                用户名：
              </label>
              <input
                type="text"
                name="username"
                required
                style={{
                  width: "100%",
                  padding: "10px",
                  border: "1px solid #ddd",
                  borderRadius: "4px",
                  fontSize: "14px",
                }}
              />
            </div>
            <button type="submit" className="btn btn-primary">提交</button>
          </form>
        </div>
      </div>

      <div style={{ marginTop: "40px" }}>
        <a href="/features" className="btn btn-secondary">← 返回功能列表</a>
      </div>
    </Layout>
  );
}
