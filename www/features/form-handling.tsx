import { Layout } from "../components/Layout.tsx";

export default async function (context: PageContext) {
  const { method, body } = context;

  return (
    <Layout title="表单处理 - TSP" description="表单提交和数据处理演示">
      <h1 className="display-6 mb-4">📝 表单处理</h1>
      <p className="text-muted mb-5">
        支持多种表单提交方式：GET、POST（表单数据）、POST（JSON 数据）
      </p>

      {/* Result Display */}
      {method === "POST" && body && (
        <div className="card bg-success bg-opacity-10 border-success border-2 mb-5">
          <h3 className="text-success mb-3">
            ✅ 提交成功！
          </h3>
          <div className="code-block bg-success bg-opacity-25 mt-3">
            {JSON.stringify(body, null, 2)}
          </div>
        </div>
      )}

      {/* POST Form */}
      <div className="section">
        <h2 className="section-title">POST 表单</h2>
        <div className="card">
          <form method="POST" style={{ maxWidth: "500px" }}>
            <div className="mb-3">
              <label className="form-label fw-semibold mb-2">
                用户名：
              </label>
              <input
                type="text"
                name="username"
                required
                className="form-control"
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
