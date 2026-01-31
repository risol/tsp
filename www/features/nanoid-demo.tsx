import { Layout } from "../components/Layout.tsx";

export default Page(async function (ctx, { nanoid }) {
  // 生成不同长度的 nanoid
  const defaultId = nanoid(); // 默认 21 字符
  const shortId = nanoid(10); // 10 字符
  const longId = nanoid(30); // 30 字符

  // 生成多个示例 ID
  const examples = Array.from({ length: 5 }, () => nanoid());

  return (
    <Layout title="nanoid 演示 - TSP" description="使用 nanoid 生成唯一 ID">
      <h1 style={{ fontSize: "32px", marginBottom: "24px" }}>
        🔐 nanoid 唯一 ID 生成
      </h1>
      <p style={{ color: "#64748b", marginBottom: "32px" }}>
        nanoid 是一个轻量级、URL-safe 的唯一 ID 生成器，可直接在 TSX 中使用
      </p>

      {/* Generated IDs */}
      <div className="section">
        <h2 className="section-title">生成的 ID 示例</h2>
        <div className="card">
          <div style={{ marginBottom: "16px" }}>
            <strong>默认长度（21 字符）:</strong>
            <div
              style={{
                background: "#064e3b",
                color: "#34d399",
                padding: "8px 12px",
                borderRadius: "4px",
                marginTop: "8px",
                fontFamily: "monospace",
                fontSize: "14px",
              }}
            >
              {defaultId}
            </div>
          </div>

          <div style={{ marginBottom: "16px" }}>
            <strong>短 ID（10 字符）:</strong>
            <div
              style={{
                background: "#064e3b",
                color: "#60a5fa",
                padding: "8px 12px",
                borderRadius: "4px",
                marginTop: "8px",
                fontFamily: "monospace",
                fontSize: "14px",
              }}
            >
              {shortId}
            </div>
          </div>

          <div style={{ marginBottom: "16px" }}>
            <strong>长 ID（30 字符）:</strong>
            <div
              style={{
                background: "#064e3b",
                color: "#f472b6",
                padding: "8px 12px",
                borderRadius: "4px",
                marginTop: "8px",
                fontFamily: "monospace",
                fontSize: "14px",
              }}
            >
              {longId}
            </div>
          </div>
        </div>
      </div>

      {/* Multiple Examples */}
      <div className="section">
        <h2 className="section-title">多个 ID 示例（每次刷新页面都会变化）</h2>
        <div className="card">
          <div
            style={{
              background: "#064e3b",
              padding: "16px",
              borderRadius: "4px",
              fontFamily: "monospace",
              fontSize: "13px",
            }}
          >
            {examples.map((id, i) => (
              <div
                key={i}
                style={{
                  color: "#34d399",
                  marginBottom: i < examples.length - 1 ? "8px" : "0",
                }}
              >
                {i + 1}. {id}
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Usage Example */}
      <div className="section">
        <h2 className="section-title">使用示例</h2>
        <div className="code-block">
          {`// 在 TSX 中直接使用 nanoid，无需导入
export default Page(async function(ctx, { nanoid }) {
  // 生成默认长度（21 字符）的唯一 ID
  const id = nanoid();

  // 生成指定长度的 ID
  const shortId = nanoid(10);

  // 用于文件名
  const filename = \`photo_\${nanoid()}.jpg\`;

  // 用于订单号
  const orderId = \`ORDER_\${nanoid()}\`;

  return <div>ID: {id}</div>;
});`}
        </div>
      </div>

      {/* Features */}
      <div className="section">
        <h2 className="section-title">特性</h2>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(250px, 1fr))", gap: "16px" }}>
          <div className="card">
            <h3 style={{ color: "#10b981", marginBottom: "8px" }}>✅ URL-safe</h3>
            <p style={{ color: "#64748b", fontSize: "14px" }}>
              使用 A-Za-z0-9_- 字符，可直接用于 URL 和文件名
            </p>
          </div>

          <div className="card">
            <h3 style={{ color: "#10b981", marginBottom: "8px" }}>⚡ 快速</h3>
            <p style={{ color: "#64748b", fontSize: "14px" }}>
              比 UUID 快 40%，性能优异
            </p>
          </div>

          <div className="card">
            <h3 style={{ color: "#10b981", marginBottom: "8px" }}>📏 紧凑</h3>
            <p style={{ color: "#64748b", fontSize: "14px" }}>
              默认 21 字符，比 UUID（36 字符）更短
            </p>
          </div>

          <div className="card">
            <h3 style={{ color: "#10b981", marginBottom: "8px" }}>🔒 安全</h3>
            <p style={{ color: "#64748b", fontSize: "14px" }}>
              使用加密强度随机生成器，碰撞概率极低
            </p>
          </div>
        </div>
      </div>

      {/* Comparison */}
      <div className="section">
        <h2 className="section-title">vs UUID v4</h2>
        <div className="code-block">
          {`// UUID v4
550e8400-e29b-41d4-a716-446655440000  // 36 字符，包含连字符

// nanoid
V1StGXR8_Z5jdHi6B-myT              // 21 字符，URL-safe

// 优势对比：
// • 更短：21 vs 36 字符
// • 更快：性能提升 40%
// • 更可读：无连字符等特殊字符
// • URL-safe：可直接用于 URL 而不需编码`}
        </div>
      </div>

      <div style={{ marginTop: "40px" }}>
        <a href="/features" className="btn btn-secondary">← 返回功能列表</a>
      </div>
    </Layout>
  );
});
