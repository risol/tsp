export function Sidebar() {
  return (
    <aside class="sidebar">
      <div style={{ marginBottom: "24px" }}>
        <a href="/" style={{ fontSize: "18px", fontWeight: "600", color: "#3178c6", textDecoration: "none" }}>TSP</a>
      </div>

      <div style={{ marginBottom: "20px" }}>
        <div style={{ fontSize: "12px", fontWeight: "600", color: "#586069", marginBottom: "8px", textTransform: "uppercase" }}>入门</div>
        <ul style={{ listStyle: "none", padding: "0", margin: "0" }}>
          <li style={{ marginBottom: "4px" }}><a href="/" style={{ color: "#24292e", textDecoration: "none", fontSize: "14px", display: "block", padding: "4px 8px", borderRadius: "4px" }}>介绍</a></li>
          <li style={{ marginBottom: "4px" }}><a href="/getting-started" style={{ color: "#24292e", textDecoration: "none", fontSize: "14px", display: "block", padding: "4px 8px", borderRadius: "4px", background: "#e8f5e9" }}>快速开始</a></li>
        </ul>
      </div>

      <div style={{ marginBottom: "20px" }}>
        <div style={{ fontSize: "12px", fontWeight: "600", color: "#586069", marginBottom: "8px", textTransform: "uppercase" }}>指南</div>
        <ul style={{ listStyle: "none", padding: "0", margin: "0" }}>
          <li style={{ marginBottom: "4px" }}><a href="/features" style={{ color: "#24292e", textDecoration: "none", fontSize: "14px", display: "block", padding: "4px 8px", borderRadius: "4px" }}>功能特性</a></li>
          <li style={{ marginBottom: "4px" }}><a href="/examples" style={{ color: "#24292e", textDecoration: "none", fontSize: "14px", display: "block", padding: "4px 8px", borderRadius: "4px" }}>代码示例</a></li>
        </ul>
      </div>

      <div style={{ marginBottom: "20px" }}>
        <div style={{ fontSize: "12px", fontWeight: "600", color: "#586069", marginBottom: "8px", textTransform: "uppercase" }}>更多</div>
        <ul style={{ listStyle: "none", padding: "0", margin: "0" }}>
          <li style={{ marginBottom: "4px" }}><a href="https://github.com/your-repo/tsp" style={{ color: "#24292e", textDecoration: "none", fontSize: "14px", display: "block", padding: "4px 8px", borderRadius: "4px" }}>GitHub</a></li>
        </ul>
      </div>

      <div style={{ marginTop: "auto", paddingTop: "20px", borderTop: "1px solid #e1e4e8" }}>
        <div style={{ fontSize: "12px", color: "#586069" }}>版本: 4.0.0</div>
      </div>
    </aside>
  );
}
